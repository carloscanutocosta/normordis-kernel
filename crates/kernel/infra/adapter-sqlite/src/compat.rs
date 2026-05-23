use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, Error as RusqliteError, ErrorCode, Transaction, TransactionBehavior};
use support_errors::MiniError;
use thiserror::Error;

use crate::error::{sqlite_error, BUSY_TIMEOUT};
use crate::relational::{
    apply_relational_pragmas, open_relational_connection, relational_sqlite_uri,
    run_relational_migrations, SqliteRelationalConfig, SqliteRelationalOpenMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteOpenMode {
    ReadOnly,
    ReadWriteCreate,
}

impl From<SqliteOpenMode> for SqliteRelationalOpenMode {
    fn from(value: SqliteOpenMode) -> Self {
        match value {
            SqliteOpenMode::ReadOnly => Self::ReadOnly,
            SqliteOpenMode::ReadWriteCreate => Self::ReadWriteCreate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteOptions {
    pub path: PathBuf,
    pub mode: SqliteOpenMode,
    pub enable_foreign_keys: bool,
    pub busy_timeout_ms: u64,
}

impl SqliteOptions {
    pub fn read_only(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            mode: SqliteOpenMode::ReadOnly,
            enable_foreign_keys: true,
            busy_timeout_ms: 5_000,
        }
    }

    pub fn read_write_create(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            mode: SqliteOpenMode::ReadWriteCreate,
            enable_foreign_keys: true,
            busy_timeout_ms: 5_000,
        }
    }
}

impl From<&SqliteOptions> for SqliteRelationalConfig {
    fn from(value: &SqliteOptions) -> Self {
        let mut config = match value.mode {
            SqliteOpenMode::ReadOnly => SqliteRelationalConfig::read_only(&value.path),
            SqliteOpenMode::ReadWriteCreate => {
                SqliteRelationalConfig::read_write_create(&value.path)
            }
        };
        config.enable_foreign_keys = value.enable_foreign_keys;
        config.busy_timeout_ms = value.busy_timeout_ms;
        config
    }
}

pub fn open_connection(options: &SqliteOptions) -> Result<Connection, MiniError> {
    open_relational_connection(&SqliteRelationalConfig::from(options))
}

pub fn apply_pragmas(conn: &Connection, options: &SqliteOptions) -> Result<(), MiniError> {
    apply_relational_pragmas(conn, &SqliteRelationalConfig::from(options))
}

pub fn run_migrations(conn: &Connection, migrations: &[&str]) -> Result<(), MiniError> {
    run_relational_migrations(conn, migrations)
}

pub fn with_transaction<T, F>(conn: &mut Connection, work: F) -> Result<T, MiniError>
where
    F: FnOnce(&Transaction<'_>) -> Result<T, MiniError>,
{
    let tx = conn.transaction().map_err(|_| {
        crate::error::sqlite_error(
            crate::error::TRANSACTION_FAILED,
            "failed to start sqlite transaction",
        )
    })?;
    let result = work(&tx)?;
    tx.commit().map_err(|_| {
        crate::error::sqlite_error(
            crate::error::TRANSACTION_FAILED,
            "failed to commit sqlite transaction",
        )
    })?;
    Ok(result)
}

pub fn sqlite_uri(path: impl AsRef<Path>, mode: SqliteOpenMode) -> String {
    relational_sqlite_uri(path, mode.into())
}

#[derive(Debug, Error)]
pub enum WriteBatchError {
    #[error(transparent)]
    SqliteAdapter(#[from] MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] RusqliteError),
    #[error("erro de operação em batch: {0}")]
    Operation(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 50,
            max_delay_ms: 2_000,
        }
    }
}

type BatchOp =
    Box<dyn Fn(&Transaction<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>>;

pub struct WriteBatch {
    config: SqliteRelationalConfig,
    retry: RetryPolicy,
    ops: Vec<BatchOp>,
}

impl WriteBatch {
    pub fn new(options: &SqliteOptions) -> Self {
        Self::new_with_retry(options, RetryPolicy::default())
    }

    pub fn new_with_retry(options: &SqliteOptions, retry: RetryPolicy) -> Self {
        let mut config = SqliteRelationalConfig::from(options);
        config.mode = SqliteRelationalOpenMode::ReadWriteCreate;
        Self {
            config,
            retry,
            ops: Vec::new(),
        }
    }

    pub fn push<E, F>(&mut self, op: F) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
        F: Fn(&Transaction<'_>) -> Result<(), E> + 'static,
    {
        self.ops.push(Box::new(move |tx| {
            op(tx).map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)
        }));
        self
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn commit(self) -> Result<usize, WriteBatchError> {
        let count = self.ops.len();
        if count == 0 {
            return Ok(0);
        }

        let mut last_busy = None;
        for attempt in 0..self.retry.max_attempts.max(1) {
            if attempt > 0 {
                std::thread::sleep(backoff_delay(attempt, &self.retry));
            }

            match try_commit_once(&self.config, &self.ops) {
                Ok(count) => return Ok(count),
                Err(err) if is_busy(&err) => last_busy = Some(err),
                Err(err) => return Err(err),
            }
        }

        Err(last_busy.unwrap_or_else(|| {
            WriteBatchError::SqliteAdapter(sqlite_error(
                BUSY_TIMEOUT,
                "sqlite write batch exhausted busy retries",
            ))
        }))
    }
}

impl std::fmt::Debug for WriteBatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteBatch")
            .field("database_path", &self.config.database_path)
            .field("ops", &self.ops.len())
            .field("retry", &self.retry)
            .finish()
    }
}

fn try_commit_once(
    config: &SqliteRelationalConfig,
    ops: &[BatchOp],
) -> Result<usize, WriteBatchError> {
    let mut conn = open_relational_connection(config)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    for op in ops {
        op(&tx).map_err(WriteBatchError::Operation)?;
    }
    tx.commit()?;
    Ok(ops.len())
}

fn is_busy(err: &WriteBatchError) -> bool {
    matches!(
        err,
        WriteBatchError::Sqlite(RusqliteError::SqliteFailure(error, _))
            if matches!(error.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

fn backoff_delay(attempt: u32, policy: &RetryPolicy) -> Duration {
    let exp_ms = policy
        .base_delay_ms
        .saturating_mul(1u64 << (attempt - 1).min(10))
        .min(policy.max_delay_ms);
    Duration::from_millis(exp_ms.saturating_add(jitter(policy.base_delay_ms / 2 + 1)))
}

fn jitter(max_ms: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);
    u64::from(nanos) % max_ms.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn compatibility_open_and_migrations_match_relational_bridge() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("compat.db");
        let options = SqliteOptions::read_write_create(&db_path);
        let conn = open_connection(&options).unwrap();

        run_migrations(
            &conn,
            &["CREATE TABLE demo (id TEXT PRIMARY KEY, label TEXT NOT NULL);"],
        )
        .unwrap();

        conn.execute("INSERT INTO demo (id, label) VALUES (?1, ?2)", ["1", "ok"])
            .unwrap();
        let label: String = conn
            .query_row("SELECT label FROM demo WHERE id = '1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(label, "ok");
    }

    #[test]
    fn compatibility_write_batch_accepts_operation_errors() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("batch.db");
        let options = SqliteOptions::read_write_create(&db_path);
        let conn = open_connection(&options).unwrap();
        run_migrations(
            &conn,
            &["CREATE TABLE demo (id TEXT PRIMARY KEY, label TEXT NOT NULL);"],
        )
        .unwrap();
        drop(conn);

        let mut batch = WriteBatch::new(&options);
        batch.push(|tx| {
            tx.execute("INSERT INTO demo (id, label) VALUES (?1, ?2)", ["1", "one"])?;
            Ok::<(), RusqliteError>(())
        });
        batch.push(|tx| {
            tx.execute("INSERT INTO demo (id, label) VALUES (?1, ?2)", ["2", "two"])?;
            Ok::<(), RusqliteError>(())
        });

        assert_eq!(batch.commit().unwrap(), 2);
    }
}

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{
    Connection, Error as RusqliteError, ErrorCode as RusqliteErrorCode, OpenFlags, Transaction,
    TransactionBehavior,
};
use support_errors::MiniError;

use crate::config::{SqliteConfig, SqliteJournalMode, SqliteSynchronous};
use crate::connection::validate_database_path;
use crate::error::{
    sqlite_error, BUSY_TIMEOUT, CONFIGURE_FAILED, CREATE_PARENT_DIR_FAILED, EXECUTE_FAILED,
    INVALID_PATH, OPEN_FAILED, TRANSACTION_FAILED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteRelationalOpenMode {
    ReadOnly,
    ReadWriteCreate,
}

#[derive(Debug, Clone)]
pub struct SqliteRelationalConfig {
    pub database_path: PathBuf,
    pub mode: SqliteRelationalOpenMode,
    pub create_parent_dir: bool,
    pub busy_timeout_ms: u64,
    pub enable_foreign_keys: bool,
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronous,
    pub wal_autocheckpoint_pages: u32,
}

impl SqliteRelationalConfig {
    pub fn read_only(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            mode: SqliteRelationalOpenMode::ReadOnly,
            create_parent_dir: false,
            busy_timeout_ms: 5_000,
            enable_foreign_keys: true,
            journal_mode: SqliteJournalMode::Wal,
            synchronous: SqliteSynchronous::Normal,
            wal_autocheckpoint_pages: 1_000,
        }
    }

    pub fn read_write_create(database_path: impl Into<PathBuf>) -> Self {
        Self::from_sqlite_config(
            SqliteConfig::new(database_path),
            SqliteRelationalOpenMode::ReadWriteCreate,
        )
    }

    pub fn from_sqlite_config(config: SqliteConfig, mode: SqliteRelationalOpenMode) -> Self {
        Self {
            database_path: config.database_path,
            mode,
            create_parent_dir: config.create_parent_dir,
            busy_timeout_ms: config.busy_timeout_ms,
            enable_foreign_keys: config.enable_foreign_keys,
            journal_mode: config.journal_mode,
            synchronous: config.synchronous,
            wal_autocheckpoint_pages: config.wal_autocheckpoint_pages,
        }
    }
}

pub fn open_relational_connection(
    config: &SqliteRelationalConfig,
) -> Result<Connection, MiniError> {
    validate_relational_database_path(config)?;

    if config.mode == SqliteRelationalOpenMode::ReadWriteCreate && config.create_parent_dir {
        if let Some(parent) = config.database_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|_| {
                    sqlite_error(
                        CREATE_PARENT_DIR_FAILED,
                        "failed to create sqlite database parent directory",
                    )
                })?;
            }
        }
    }

    let flags = match config.mode {
        SqliteRelationalOpenMode::ReadOnly => {
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI
        }
        SqliteRelationalOpenMode::ReadWriteCreate => {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
        }
    };

    let conn = Connection::open_with_flags(&config.database_path, flags)
        .map_err(|_| sqlite_error(OPEN_FAILED, "failed to open sqlite database"))?;
    apply_relational_pragmas(&conn, config)?;
    Ok(conn)
}

pub fn run_relational_migrations(conn: &Connection, migrations: &[&str]) -> Result<(), MiniError> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS _migrations (id INTEGER PRIMARY KEY);")
        .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to initialize sqlite migrations"))?;
    for migration in migrations {
        let id = fnv1a(migration);
        let applied: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to read sqlite migrations"))?;
        if !applied {
            conn.execute_batch(migration)
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to run sqlite migration"))?;
            conn.execute("INSERT OR IGNORE INTO _migrations (id) VALUES (?1)", [id])
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to record sqlite migration"))?;
        }
    }
    Ok(())
}

pub fn with_relational_transaction<T, F>(conn: &mut Connection, work: F) -> Result<T, MiniError>
where
    F: FnOnce(&Transaction<'_>) -> Result<T, MiniError>,
{
    let tx = conn
        .transaction()
        .map_err(|_| sqlite_error(TRANSACTION_FAILED, "failed to start sqlite transaction"))?;
    let result = work(&tx)?;
    tx.commit()
        .map_err(|_| sqlite_error(TRANSACTION_FAILED, "failed to commit sqlite transaction"))?;
    Ok(result)
}

pub fn relational_sqlite_uri(path: impl AsRef<Path>, mode: SqliteRelationalOpenMode) -> String {
    let mode = match mode {
        SqliteRelationalOpenMode::ReadOnly => "ro",
        SqliteRelationalOpenMode::ReadWriteCreate => "rwc",
    };
    format!("file:{}?mode={mode}", path.as_ref().to_string_lossy())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationalRetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RelationalRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 50,
            max_delay_ms: 2_000,
        }
    }
}

type RelationalBatchOp = Box<dyn Fn(&Transaction<'_>) -> Result<(), MiniError>>;

pub struct RelationalWriteBatch {
    config: SqliteRelationalConfig,
    retry: RelationalRetryPolicy,
    ops: Vec<RelationalBatchOp>,
}

#[derive(Debug, Clone)]
pub struct AttachedRelationalDatabase {
    pub alias: String,
    pub config: SqliteRelationalConfig,
}

impl AttachedRelationalDatabase {
    pub fn new(alias: impl Into<String>, config: SqliteRelationalConfig) -> Self {
        Self {
            alias: alias.into(),
            config,
        }
    }
}

type MultiDbBatchOp = Box<dyn Fn(&Connection) -> Result<(), MiniError>>;

pub struct MultiDbRelationalWriteBatch {
    primary: SqliteRelationalConfig,
    attached: Vec<AttachedRelationalDatabase>,
    retry: RelationalRetryPolicy,
    ops: Vec<MultiDbBatchOp>,
}

impl MultiDbRelationalWriteBatch {
    pub fn new(primary: &SqliteRelationalConfig) -> Self {
        Self::new_with_retry(primary, RelationalRetryPolicy::default())
    }

    pub fn new_with_retry(primary: &SqliteRelationalConfig, retry: RelationalRetryPolicy) -> Self {
        let mut primary = primary.clone();
        primary.mode = SqliteRelationalOpenMode::ReadWriteCreate;
        Self {
            primary,
            attached: Vec::new(),
            retry,
            ops: Vec::new(),
        }
    }

    pub fn attach(
        &mut self,
        alias: impl Into<String>,
        config: &SqliteRelationalConfig,
    ) -> &mut Self {
        let mut config = config.clone();
        config.mode = SqliteRelationalOpenMode::ReadWriteCreate;
        self.attached
            .push(AttachedRelationalDatabase::new(alias, config));
        self
    }

    pub fn push<F>(&mut self, op: F) -> &mut Self
    where
        F: Fn(&Connection) -> Result<(), MiniError> + 'static,
    {
        self.ops.push(Box::new(op));
        self
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn attached_len(&self) -> usize {
        self.attached.len()
    }

    pub fn commit(self) -> Result<usize, MiniError> {
        let count = self.ops.len();
        if count == 0 {
            return Ok(0);
        }

        let mut last_busy = None;
        for attempt in 0..self.retry.max_attempts.max(1) {
            if attempt > 0 {
                std::thread::sleep(backoff_delay(attempt, &self.retry));
            }
            match try_commit_multi_db_once(&self.primary, &self.attached, &self.ops) {
                Ok(count) => return Ok(count),
                Err(RelationalBatchError::Busy(err)) => last_busy = Some(err),
                Err(RelationalBatchError::Mini(err)) => return Err(err),
            }
        }

        Err(last_busy.unwrap_or_else(|| {
            sqlite_error(BUSY_TIMEOUT, "sqlite multi-db batch exhausted busy retries")
        }))
    }
}

impl std::fmt::Debug for MultiDbRelationalWriteBatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiDbRelationalWriteBatch")
            .field("primary_database_path", &self.primary.database_path)
            .field("attached", &self.attached.len())
            .field("ops", &self.ops.len())
            .field("retry", &self.retry)
            .finish()
    }
}

impl RelationalWriteBatch {
    pub fn new(config: &SqliteRelationalConfig) -> Self {
        Self::new_with_retry(config, RelationalRetryPolicy::default())
    }

    pub fn new_with_retry(config: &SqliteRelationalConfig, retry: RelationalRetryPolicy) -> Self {
        let mut config = config.clone();
        config.mode = SqliteRelationalOpenMode::ReadWriteCreate;
        Self {
            config,
            retry,
            ops: Vec::new(),
        }
    }

    pub fn push<F>(&mut self, op: F) -> &mut Self
    where
        F: Fn(&Transaction<'_>) -> Result<(), MiniError> + 'static,
    {
        self.ops.push(Box::new(op));
        self
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn commit(self) -> Result<usize, MiniError> {
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
                Err(RelationalBatchError::Busy(err)) => last_busy = Some(err),
                Err(RelationalBatchError::Mini(err)) => return Err(err),
            }
        }

        Err(last_busy.unwrap_or_else(|| {
            sqlite_error(BUSY_TIMEOUT, "sqlite write batch exhausted busy retries")
        }))
    }
}

impl std::fmt::Debug for RelationalWriteBatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelationalWriteBatch")
            .field("database_path", &self.config.database_path)
            .field("ops", &self.ops.len())
            .field("retry", &self.retry)
            .finish()
    }
}

enum RelationalBatchError {
    Busy(MiniError),
    Mini(MiniError),
}

fn try_commit_once(
    config: &SqliteRelationalConfig,
    ops: &[RelationalBatchOp],
) -> Result<usize, RelationalBatchError> {
    let mut conn = open_relational_connection(config).map_err(RelationalBatchError::Mini)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(map_rusqlite_transaction_error)?;
    for op in ops {
        op(&tx).map_err(RelationalBatchError::Mini)?;
    }
    tx.commit().map_err(map_rusqlite_transaction_error)?;
    Ok(ops.len())
}

fn try_commit_multi_db_once(
    primary: &SqliteRelationalConfig,
    attached: &[AttachedRelationalDatabase],
    ops: &[MultiDbBatchOp],
) -> Result<usize, RelationalBatchError> {
    let conn = open_relational_connection(primary).map_err(RelationalBatchError::Mini)?;
    for database in attached {
        validate_attached_alias(&database.alias).map_err(RelationalBatchError::Mini)?;
        validate_relational_database_path(&database.config).map_err(RelationalBatchError::Mini)?;
        attach_database(&conn, database).map_err(RelationalBatchError::Mini)?;
    }

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(map_rusqlite_transaction_error)?;
    let result = (|| {
        for op in ops {
            op(&conn)?;
        }
        Ok::<(), MiniError>(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")
                .map_err(map_rusqlite_transaction_error)?;
            Ok(ops.len())
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(RelationalBatchError::Mini(err))
        }
    }
}

fn attach_database(
    conn: &Connection,
    database: &AttachedRelationalDatabase,
) -> Result<(), MiniError> {
    conn.execute(
        &format!("ATTACH DATABASE ?1 AS {}", database.alias),
        [database.config.database_path.to_string_lossy().as_ref()],
    )
    .map_err(|_| sqlite_error(OPEN_FAILED, "failed to attach sqlite database"))?;
    apply_relational_pragmas(conn, &database.config)?;
    Ok(())
}

fn validate_attached_alias(alias: &str) -> Result<(), MiniError> {
    let mut chars = alias.chars();
    let Some(first) = chars.next() else {
        return Err(sqlite_error(
            INVALID_PATH,
            "empty sqlite attached database alias",
        ));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(sqlite_error(
            INVALID_PATH,
            "invalid sqlite attached database alias",
        ));
    }
    if !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        return Err(sqlite_error(
            INVALID_PATH,
            "invalid sqlite attached database alias",
        ));
    }
    Ok(())
}

fn map_rusqlite_transaction_error(err: RusqliteError) -> RelationalBatchError {
    let message = "sqlite relational batch transaction failed";
    if is_busy_error(&err) {
        return RelationalBatchError::Busy(sqlite_error(BUSY_TIMEOUT, message));
    }
    RelationalBatchError::Mini(sqlite_error(TRANSACTION_FAILED, message))
}

fn is_busy_error(err: &RusqliteError) -> bool {
    matches!(
        err,
        RusqliteError::SqliteFailure(error, _)
            if matches!(
                error.code,
                RusqliteErrorCode::DatabaseBusy | RusqliteErrorCode::DatabaseLocked
            )
    )
}

pub fn apply_relational_pragmas(
    conn: &Connection,
    config: &SqliteRelationalConfig,
) -> Result<(), MiniError> {
    conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms))
        .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;

    if config.enable_foreign_keys {
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
    }

    match config.mode {
        SqliteRelationalOpenMode::ReadOnly => {
            conn.execute_batch(
                "PRAGMA query_only = ON;
                 PRAGMA temp_store = MEMORY;
                 PRAGMA mmap_size = 0;",
            )
            .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
        }
        SqliteRelationalOpenMode::ReadWriteCreate => {
            conn.pragma_update(None, "journal_mode", config.journal_mode.as_pragma_value())
                .map_err(|_| {
                    sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection")
                })?;
            conn.pragma_update(None, "synchronous", config.synchronous.as_pragma_value())
                .map_err(|_| {
                    sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection")
                })?;
            conn.pragma_update(None, "wal_autocheckpoint", config.wal_autocheckpoint_pages)
                .map_err(|_| {
                    sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection")
                })?;
            conn.execute_batch(
                "PRAGMA temp_store = MEMORY;
                 PRAGMA mmap_size = 0;",
            )
            .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
        }
    }

    Ok(())
}

fn validate_relational_database_path(config: &SqliteRelationalConfig) -> Result<(), MiniError> {
    validate_database_path(&SqliteConfig {
        database_path: config.database_path.clone(),
        create_parent_dir: config.create_parent_dir,
        busy_timeout_ms: config.busy_timeout_ms,
        enable_foreign_keys: config.enable_foreign_keys,
        journal_mode: config.journal_mode,
        synchronous: config.synchronous,
        wal_autocheckpoint_pages: config.wal_autocheckpoint_pages,
        write_queue_capacity: 1,
        write_batch_max_commands: 1,
        write_batch_max_delay_ms: 0,
        write_retry_max_attempts: 1,
        write_retry_base_delay_ms: 0,
        write_retry_max_delay_ms: 0,
        write_retry_jitter_ms: 0,
    })
}

fn fnv1a(value: &str) -> i64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash as i64
}

fn backoff_delay(attempt: u32, policy: &RelationalRetryPolicy) -> Duration {
    let exp_ms = policy
        .base_delay_ms
        .saturating_mul(1u64 << (attempt - 1).min(10))
        .min(policy.max_delay_ms);
    Duration::from_millis(exp_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn opens_relational_database_and_runs_migrations_once() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("relational.db");
        let config = SqliteRelationalConfig::read_write_create(&db_path);
        let conn = open_relational_connection(&config).unwrap();

        run_relational_migrations(
            &conn,
            &["CREATE TABLE demo (id TEXT PRIMARY KEY, label TEXT NOT NULL);"],
        )
        .unwrap();
        run_relational_migrations(
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
    fn relational_write_batch_commits_all_ops() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("batch.db");
        let config = SqliteRelationalConfig::read_write_create(&db_path);
        let conn = open_relational_connection(&config).unwrap();
        run_relational_migrations(
            &conn,
            &["CREATE TABLE demo (id TEXT PRIMARY KEY, label TEXT NOT NULL);"],
        )
        .unwrap();
        drop(conn);

        let mut batch = RelationalWriteBatch::new(&config);
        batch.push(|tx| {
            tx.execute("INSERT INTO demo (id, label) VALUES (?1, ?2)", ["1", "one"])
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert demo row"))?;
            Ok(())
        });
        batch.push(|tx| {
            tx.execute("INSERT INTO demo (id, label) VALUES (?1, ?2)", ["2", "two"])
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert demo row"))?;
            Ok(())
        });

        assert_eq!(batch.commit().unwrap(), 2);

        let conn = open_relational_connection(&config).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn multi_db_relational_write_batch_commits_all_attached_writes() {
        let dir = tempdir().unwrap();
        let primary_path = dir.path().join("primary.db");
        let secondary_path = dir.path().join("secondary.db");
        let primary = SqliteRelationalConfig::read_write_create(&primary_path);
        let secondary = SqliteRelationalConfig::read_write_create(&secondary_path);

        let primary_conn = open_relational_connection(&primary).unwrap();
        run_relational_migrations(
            &primary_conn,
            &["CREATE TABLE primary_demo (id TEXT PRIMARY KEY);"],
        )
        .unwrap();
        drop(primary_conn);
        let secondary_conn = open_relational_connection(&secondary).unwrap();
        run_relational_migrations(
            &secondary_conn,
            &["CREATE TABLE secondary_demo (id TEXT PRIMARY KEY);"],
        )
        .unwrap();
        drop(secondary_conn);

        let mut batch = MultiDbRelationalWriteBatch::new(&primary);
        batch.attach("secondary", &secondary);
        batch.push(|conn| {
            conn.execute("INSERT INTO primary_demo (id) VALUES ('p1')", [])
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert primary row"))?;
            Ok(())
        });
        batch.push(|conn| {
            conn.execute(
                "INSERT INTO secondary.secondary_demo (id) VALUES ('s1')",
                [],
            )
            .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert secondary row"))?;
            Ok(())
        });

        assert_eq!(batch.commit().unwrap(), 2);

        let primary_conn = open_relational_connection(&primary).unwrap();
        let primary_count: i64 = primary_conn
            .query_row("SELECT COUNT(*) FROM primary_demo", [], |row| row.get(0))
            .unwrap();
        let secondary_conn = open_relational_connection(&secondary).unwrap();
        let secondary_count: i64 = secondary_conn
            .query_row("SELECT COUNT(*) FROM secondary_demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(primary_count, 1);
        assert_eq!(secondary_count, 1);
    }

    #[test]
    fn multi_db_relational_write_batch_rolls_back_all_writes_on_failure() {
        let dir = tempdir().unwrap();
        let primary_path = dir.path().join("primary.db");
        let secondary_path = dir.path().join("secondary.db");
        let primary = SqliteRelationalConfig::read_write_create(&primary_path);
        let secondary = SqliteRelationalConfig::read_write_create(&secondary_path);

        let primary_conn = open_relational_connection(&primary).unwrap();
        run_relational_migrations(
            &primary_conn,
            &["CREATE TABLE primary_demo (id TEXT PRIMARY KEY);"],
        )
        .unwrap();
        drop(primary_conn);
        let secondary_conn = open_relational_connection(&secondary).unwrap();
        run_relational_migrations(
            &secondary_conn,
            &["CREATE TABLE secondary_demo (id TEXT PRIMARY KEY);"],
        )
        .unwrap();
        drop(secondary_conn);

        let mut batch = MultiDbRelationalWriteBatch::new(&primary);
        batch.attach("secondary", &secondary);
        batch.push(|conn| {
            conn.execute("INSERT INTO primary_demo (id) VALUES ('p1')", [])
                .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert primary row"))?;
            Ok(())
        });
        batch.push(|conn| {
            conn.execute(
                "INSERT INTO secondary.secondary_demo (id) VALUES ('s1')",
                [],
            )
            .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to insert secondary row"))?;
            Err(sqlite_error(
                EXECUTE_FAILED,
                "forced multi-db batch failure",
            ))
        });

        assert!(batch.commit().is_err());

        let primary_conn = open_relational_connection(&primary).unwrap();
        let primary_count: i64 = primary_conn
            .query_row("SELECT COUNT(*) FROM primary_demo", [], |row| row.get(0))
            .unwrap();
        let secondary_conn = open_relational_connection(&secondary).unwrap();
        let secondary_count: i64 = secondary_conn
            .query_row("SELECT COUNT(*) FROM secondary_demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(primary_count, 0);
        assert_eq!(secondary_count, 0);
    }

    #[test]
    fn relational_read_only_connection_rejects_writes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("readonly.db");
        let config = SqliteRelationalConfig::read_write_create(&db_path);
        let conn = open_relational_connection(&config).unwrap();
        run_relational_migrations(&conn, &["CREATE TABLE demo (id TEXT PRIMARY KEY);"]).unwrap();
        drop(conn);

        let read_config = SqliteRelationalConfig::read_only(&db_path);
        let conn = open_relational_connection(&read_config).unwrap();
        assert!(conn
            .execute("INSERT INTO demo (id) VALUES ('blocked')", [])
            .is_err());
    }
}

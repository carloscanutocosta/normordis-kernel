use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::storage_mode::{DbError, StorageMode};

pub struct DbManager {
    plain_pool: Option<Pool<SqliteConnectionManager>>,
    secure_pool: Option<Pool<SqliteConnectionManager>>,
    mode: StorageMode,
}

impl DbManager {
    pub fn init(data_dir: &Path, mode: StorageMode) -> Result<Self, DbError> {
        fs::create_dir_all(data_dir)
            .map_err(|e| DbError::MigrationError(e.to_string()))?;

        let plain_pool = match &mode {
            StorageMode::Plain | StorageMode::Mixed { .. } => {
                Some(build_plain_pool(&data_dir.join("app.db"))?)
            }
            StorageMode::Encrypted { .. } => None,
        };

        let secure_pool = match &mode {
            StorageMode::Encrypted { key } | StorageMode::Mixed { secure_key: key } => {
                Some(build_secure_pool(&data_dir.join("app.secure.db"), key)?)
            }
            StorageMode::Plain => None,
        };

        Ok(Self { plain_pool, secure_pool, mode })
    }

    pub fn plain(&self) -> Result<&Pool<SqliteConnectionManager>, DbError> {
        match &self.mode {
            StorageMode::Encrypted { .. } => Err(DbError::PlainDbNotAvailable),
            _ => Ok(self
                .plain_pool
                .as_ref()
                .expect("plain_pool is Some for Plain and Mixed modes")),
        }
    }

    pub fn secure(&self) -> Result<&Pool<SqliteConnectionManager>, DbError> {
        match &self.mode {
            StorageMode::Plain => Err(DbError::SecureDbNotAvailable),
            _ => Ok(self
                .secure_pool
                .as_ref()
                .expect("secure_pool is Some for Encrypted and Mixed modes")),
        }
    }

    pub fn has_secure(&self) -> bool {
        self.secure_pool.is_some()
    }

    /// Executa `sql` na base de dados plain com retry automático em caso de
    /// `SQLITE_BUSY` ou timeout de pool transitório.
    pub fn execute_plain_batch(&self, sql: &str) -> Result<(), DbError> {
        retry_batch(self.plain()?, sql)
    }

    /// Executa `sql` na base de dados segura com retry automático em caso de
    /// `SQLITE_BUSY` ou timeout de pool transitório.
    pub fn execute_secure_batch(&self, sql: &str) -> Result<(), DbError> {
        retry_batch(self.secure()?, sql)
    }
}

// ---------------------------------------------------------------------------
// Retry
// ---------------------------------------------------------------------------

const RETRY_MAX: u32 = 5;
const RETRY_BASE_MS: u64 = 10; // delays: 10 20 40 80 ms (exponential)

fn is_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::DatabaseBusy
    )
}

fn retry_batch(pool: &Pool<SqliteConnectionManager>, sql: &str) -> Result<(), DbError> {
    let mut last_err = String::new();
    for attempt in 0..RETRY_MAX {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(RETRY_BASE_MS << (attempt - 1)));
        }
        let conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        match conn.execute_batch(sql) {
            Ok(()) => return Ok(()),
            Err(e) if is_busy(&e) => last_err = e.to_string(),
            Err(e) => return Err(DbError::Sqlite(e)),
        }
    }
    Err(DbError::Exhausted(RETRY_MAX, last_err))
}

// ---------------------------------------------------------------------------
// Pool builders
// ---------------------------------------------------------------------------

fn build_plain_pool(path: &Path) -> Result<Pool<SqliteConnectionManager>, DbError> {
    let manager = SqliteConnectionManager::file(path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 0;
             PRAGMA wal_autocheckpoint = 1000;",
        )?;
        conn.busy_timeout(Duration::from_millis(5_000))?;
        Ok(())
    });

    Pool::builder().max_size(4).build(manager).map_err(DbError::Pool)
}

fn build_secure_pool(path: &Path, key: &str) -> Result<Pool<SqliteConnectionManager>, DbError> {
    validate_encryption_key(path, key)?;

    let key_escaped = key.replace('\'', "''");
    let manager = SqliteConnectionManager::file(path).with_init(move |conn| {
        conn.execute_batch(&format!("PRAGMA key = '{key_escaped}';"))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 0;
             PRAGMA wal_autocheckpoint = 1000;",
        )?;
        conn.busy_timeout(Duration::from_millis(5_000))?;
        Ok(())
    });

    Pool::builder().max_size(4).build(manager).map_err(DbError::Pool)
}

fn validate_encryption_key(path: &Path, key: &str) -> Result<(), DbError> {
    let conn = rusqlite::Connection::open(path).map_err(DbError::Sqlite)?;
    let key_escaped = key.replace('\'', "''");
    conn.execute_batch(&format!("PRAGMA key = '{key_escaped}';"))
        .map_err(DbError::Sqlite)?;
    conn.execute_batch("SELECT count(*) FROM sqlite_master;")
        .map_err(|_| DbError::InvalidEncryptionKey)
}

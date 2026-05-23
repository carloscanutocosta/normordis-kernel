use crate::config::SqliteConfig;
use crate::error::{
    sqlite_error, CHECKPOINT_FAILED, CONFIGURE_FAILED, CREATE_PARENT_DIR_FAILED, EXECUTE_FAILED,
    INIT_FAILED, INVALID_METADATA_KEY, INVALID_PATH, LOCK_FAILED, METADATA_READ_FAILED,
    METADATA_WRITE_FAILED, OPEN_FAILED, OPTIMIZE_FAILED, TRANSACTION_FAILED,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use support_errors::MiniError;

const METADATA_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mini_kernel_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

pub struct SqliteAdapter {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAdapter {
    pub fn open(config: SqliteConfig) -> Result<Self, MiniError> {
        validate_database_path(&config)?;

        if config.create_parent_dir {
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

        let conn = Connection::open(&config.database_path)
            .map_err(|_| sqlite_error(OPEN_FAILED, "failed to open sqlite database"))?;
        apply_pragmas(&conn, &config)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn initialize(&self) -> Result<(), MiniError> {
        let conn = self.lock_connection()?;
        conn.execute_batch(METADATA_SCHEMA)
            .map_err(|_| sqlite_error(INIT_FAILED, "failed to initialize sqlite schema"))
    }

    pub fn execute_batch(&self, sql: &str) -> Result<(), MiniError> {
        let conn = self.lock_connection()?;
        conn.execute_batch(sql)
            .map_err(|_| sqlite_error(EXECUTE_FAILED, "failed to execute sqlite batch"))
    }

    pub fn execute_batch_in_transaction(&self, sql: &str) -> Result<(), MiniError> {
        let mut conn = self.lock_connection()?;
        let tx = conn
            .transaction()
            .map_err(|_| sqlite_error(TRANSACTION_FAILED, "failed to start sqlite transaction"))?;
        tx.execute_batch(sql).map_err(|_| {
            sqlite_error(
                TRANSACTION_FAILED,
                "failed to execute sqlite batch in transaction",
            )
        })?;
        tx.commit()
            .map_err(|_| sqlite_error(TRANSACTION_FAILED, "failed to commit sqlite transaction"))
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<(), MiniError> {
        validate_metadata_key(key)?;
        self.initialize()?;
        let conn = self.lock_connection()?;
        conn.execute(
            "INSERT INTO mini_kernel_metadata (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![key, value, current_timestamp_text()],
        )
        .map(|_| ())
        .map_err(|_| sqlite_error(METADATA_WRITE_FAILED, "failed to write sqlite metadata"))
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>, MiniError> {
        validate_metadata_key(key)?;
        self.initialize()?;
        let conn = self.lock_connection()?;
        conn.query_row(
            "SELECT value FROM mini_kernel_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| sqlite_error(METADATA_READ_FAILED, "failed to read sqlite metadata"))
    }

    pub fn optimize(&self) -> Result<(), MiniError> {
        let conn = self.lock_connection()?;
        conn.execute_batch("PRAGMA optimize;")
            .map_err(|_| sqlite_error(OPTIMIZE_FAILED, "failed to optimize sqlite database"))
    }

    pub fn checkpoint(&self) -> Result<(), MiniError> {
        let conn = self.lock_connection()?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|_| sqlite_error(CHECKPOINT_FAILED, "failed to checkpoint sqlite database"))
    }

    pub(crate) fn lock_connection(&self) -> Result<MutexGuard<'_, Connection>, MiniError> {
        self.conn
            .lock()
            .map_err(|_| sqlite_error(LOCK_FAILED, "failed to acquire sqlite connection lock"))
    }
}

pub(crate) fn validate_database_path(config: &SqliteConfig) -> Result<(), MiniError> {
    if config.database_path.as_os_str().is_empty() {
        return Err(sqlite_error(
            INVALID_PATH,
            "sqlite database path is invalid",
        ));
    }

    Ok(())
}

pub(crate) fn validate_metadata_key(key: &str) -> Result<(), MiniError> {
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return Err(sqlite_error(
            INVALID_METADATA_KEY,
            "sqlite metadata key is invalid",
        ));
    }

    Ok(())
}

fn apply_pragmas(conn: &Connection, config: &SqliteConfig) -> Result<(), MiniError> {
    conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms))
        .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;

    if config.enable_foreign_keys {
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
    }

    conn.pragma_update(None, "journal_mode", config.journal_mode.as_pragma_value())
        .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
    conn.pragma_update(None, "synchronous", config.synchronous.as_pragma_value())
        .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;
    conn.pragma_update(None, "wal_autocheckpoint", config.wal_autocheckpoint_pages)
        .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))?;

    conn.execute_batch(
        "PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 0;",
    )
    .map_err(|_| sqlite_error(CONFIGURE_FAILED, "failed to configure sqlite connection"))
}

pub(crate) fn current_timestamp_text() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seconds.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn opens_temporary_sqlite_database() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("app.db");

        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter.initialize().unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn creates_parent_directory_when_missing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("nested").join("app.db");

        SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        assert!(db_path.parent().unwrap().exists());
    }

    #[test]
    fn initializes_technical_schema() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("schema.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter.initialize().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let table_name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'mini_kernel_metadata'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_name, "mini_kernel_metadata");
    }

    #[test]
    fn initialize_is_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("idempotent.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter.initialize().unwrap();
        adapter.initialize().unwrap();
    }

    #[test]
    fn execute_batch_runs_valid_sql() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("batch.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter
            .execute_batch(
                "CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 INSERT INTO demo (id, name) VALUES (1, 'alpha');",
            )
            .unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM demo WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "alpha");
    }

    #[test]
    fn applies_connection_pragmas() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("pragmas.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        let conn = adapter.lock_connection().unwrap();
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        let busy_timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        let wal_autocheckpoint: i64 = conn
            .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
            .unwrap();

        assert_eq!(foreign_keys, 1);
        assert_eq!(busy_timeout, 5_000);
        assert_eq!(wal_autocheckpoint, 1_000);
    }

    #[test]
    fn execute_batch_in_transaction_rolls_back_on_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tx.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let result = adapter.execute_batch_in_transaction(
            "INSERT INTO demo (id) VALUES (1);
             INSERT INTO demo (id) VALUES (1);",
        );

        assert!(result.is_err());
        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn metadata_helpers_write_and_read_values() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("metadata.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter.set_metadata("schema_version", "1").unwrap();
        adapter.set_metadata("schema_version", "2").unwrap();

        assert_eq!(
            adapter.get_metadata("schema_version").unwrap(),
            Some("2".to_owned())
        );
        assert_eq!(adapter.get_metadata("missing").unwrap(), None);
    }

    #[test]
    fn rejects_invalid_metadata_key() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("metadata-key.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        let err = adapter.set_metadata("bad key", "value").unwrap_err();

        assert_eq!(err.code.as_str(), INVALID_METADATA_KEY);
    }

    #[test]
    fn rejects_empty_database_path() {
        let err = match SqliteAdapter::open(SqliteConfig::new("")) {
            Ok(_) => panic!("empty sqlite path should fail"),
            Err(err) => err,
        };

        assert_eq!(err.code.as_str(), INVALID_PATH);
    }

    #[test]
    fn optimize_and_checkpoint_are_controlled_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ops.db");
        let adapter = SqliteAdapter::open(SqliteConfig::new(&db_path)).unwrap();

        adapter.initialize().unwrap();
        adapter.optimize().unwrap();
        adapter.checkpoint().unwrap();
    }

    #[test]
    fn open_error_is_converted_to_mini_error() {
        let dir = tempdir().unwrap();
        let result = SqliteAdapter::open(SqliteConfig::new(dir.path()));

        let err = match result {
            Ok(_) => panic!("opening a directory as sqlite database should fail"),
            Err(err) => err,
        };
        assert_eq!(err.code.as_str(), OPEN_FAILED);
        assert_eq!(err.component.as_str(), crate::error::COMPONENT);
    }

    #[test]
    fn public_error_does_not_expose_absolute_path() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().to_path_buf();

        let err = match SqliteAdapter::open(SqliteConfig::new(&db_path)) {
            Ok(_) => panic!("opening a directory as sqlite database should fail"),
            Err(err) => err,
        };
        let public = err.to_public();
        let path_text = db_path.to_string_lossy();

        assert_eq!(public.code, OPEN_FAILED);
        assert!(!public.message.contains(path_text.as_ref()));
    }
}

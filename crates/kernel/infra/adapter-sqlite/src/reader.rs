use crate::config::SqliteConfig;
use crate::connection::{validate_database_path, validate_metadata_key};
use crate::error::{
    sqlite_error, CONFIGURE_FAILED, METADATA_READ_FAILED, OPEN_FAILED, QUERY_FAILED,
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use support_errors::MiniError;

#[derive(Debug, Clone)]
pub struct SqliteReader {
    config: SqliteConfig,
}

impl SqliteReader {
    pub fn new(config: SqliteConfig) -> Self {
        Self { config }
    }

    pub fn read<T, F>(&self, work: F) -> Result<T, MiniError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.open_read_connection()?;
        work(&conn).map_err(|_| sqlite_error(QUERY_FAILED, "failed to execute sqlite read query"))
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>, MiniError> {
        validate_metadata_key(key)?;
        self.read(|conn| {
            conn.query_row(
                "SELECT value FROM mini_kernel_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
        })
        .map_err(|err| {
            if err.code.as_str() == QUERY_FAILED {
                sqlite_error(METADATA_READ_FAILED, "failed to read sqlite metadata")
            } else {
                err
            }
        })
    }

    fn open_read_connection(&self) -> Result<Connection, MiniError> {
        validate_database_path(&self.config)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
        let conn = Connection::open_with_flags(&self.config.database_path, flags)
            .map_err(|_| sqlite_error(OPEN_FAILED, "failed to open sqlite database"))?;
        apply_read_pragmas(&conn, &self.config)?;
        Ok(conn)
    }
}

fn apply_read_pragmas(conn: &Connection, config: &SqliteConfig) -> Result<(), MiniError> {
    conn.busy_timeout(std::time::Duration::from_millis(config.busy_timeout_ms))
        .map_err(|_| {
            sqlite_error(
                CONFIGURE_FAILED,
                "failed to configure sqlite read connection",
            )
        })?;

    if config.enable_foreign_keys {
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|_| {
                sqlite_error(
                    CONFIGURE_FAILED,
                    "failed to configure sqlite read connection",
                )
            })?;
    }

    conn.execute_batch(
        "PRAGMA query_only = ON;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 0;",
    )
    .map_err(|_| {
        sqlite_error(
            CONFIGURE_FAILED,
            "failed to configure sqlite read connection",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::SqliteWriteQueue;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn reader_reads_data_written_by_queue() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("read.db");
        let config = SqliteConfig::new(&db_path);
        let queue = SqliteWriteQueue::start(config.clone()).unwrap();
        let reader = SqliteReader::new(config);

        queue
            .execute_batch(
                "CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 INSERT INTO demo (id, name) VALUES (1, 'alpha');",
            )
            .unwrap();

        let name: String = reader
            .read(|conn| conn.query_row("SELECT name FROM demo WHERE id = 1", [], |row| row.get(0)))
            .unwrap();

        queue.shutdown().unwrap();
        assert_eq!(name, "alpha");
    }

    #[test]
    fn reader_gets_metadata_without_using_writer_connection() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("metadata-read.db");
        let config = SqliteConfig::new(&db_path);
        let queue = SqliteWriteQueue::start(config.clone()).unwrap();
        let reader = SqliteReader::new(config);

        queue.set_metadata("schema_version", "1").unwrap();

        assert_eq!(
            reader.get_metadata("schema_version").unwrap(),
            Some("1".to_owned())
        );
        queue.shutdown().unwrap();
    }

    #[test]
    fn wal_reader_does_not_block_queued_writer() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("wal-readers.db");
        let config = SqliteConfig::new(&db_path);
        let queue = SqliteWriteQueue::start(config.clone()).unwrap();
        let reader = SqliteReader::new(config);

        queue
            .execute_batch(
                "CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 INSERT INTO demo (id, name) VALUES (1, 'alpha');",
            )
            .unwrap();

        let (ready_tx, ready_rx) = mpsc::channel();
        let reader_thread = thread::spawn(move || {
            reader
                .read(|conn| {
                    conn.execute_batch("BEGIN;")?;
                    let _: String =
                        conn.query_row("SELECT name FROM demo WHERE id = 1", [], |row| row.get(0))?;
                    ready_tx.send(()).unwrap();
                    thread::sleep(Duration::from_millis(80));
                    conn.execute_batch("COMMIT;")?;
                    Ok(())
                })
                .unwrap();
        });

        ready_rx.recv().unwrap();
        queue
            .execute_batch("INSERT INTO demo (id, name) VALUES (2, 'beta');")
            .unwrap();

        reader_thread.join().unwrap();
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}

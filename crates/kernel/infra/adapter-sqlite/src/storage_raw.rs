use crate::config::SqliteConfig;
use crate::reader::SqliteReader;
use crate::writer::{SqliteWriteQueue, SqliteWriteQueueMetrics};
use rusqlite::{params, OptionalExtension};
use support_crypto::StorageEnvelope;
use support_storage::{RawStorage, StorageError, StorageKey, StorageNamespace};

const STORAGE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mini_storage_envelopes (
    namespace TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    envelope_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (namespace, storage_key)
);
";

#[derive(Clone)]
pub struct SqliteRawStorage {
    writer: SqliteWriteQueue,
    reader: SqliteReader,
}

impl SqliteRawStorage {
    pub fn open(config: SqliteConfig) -> Result<Self, StorageError> {
        let writer =
            SqliteWriteQueue::start(config.clone()).map_err(|_| StorageError::BackendFailed)?;
        writer
            .execute_batch(STORAGE_SCHEMA)
            .map_err(|_| StorageError::BackendFailed)?;

        Ok(Self {
            writer,
            reader: SqliteReader::new(config),
        })
    }

    pub fn shutdown(&self) -> Result<(), StorageError> {
        self.writer
            .shutdown()
            .map_err(|_| StorageError::BackendFailed)
    }

    pub fn metrics(&self) -> SqliteWriteQueueMetrics {
        self.writer.metrics()
    }
}

impl RawStorage for SqliteRawStorage {
    fn put_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<(), StorageError> {
        let envelope_json =
            serde_json::to_string(envelope).map_err(|_| StorageError::SerializationFailed)?;
        self.writer
            .put_storage_envelope(namespace.as_str(), key.as_str(), envelope_json)
            .map_err(|_| StorageError::BackendFailed)
    }

    fn put_raw_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<bool, StorageError> {
        let envelope_json =
            serde_json::to_string(envelope).map_err(|_| StorageError::SerializationFailed)?;
        self.writer
            .put_storage_envelope_if_absent(namespace.as_str(), key.as_str(), envelope_json)
            .map_err(|_| StorageError::BackendFailed)
    }

    fn get_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageEnvelope>, StorageError> {
        let envelope_json = self
            .reader
            .read(|conn| {
                conn.query_row(
                    "SELECT envelope_json
                     FROM mini_storage_envelopes
                     WHERE namespace = ?1 AND storage_key = ?2",
                    params![namespace.as_str(), key.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
            })
            .map_err(|_| StorageError::BackendFailed)?;

        envelope_json
            .map(|json| {
                serde_json::from_str(&json).map_err(|_| StorageError::DeserializationFailed)
            })
            .transpose()
    }

    fn delete_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<(), StorageError> {
        self.writer
            .delete_storage_envelope(namespace.as_str(), key.as_str())
            .map_err(|_| StorageError::BackendFailed)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use support_crypto::{SecretKey, StaticKeyProvider};
    use support_storage::{CryptoStorageProtector, JsonStorageCodec, ProtectedStorage, Storage};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn sqlite_raw_storage_writes_and_reads_protected_json() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("storage.db");
        let raw = SqliteRawStorage::open(SqliteConfig::new(&db_path)).unwrap();
        let storage = ProtectedStorage::new(
            raw,
            JsonStorageCodec,
            CryptoStorageProtector::new(StaticKeyProvider::new(
                SecretKey::new([17; support_crypto::KEY_LENGTH_BYTES]),
                None,
            )),
        );
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let value = json!({"title":"hello"});

        storage.put_json(&namespace, &key, &value).unwrap();

        assert_eq!(storage.get_json(&namespace, &key).unwrap(), Some(value));
        storage.raw().shutdown().unwrap();
    }

    #[test]
    fn sqlite_raw_storage_deletes_value() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("delete.db");
        let raw = SqliteRawStorage::open(SqliteConfig::new(&db_path)).unwrap();
        let storage = ProtectedStorage::new(
            raw,
            JsonStorageCodec,
            CryptoStorageProtector::new(StaticKeyProvider::new(
                SecretKey::new([18; support_crypto::KEY_LENGTH_BYTES]),
                None,
            )),
        );
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();

        storage
            .put_json(&namespace, &key, &json!({"title":"hello"}))
            .unwrap();
        storage.delete(&namespace, &key).unwrap();

        assert_eq!(storage.get_json(&namespace, &key).unwrap(), None);
        storage.raw().shutdown().unwrap();
    }

    #[test]
    fn sqlite_raw_storage_put_if_absent_does_not_overwrite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("insert-if-absent.db");
        let raw = SqliteRawStorage::open(SqliteConfig::new(&db_path)).unwrap();
        let storage = ProtectedStorage::new(
            raw,
            JsonStorageCodec,
            CryptoStorageProtector::new(StaticKeyProvider::new(
                SecretKey::new([19; support_crypto::KEY_LENGTH_BYTES]),
                None,
            )),
        );
        let namespace = StorageNamespace::new("audit.events").unwrap();
        let key = StorageKey::new("event-1").unwrap();

        assert!(storage
            .put_json_if_absent(&namespace, &key, &json!({"v":1}))
            .unwrap());
        assert!(!storage
            .put_json_if_absent(&namespace, &key, &json!({"v":2}))
            .unwrap());

        assert_eq!(
            storage.get_json(&namespace, &key).unwrap(),
            Some(json!({"v":1}))
        );
        storage.raw().shutdown().unwrap();
    }
}

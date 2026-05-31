use std::sync::{Arc, Mutex};

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use rusqlite::{params, OptionalExtension};
use support_storage::{Storage, StorageError, StorageKey, StorageNamespace, StorageValue};

const SCHEMA: &[&str] = &[r#"
CREATE TABLE IF NOT EXISTS kv_json (
    Namespace    TEXT NOT NULL,
    Key          TEXT NOT NULL,
    ValueJson    TEXT NOT NULL,
    UpdatedAtUtc TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (Namespace, Key)
);
"#];

/// Implementação SQLite de `Storage` sem encriptação.
/// Adequada para dados auditáveis que devem ser legíveis directamente (ex.: audit trail).
/// Thread-safe via `Arc<Mutex<Connection>>`.
#[derive(Clone, Debug)]
pub struct PlainJsonSqliteStorage {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl PlainJsonSqliteStorage {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, StorageError> {
        let conn = open_relational_connection(config).map_err(|_| StorageError::BackendFailed)?;
        run_relational_migrations(&conn, SCHEMA).map_err(|_| StorageError::BackendFailed)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, StorageError> {
        self.conn.lock().map_err(|_| StorageError::BackendFailed)
    }
}

impl Storage for PlainJsonSqliteStorage {
    fn put_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<(), StorageError> {
        let json = serde_json::to_string(value).map_err(|_| StorageError::SerializationFailed)?;
        let now = chrono::Utc::now().to_rfc3339();
        self.lock()?
            .execute(
                "INSERT OR REPLACE INTO kv_json (Namespace, Key, ValueJson, UpdatedAtUtc) \
             VALUES (?1, ?2, ?3, ?4)",
                params![namespace.as_str(), key.as_str(), json, now],
            )
            .map_err(|_| StorageError::BackendFailed)?;
        Ok(())
    }

    fn put_json_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<bool, StorageError> {
        let json = serde_json::to_string(value).map_err(|_| StorageError::SerializationFailed)?;
        let now = chrono::Utc::now().to_rfc3339();
        let rows_changed = self
            .lock()?
            .execute(
                "INSERT OR IGNORE INTO kv_json (Namespace, Key, ValueJson, UpdatedAtUtc) \
             VALUES (?1, ?2, ?3, ?4)",
                params![namespace.as_str(), key.as_str(), json, now],
            )
            .map_err(|_| StorageError::BackendFailed)?;
        Ok(rows_changed > 0)
    }

    fn get_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageValue>, StorageError> {
        let json_opt = self
            .lock()?
            .query_row(
                "SELECT ValueJson FROM kv_json WHERE Namespace = ?1 AND Key = ?2",
                params![namespace.as_str(), key.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| StorageError::BackendFailed)?;

        json_opt
            .map(|json| {
                serde_json::from_str(&json).map_err(|_| StorageError::DeserializationFailed)
            })
            .transpose()
    }

    fn delete(&self, namespace: &StorageNamespace, key: &StorageKey) -> Result<(), StorageError> {
        self.lock()?
            .execute(
                "DELETE FROM kv_json WHERE Namespace = ?1 AND Key = ?2",
                params![namespace.as_str(), key.as_str()],
            )
            .map_err(|_| StorageError::BackendFailed)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    fn open_test_storage() -> PlainJsonSqliteStorage {
        let dir = tempdir().unwrap();
        let path = dir.keep().join("audit.db");
        PlainJsonSqliteStorage::open(&SqliteRelationalConfig::read_write_create(path)).unwrap()
    }

    #[test]
    fn put_and_get_roundtrip() {
        let s = open_test_storage();
        let ns = StorageNamespace::new("audit.events").unwrap();
        let k = StorageKey::new("ev-1").unwrap();
        let v = json!({"event_type": "document.created", "actor": "user-1"});

        s.put_json(&ns, &k, &v).unwrap();
        assert_eq!(s.get_json(&ns, &k).unwrap(), Some(v));
    }

    #[test]
    fn put_overwrites_existing() {
        let s = open_test_storage();
        let ns = StorageNamespace::new("audit.events").unwrap();
        let k = StorageKey::new("ev-1").unwrap();

        s.put_json(&ns, &k, &json!({"v": 1})).unwrap();
        s.put_json(&ns, &k, &json!({"v": 2})).unwrap();
        assert_eq!(s.get_json(&ns, &k).unwrap(), Some(json!({"v": 2})));
    }

    #[test]
    fn put_if_absent_does_not_overwrite() {
        let s = open_test_storage();
        let ns = StorageNamespace::new("audit.events").unwrap();
        let k = StorageKey::new("ev-1").unwrap();

        assert!(s.put_json_if_absent(&ns, &k, &json!({"v": 1})).unwrap());
        assert!(!s.put_json_if_absent(&ns, &k, &json!({"v": 2})).unwrap());
        assert_eq!(s.get_json(&ns, &k).unwrap(), Some(json!({"v": 1})));
    }

    #[test]
    fn delete_removes_entry() {
        let s = open_test_storage();
        let ns = StorageNamespace::new("audit.events").unwrap();
        let k = StorageKey::new("ev-1").unwrap();

        s.put_json(&ns, &k, &json!({"v": 1})).unwrap();
        s.delete(&ns, &k).unwrap();
        assert_eq!(s.get_json(&ns, &k).unwrap(), None);
    }

    #[test]
    fn namespaces_are_isolated() {
        let s = open_test_storage();
        let ns_a = StorageNamespace::new("audit.a").unwrap();
        let ns_b = StorageNamespace::new("audit.b").unwrap();
        let k = StorageKey::new("same-key").unwrap();

        s.put_json(&ns_a, &k, &json!({"ns": "a"})).unwrap();
        s.put_json(&ns_b, &k, &json!({"ns": "b"})).unwrap();

        assert_eq!(s.get_json(&ns_a, &k).unwrap(), Some(json!({"ns": "a"})));
        assert_eq!(s.get_json(&ns_b, &k).unwrap(), Some(json!({"ns": "b"})));
    }
}

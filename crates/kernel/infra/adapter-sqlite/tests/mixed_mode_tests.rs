#[cfg(feature = "encrypted")]
mod tests {
    use adapter_sqlite::{DbError, DbManager, StorageMode};
    use tempfile::tempdir;

    #[test]
    fn plain_mode_secure_returns_error() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(dir.path(), StorageMode::Plain).unwrap();
        assert!(matches!(manager.secure().unwrap_err(), DbError::SecureDbNotAvailable));
    }

    #[test]
    fn encrypted_mode_plain_returns_error() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(
            dir.path(),
            StorageMode::Encrypted { key: "normaxis-test-key-dev".to_owned() },
        )
        .unwrap();
        assert!(matches!(manager.plain().unwrap_err(), DbError::PlainDbNotAvailable));
    }

    #[test]
    fn encrypted_mode_wrong_key_returns_invalid_key_error() {
        let dir = tempdir().unwrap();
        DbManager::init(
            dir.path(),
            StorageMode::Encrypted { key: "correct-key".to_owned() },
        )
        .unwrap();

        let result = DbManager::init(
            dir.path(),
            StorageMode::Encrypted { key: "wrong-key".to_owned() },
        );
        assert!(
            matches!(result, Err(DbError::InvalidEncryptionKey)),
            "chave errada deve retornar InvalidEncryptionKey"
        );
    }

    #[test]
    fn mixed_mode_both_pools_available() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(
            dir.path(),
            StorageMode::Mixed { secure_key: "normaxis-test-key-dev".to_owned() },
        )
        .unwrap();

        assert!(manager.plain().is_ok(), "plain() deve estar disponível em Mixed");
        assert!(manager.secure().is_ok(), "secure() deve estar disponível em Mixed");
        assert!(manager.has_secure());
    }

    #[test]
    fn plain_mode_has_no_secure_pool() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(dir.path(), StorageMode::Plain).unwrap();
        assert!(!manager.has_secure());
    }

    #[test]
    fn plain_pool_is_usable() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(dir.path(), StorageMode::Plain).unwrap();
        let pool = manager.plain().unwrap();
        let conn = pool.get().unwrap();
        let result: i64 =
            conn.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn secure_pool_is_usable() {
        let dir = tempdir().unwrap();
        let manager = DbManager::init(
            dir.path(),
            StorageMode::Encrypted { key: "normaxis-test-key-dev".to_owned() },
        )
        .unwrap();
        let pool = manager.secure().unwrap();
        let conn = pool.get().unwrap();
        let result: i64 =
            conn.query_row("SELECT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(result, 1);
    }
}

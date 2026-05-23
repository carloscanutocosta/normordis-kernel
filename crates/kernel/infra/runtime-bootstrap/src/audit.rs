use std::path::{Path, PathBuf};

use adapter_sqlite::SqliteConfig;
use core_audit::{AuditService, AuditStoreConfig, StorageAuditStore};
use support_crypto::{KeyProvider, KeyResolver};
use support_errors::MiniError;
use support_storage::StorageNamespace;

use crate::error::RuntimeError;
use crate::storage::RuntimeStorage;

pub const AUDIT_DB_FILE_NAME: &str = "audit.db";

pub type AuditDbStorage<P> = RuntimeStorage<P>;
pub type AuditDbStore<P> = StorageAuditStore<AuditDbStorage<P>>;
pub type AuditDbService<P> = AuditService<AuditDbStore<P>>;

#[derive(Debug, Clone)]
pub struct AuditDbConfig {
    pub database_path: PathBuf,
    pub create_parent_dir: bool,
}

impl AuditDbConfig {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            create_parent_dir: true,
        }
    }

    pub fn from_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self::new(data_dir.as_ref().join(AUDIT_DB_FILE_NAME))
    }

    fn sqlite_config(&self) -> SqliteConfig {
        let mut config = SqliteConfig::new(&self.database_path);
        config.create_parent_dir = self.create_parent_dir;
        config
    }
}

pub struct AuditDbRuntime<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    service: AuditDbService<P>,
}

impl<P> AuditDbRuntime<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    pub fn open(config: AuditDbConfig, keys: P) -> Result<Self, MiniError> {
        Self::open_with_store_config(config, keys, AuditStoreConfig::default())
    }

    pub fn open_with_namespace(
        config: AuditDbConfig,
        keys: P,
        namespace: impl Into<String>,
    ) -> Result<Self, MiniError> {
        let namespace = StorageNamespace::new(namespace.into())
            .map_err(|_| RuntimeError::AuditRuntimeFailed)?;
        Self::open_with_store_config(config, keys, AuditStoreConfig::new(namespace))
    }

    pub fn open_with_store_config(
        config: AuditDbConfig,
        keys: P,
        store_config: AuditStoreConfig,
    ) -> Result<Self, MiniError> {
        let storage = RuntimeStorage::open_sqlite_config(config.sqlite_config(), keys)
            .map_err(MiniError::from)?;
        Ok(Self::from_storage(storage, store_config))
    }

    pub(crate) fn from_storage(storage: AuditDbStorage<P>, store_config: AuditStoreConfig) -> Self {
        let store = StorageAuditStore::new(storage, store_config);
        Self {
            service: AuditService::new(store),
        }
    }

    pub fn service(&self) -> &AuditDbService<P> {
        &self.service
    }

    pub fn shutdown(&self) -> Result<(), MiniError> {
        self.service
            .store()
            .storage()
            .shutdown()
            .map_err(MiniError::from)
    }
}

#[cfg(test)]
mod tests {
    use core_audit::{
        sign_manifest, verify_signed_manifest, AuditActor, AuditSigningKey, AuditStore, AuditTarget,
    };
    use serde_json::json;
    use support_crypto::{KeyId, SecretKey, StaticKeyProvider};
    use tempfile::tempdir;

    use super::*;

    fn keys() -> StaticKeyProvider {
        StaticKeyProvider::new(
            SecretKey::new([23; support_crypto::KEY_LENGTH_BYTES]),
            Some(KeyId::new("audit-test-key").unwrap()),
        )
    }

    #[test]
    fn audit_config_uses_dedicated_audit_db_name() {
        let dir = tempdir().unwrap();
        let config = AuditDbConfig::from_data_dir(dir.path());

        assert_eq!(config.database_path, dir.path().join(AUDIT_DB_FILE_NAME));
    }

    #[test]
    fn audit_runtime_records_and_reads_from_dedicated_sqlite_db() {
        let dir = tempdir().unwrap();
        let config = AuditDbConfig::from_data_dir(dir.path());
        let runtime = AuditDbRuntime::open(config.clone(), keys()).unwrap();

        let event = runtime
            .service()
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                Some(json!({"reason":"created"})),
            )
            .unwrap();

        assert!(config.database_path.exists());
        assert_eq!(
            runtime.service().get(&event.event_id).unwrap(),
            Some(event.clone())
        );
        assert_eq!(
            runtime.service().list_by_actor("user-1").unwrap(),
            vec![event]
        );
        assert_eq!(runtime.service().verify_chain().unwrap().checked_events, 1);
        let manifest = runtime.service().export_manifest().unwrap();
        assert_eq!(manifest.events_count, 1);
        let signing_key = AuditSigningKey::from_bytes([31; 32]);
        let signed = sign_manifest(
            manifest,
            &signing_key,
            Some("audit-export-test".to_string()),
        )
        .unwrap();
        verify_signed_manifest(&signed).unwrap();
        runtime.shutdown().unwrap();
    }

    #[test]
    fn audit_runtime_rejects_duplicate_event_via_sqlite_insert_if_absent() {
        let dir = tempdir().unwrap();
        let runtime =
            AuditDbRuntime::open(AuditDbConfig::from_data_dir(dir.path()), keys()).unwrap();
        let event = core_audit::AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            None,
        )
        .unwrap();

        runtime.service().store().record(&event).unwrap();
        let err = runtime.service().store().record(&event).unwrap_err();

        assert_eq!(err, core_audit::AuditError::DuplicateEvent);
        runtime.shutdown().unwrap();
    }
}

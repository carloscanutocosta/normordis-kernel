use std::path::{Path, PathBuf};

use adapter_audit_sqlite::{AuditSqliteStore, DetailsEncryptor};
use adapter_sqlite::SqliteRelationalConfig;
use core_audit::{AuditError, AuditService};
use support_crypto::{
    decrypt_text_with_key, encrypt_text_with_key, EncryptedPayload, KeyProvider, KeyResolver,
    SecretKey,
};
use support_errors::MiniError;
use zeroize::Zeroizing;

use crate::error::RuntimeError;

pub const AUDIT_DB_FILE_NAME: &str = "audit.db";

pub type AuditDbStore = AuditSqliteStore<CryptoDetailsEncryptor>;
pub type AuditDbService = AuditService<AuditDbStore>;

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

    fn sqlite_relational_config(&self) -> SqliteRelationalConfig {
        SqliteRelationalConfig::read_write_create(&self.database_path)
    }
}

// ─── CryptoDetailsEncryptor ──────────────────────────────────────────────────

/// Encriptador de `details_json` com XChaCha20-Poly1305 (via `support-crypto`).
///
/// O `event_id` é usado como AAD (additional authenticated data), vinculando a
/// cifra ao evento específico e impedindo substituição de cifratextos entre eventos.
///
/// A chave é extraída do provider no momento da construção — `AuditDbRuntime`
/// não fica genérico sobre o tipo de provider.
pub struct CryptoDetailsEncryptor {
    key_bytes: Zeroizing<[u8; support_crypto::KEY_LENGTH_BYTES]>,
}

impl std::fmt::Debug for CryptoDetailsEncryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CryptoDetailsEncryptor([REDACTED])")
    }
}

impl CryptoDetailsEncryptor {
    pub fn from_provider<P: KeyProvider>(provider: &P) -> Result<Self, MiniError> {
        let key = provider.current_key()?;
        Ok(Self {
            key_bytes: Zeroizing::new(key.0),
        })
    }
}

const ENC_PREFIX: &str = "enc1:";

impl DetailsEncryptor for CryptoDetailsEncryptor {
    fn encrypt(&self, plaintext: &str, aad: &[u8]) -> Result<String, AuditError> {
        let key = SecretKey::new(*self.key_bytes);
        let payload = encrypt_text_with_key(plaintext, &key, Some(aad), None)
            .map_err(|_| AuditError::SerializationFailed)?;
        let json = serde_json::to_string(&payload).map_err(|_| AuditError::SerializationFailed)?;
        Ok(format!("{ENC_PREFIX}{json}"))
    }

    fn decrypt(&self, stored: &str, aad: &[u8]) -> Result<String, AuditError> {
        let data = stored
            .strip_prefix(ENC_PREFIX)
            .ok_or(AuditError::DeserializationFailed)?;
        let payload: EncryptedPayload =
            serde_json::from_str(data).map_err(|_| AuditError::DeserializationFailed)?;
        let key = SecretKey::new(*self.key_bytes);
        decrypt_text_with_key(&payload, &key, Some(aad))
            .map_err(|_| AuditError::DeserializationFailed)
    }
}

// ─── AuditDbRuntime ───────────────────────────────────────────────────────────

pub struct AuditDbRuntime {
    service: AuditDbService,
}

impl AuditDbRuntime {
    pub fn open<P: KeyProvider + KeyResolver>(
        config: AuditDbConfig,
        keys: P,
    ) -> Result<Self, MiniError> {
        if config.create_parent_dir {
            if let Some(parent) = config.database_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| RuntimeError::AuditRuntimeFailed)?;
            }
        }
        let encryptor =
            CryptoDetailsEncryptor::from_provider(&keys).map_err(|_| RuntimeError::AuditRuntimeFailed)?;
        let store =
            AuditSqliteStore::open_with_encryptor(&config.sqlite_relational_config(), encryptor)
                .map_err(|_| RuntimeError::AuditRuntimeFailed)?;
        Ok(Self {
            service: AuditService::new(store),
        })
    }

    pub fn service(&self) -> &AuditDbService {
        &self.service
    }

    pub fn shutdown(&self) -> Result<(), MiniError> {
        self.service.store().shutdown();
        Ok(())
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
            runtime.service().list_by_actor("user-1", 10, 0).unwrap(),
            vec![event]
        );
        assert_eq!(runtime.service().verify_chain().unwrap().checked_events, 1);
        let manifest = runtime.service().export_manifest().unwrap();
        assert_eq!(manifest.events_count, 1);
        let signing_key = AuditSigningKey::from_bytes([31; 32]);
        let signed = sign_manifest(manifest, &signing_key, Some("audit-export-test".to_string()))
            .unwrap();
        verify_signed_manifest(&signed).unwrap();
        runtime.shutdown().unwrap();
    }

    #[test]
    fn audit_runtime_rejects_duplicate_event() {
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

    #[test]
    fn details_json_is_encrypted_at_rest() {
        let dir = tempdir().unwrap();
        let config = AuditDbConfig::from_data_dir(dir.path());
        let runtime = AuditDbRuntime::open(config, keys()).unwrap();

        let event = runtime
            .service()
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                Some(json!({"dado_sensivel": "valor_secreto"})),
            )
            .unwrap();

        // Verifica que o plaintext não aparece no ficheiro SQLite
        let db_path = dir.path().join(AUDIT_DB_FILE_NAME);
        let raw_bytes = std::fs::read(&db_path).unwrap();
        let raw_str = String::from_utf8_lossy(&raw_bytes);
        assert!(
            !raw_str.contains("valor_secreto"),
            "plaintext não deve aparecer no ficheiro SQLite"
        );

        // Verifica que a leitura desencripta correctamente
        let retrieved = runtime.service().get(&event.event_id).unwrap().unwrap();
        assert_eq!(
            retrieved.details_json,
            Some(json!({"dado_sensivel": "valor_secreto"}))
        );

        runtime.shutdown().unwrap();
    }

    #[test]
    fn verify_chain_works_with_encrypted_store() {
        let dir = tempdir().unwrap();
        let runtime =
            AuditDbRuntime::open(AuditDbConfig::from_data_dir(dir.path()), keys()).unwrap();

        for i in 1..=3u32 {
            runtime
                .service()
                .record_event(
                    "document.created",
                    AuditActor::new(format!("user-{i}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{i}")).unwrap(),
                    Some(json!({"index": i})),
                )
                .unwrap();
        }

        let report = runtime.service().verify_chain().unwrap();
        assert_eq!(report.checked_events, 3);
        assert!(report.head_record_hash.is_some());
        runtime.shutdown().unwrap();
    }

    #[test]
    fn verify_chain_since_works_with_encrypted_store() {
        let dir = tempdir().unwrap();
        let runtime =
            AuditDbRuntime::open(AuditDbConfig::from_data_dir(dir.path()), keys()).unwrap();

        for i in 1..=5u32 {
            runtime
                .service()
                .record_event(
                    "document.created",
                    AuditActor::new(format!("user-{i}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{i}")).unwrap(),
                    None,
                )
                .unwrap();
        }

        let report = runtime.service().verify_chain_since(3).unwrap();
        assert_eq!(report.checked_events, 3);
        runtime.shutdown().unwrap();
    }
}

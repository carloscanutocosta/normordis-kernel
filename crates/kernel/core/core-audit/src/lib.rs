mod actor;
mod chain;
mod config;
mod error;
mod event;
mod integrity;
mod policy;
mod query;
mod service;
mod signature;
mod store;
mod target;

pub use actor::AuditActor;
pub use chain::{
    AuditChainIndex, AuditChainIndexEntry, AuditChainLink, AuditChainReport, AuditChainState,
    AuditExportManifest,
};
pub use config::{AuditStoreConfig, DEFAULT_AUDIT_EVENTS_NAMESPACE};
pub use error::{
    AuditError, AUDIT_COMPONENT, CHAIN_VERIFICATION_FAILED, DESERIALIZATION_FAILED,
    DETAILS_TOO_LARGE, DUPLICATE_EVENT, INTEGRITY_FAILED, INVALID_ACTOR, INVALID_EVENT_TYPE,
    INVALID_TARGET, OPERATION_FAILED, SENSITIVE_DETAILS, SERIALIZATION_FAILED,
    SIGNATURE_VERIFICATION_FAILED, SIGN_FAILED, STORE_FAILED,
};
pub use event::AuditEvent;
pub use integrity::event_hash;
pub use policy::{
    DEFAULT_MAX_ACTOR_FIELD_CHARS, DEFAULT_MAX_DETAILS_BYTES, DEFAULT_MAX_EVENT_TYPE_CHARS,
    DEFAULT_MAX_TARGET_FIELD_CHARS,
};
pub use query::audit_event_key;
pub use service::AuditService;
pub use signature::{
    sign_manifest, verify_signed_manifest, AuditManifestSignature, AuditSigningKey,
    SignedAuditExportManifest, AUDIT_SIGNATURE_ALGORITHM,
};
pub use store::{AuditStore, StorageAuditStore};
pub use target::AuditTarget;

#[cfg(test)]
mod manifest_tests {
    use std::fs;

    fn manifest() -> String {
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap()
    }

    #[test]
    fn core_audit_does_not_depend_on_sqlite() {
        let manifest = manifest();

        assert!(!manifest.contains("rusqlite"));
        assert!(!manifest.contains("adapter-sqlite"));
    }

    #[test]
    fn core_audit_does_not_depend_on_tauri() {
        assert!(!manifest().contains("tauri"));
    }

    #[test]
    fn core_audit_does_not_depend_on_core_config() {
        assert!(!manifest().contains("core-config"));
    }

    #[test]
    fn core_audit_does_not_depend_on_support_logging() {
        assert!(!manifest().contains("support-logging"));
    }
}

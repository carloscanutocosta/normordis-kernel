pub mod adapters;
pub mod error;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "test-helpers"))]
pub use adapters::{
    DeterministicScanner, MemoryStoragePort, PassthroughContentValidator,
    RejectingContentValidator,
};
pub use error::{
    IngestError, AUDIT_ERROR, CONTENT_VALIDATION_FAILED, HASH_MISMATCH, INGEST_COMPONENT,
    INVALID_REQUEST, MARSHAL_FAILED, MISSING_FIELD, OVERSIZED, SCAN_FAILED, SCAN_REJECTED,
    STORE_FAILED,
};
pub use service::{
    build_ingest_audit_event, process_bundle, validate_ingest_bundle, validate_ingest_evidence,
};
pub use types::{
    AuditEvidence, ContentValidator, HashEvidence, IngestBundle, IngestConfig, IngestDecision,
    IngestEvidence, IngestOutcome, IngestSource, IngestStoragePort, Outcome, ScanAdapter,
    ScanEvidence, ScanInput, ScanResult, ValidationEvidence,
};

#[cfg(test)]
mod dependency_tests {
    #[test]
    fn core_ingest_nao_depende_de_sqlite() {
        let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(!m.contains("rusqlite") && !m.contains("adapter-sqlite"));
    }

    #[test]
    fn core_ingest_nao_depende_de_tauri() {
        assert!(
            !include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).contains("tauri")
        );
    }

    #[test]
    fn core_ingest_nao_depende_de_core_exports() {
        let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(!m.contains("core-exports"), "core-ingest não deve depender de core-exports");
    }
}

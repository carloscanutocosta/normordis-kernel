pub mod adapters;
pub mod error;
pub mod service;
pub mod types;

#[cfg(any(test, feature = "test-helpers"))]
pub use adapters::{DeterministicScanner, MemoryRouter};
pub use error::{
    IngestError, AUDIT_ERROR, HASH_MISMATCH, INGEST_COMPONENT, INVALID_REQUEST, MARSHAL_FAILED,
    MISSING_FIELD, OVERSIZED, ROUTE_UNAVAILABLE, SCAN_FAILED, SCAN_REJECTED,
};
pub use service::{
    build_ingest_audit_event, process_export_snapshot, validate_ingest_evidence,
    validate_ingest_request,
};
pub use types::{
    AuditEvidence, HashEvidence, IngestConfig, IngestEvidence, IngestOutcome, IngestRequest,
    IngestSource, Outcome, RouteEvidence, RouteInput, RouteResult, Router, ScanAdapter,
    ScanEvidence, ScanInput, ScanResult, ValidationEvidence, DECISION_ACCEPTED, DECISION_REJECTED,
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
}

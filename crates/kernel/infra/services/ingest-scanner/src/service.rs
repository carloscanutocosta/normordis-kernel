use chrono::Utc;

use adapter_scanner::ScannedDocument;
use core_ingest::{
    process_bundle, IngestBundle, IngestConfig, IngestError, IngestOutcome, IngestSource,
    IngestStoragePort, ScanAdapter, ScanInput, ScanResult,
};

pub const SCANNED_DOCUMENT_KIND: &str = "scanned_document";

/// Request to ingest a scanned document through the audit pipeline.
pub struct ScanIngestRequest {
    /// Unique request identifier (caller-generated).
    pub request_id: String,
    /// Subject being digitized (e.g. `"process:2026-001"`).
    pub subject_id: String,
    /// Sequence or version within the subject (e.g. `"1"`).
    pub version: String,
    /// Output from `ScannerClient::scan()`.
    pub document: ScannedDocument,
}

/// Trivial `ScanAdapter` for use when the physical scan itself is trusted
/// and no additional policy check is required.
pub struct AlwaysCleanScanner {
    pub adapter_name: String,
}

impl Default for AlwaysCleanScanner {
    fn default() -> Self {
        Self {
            adapter_name: "always-clean".into(),
        }
    }
}

impl ScanAdapter for AlwaysCleanScanner {
    fn scan(&self, _: &ScanInput) -> Result<ScanResult, IngestError> {
        Ok(ScanResult {
            adapter: self.adapter_name.clone(),
            verdict: "clean".into(),
            reason: None,
        })
    }

    fn adapter_id(&self) -> &str {
        &self.adapter_name
    }
}

/// Returns a pre-configured `IngestConfig` for scanned document ingestion.
///
/// Sets `allowed_source_kinds` to `["scanned_document"]` and plugs in `AlwaysCleanScanner`.
/// Callers that need a policy scanner can override `scanner` after calling this helper.
pub fn scanned_document_ingest_config(
    storage: Box<dyn IngestStoragePort>,
    actor: String,
) -> IngestConfig {
    IngestConfig {
        scanner: Some(Box::new(AlwaysCleanScanner::default())),
        content_validator: None,
        storage: Some(storage),
        max_bundle_bytes: None,
        actor,
        now: None,
        allowed_source_kinds: Some(vec![SCANNED_DOCUMENT_KIND.to_string()]),
    }
}

/// Wraps a `ScannedDocument` in an `IngestBundle` and runs it through the audit pipeline.
///
/// Raw scan bytes are passed directly in `bundle.raw`; the `IngestStoragePort` implementation
/// in `ingest_cfg.storage` is responsible for storing them and returning a `document_ref`.
/// The hash is computed internally by `process_bundle` (SHA-256 over raw bytes).
pub fn ingest_scanned_document(
    req: &ScanIngestRequest,
    correlation_id: &str,
    ingest_cfg: IngestConfig,
) -> IngestOutcome {
    let received_at = ingest_cfg.now.map(|f| f()).unwrap_or_else(Utc::now);

    let bundle = IngestBundle {
        bundle_id: req.request_id.clone(),
        received_at,
        source: IngestSource {
            kind: SCANNED_DOCUMENT_KIND.into(),
            subject_id: req.subject_id.clone(),
            version: req.version.clone(),
        },
        raw: req.document.data.clone(),
        content_type: req.document.content_type.clone(),
        declared_hash: None,
        meta: Some(serde_json::json!({
            "format": req.document.format.mime_type(),
            "size_bytes": req.document.data.len(),
        })),
    };

    process_bundle(&bundle, correlation_id, &ingest_cfg)
}

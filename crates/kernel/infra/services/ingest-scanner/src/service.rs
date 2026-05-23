use chrono::Utc;
use serde_json::json;

use adapter_scanner::ScannedDocument;
use core_documental::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};
use core_exports::{canonical_bytes, build_export_receipt, BuildSnapshotConfig, SourceRef};
use core_ingest::{
    process_export_snapshot, IngestConfig, IngestError, IngestOutcome, IngestRequest, IngestSource,
    ScanAdapter, ScanInput, ScanResult,
};
use core_validation::sha256_bytes;

use crate::error::ScanIngestError;

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
        Self { adapter_name: "always-clean".into() }
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
    router: Box<dyn core_ingest::Router>,
    actor: String,
) -> IngestConfig {
    IngestConfig {
        scanner: Some(Box::new(AlwaysCleanScanner::default())),
        router: Some(router),
        max_bundle_bytes: None,
        actor,
        now: None,
        allowed_source_kinds: Some(vec![SCANNED_DOCUMENT_KIND.to_string()]),
    }
}

/// Wraps a `ScannedDocument` in an auditable ingest pipeline and returns the outcome.
///
/// Internally:
/// 1. Hashes the raw scan bytes (SHA-256, content-addressed).
/// 2. Builds a `DocumentPackage` with the scan as a single artefact.
/// 3. Creates an `ExportSnapshot` via `build_export_receipt`.
/// 4. Computes `expected_hash = sha256(canonical_bytes(snapshot))`.
/// 5. Calls `process_export_snapshot` and returns the `IngestOutcome`.
///
/// The raw scan bytes are NOT embedded in the snapshot — they are identified by
/// `artefact_ref` (first 16 hex chars of SHA-256). The caller is responsible for
/// storing the raw bytes using that ref as the storage key.
pub fn ingest_scanned_document(
    req: &ScanIngestRequest,
    correlation_id: &str,
    ingest_cfg: IngestConfig,
) -> Result<IngestOutcome, ScanIngestError> {
    let now = ingest_cfg.now.map(|f| f()).unwrap_or_else(Utc::now);
    let actor = ingest_cfg.actor.clone();

    // 1. Hash the raw scan bytes
    let scan_hash_hex = sha256_bytes(&req.document.data);
    let scan_hash = format!("sha256:{}", scan_hash_hex);
    let artefact_ref = format!("sha256:{}", &scan_hash_hex[..16]);

    // 2. Build DocumentPackage (metadata only — bytes stored externally by caller)
    let pkg = DocumentPackage {
        document_id: format!(
            "doc:{}:{}:{}",
            SCANNED_DOCUMENT_KIND, req.subject_id, req.version
        ),
        created_at: now,
        template: TemplateRef {
            template_id: "raw-scan".into(),
            template_version: "v1".into(),
            valid_at: None,
        },
        engine: EngineRef {
            engine_id: "escl-airscan".into(),
            engine_version: "v1".into(),
        },
        artefacts: vec![Artefact {
            kind: "scan_payload".into(),
            artefact_ref: artefact_ref.clone(),
            hash_result: HashResult {
                algorithm: "SHA-256".into(),
                hash: scan_hash,
                timestamp: now,
                input_kind: Some(req.document.format.mime_type().into()),
                input_ref: Some(format!("{}:{}", req.subject_id, req.version)),
                meta: None,
            },
            mime: Some(req.document.content_type.clone()),
            size_bytes: Some(req.document.data.len()),
        }],
        subject: Some(json!({
            "kind": SCANNED_DOCUMENT_KIND,
            "subject_id": req.subject_id,
            "version": req.version,
        })),
        meta: Some(json!({
            "format": req.document.format.mime_type(),
            "source": "escl-airscan",
            "artefact_ref": artefact_ref,
        })),
    };

    // 3. Build ExportSnapshot
    let source_ref = SourceRef {
        kind: SCANNED_DOCUMENT_KIND.into(),
        subject_id: req.subject_id.clone(),
        version: req.version.clone(),
    };
    let receipt = build_export_receipt(
        pkg,
        source_ref,
        BuildSnapshotConfig {
            exported_at: Some(now),
            actor: actor.clone(),
            correlation_id: correlation_id.to_string(),
            transport: Some("digital-scan".into()),
        },
    )
    .map_err(|e| ScanIngestError::SnapshotBuild(e.to_string()))?;

    // 4. Compute expected_hash = sha256(canonical_bytes(snapshot))
    let canonical = canonical_bytes(&receipt.snapshot)
        .map_err(|e| ScanIngestError::Serialization(e.to_string()))?;
    let expected_hash = format!("sha256:{}", sha256_bytes(&canonical));

    // 5. Build IngestRequest and run pipeline
    let ingest_req = IngestRequest {
        request_id: req.request_id.clone(),
        received_at: now,
        source: IngestSource {
            kind: SCANNED_DOCUMENT_KIND.into(),
            subject_id: req.subject_id.clone(),
            version: req.version.clone(),
        },
        expected_hash,
        bundle: receipt.snapshot,
        meta: Some(json!({
            "format": req.document.format.mime_type(),
            "size_bytes": req.document.data.len(),
        })),
    };

    Ok(process_export_snapshot(&ingest_req, correlation_id, &ingest_cfg))
}

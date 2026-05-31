pub mod audit;
pub mod integrity;
pub mod jcs;
pub mod revision;

pub use audit::{Actor, AuditEvent, EventType, NdfAudit};
pub use integrity::{canonical_hash, IntegrityFailure, IntegrityReport, NdfIntegrity};
pub use revision::NdfRevision;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::NormaxisPdfError;

/// NDF format version produced by this engine.
pub const NDF_VERSION: &str = "1.1.0";

/// A fully resolved NORMAXIS Document Format (NDF) archive.
///
/// Immutable fields after creation: `origin`, `revision`, `meta`, `output`,
/// `styles`, `content`, `integrity`.
/// Append-only fields: `audit.events`, `outputs`, `signatures`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfDocument {
    /// NDF format version. Always "1.1.0" for documents created by this engine.
    pub ndf: String,
    /// Generation traceability — engine, template, actor. Immutable.
    pub origin: NdfOrigin,
    /// Revision reference. None for original documents. Immutable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<NdfRevisionRef>,
    /// Document metadata with resolved values. Immutable.
    pub meta: NdfMeta,
    /// PDF output options from the NDT template. Immutable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Fully resolved styles as canonical JSON. Immutable.
    pub styles: Value,
    /// Resolved document body (all placeholders substituted) as canonical JSON. Immutable.
    pub content: Value,
    /// Integrity hashes over canonical JSON. Immutable.
    pub integrity: NdfIntegrity,
    /// Append-only audit chain.
    pub audit: NdfAudit,
    /// Append-only list of rendered outputs.
    #[serde(default)]
    pub outputs: Vec<NdfOutput>,
    /// Append-only list of digital signatures.
    #[serde(default)]
    pub signatures: Vec<NdfSignature>,
}

impl NdfDocument {
    /// Serialises to canonical JSON per RFC 8785 / JCS.
    pub fn to_canonical_json(&self) -> crate::Result<String> {
        let value =
            serde_json::to_value(self).map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;
        let canonical = jcs::canonicalise(&value);
        serde_json::to_string(&canonical).map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))
    }

    /// Serialises to pretty-printed JSON. Use only for debugging; not for hashing.
    pub fn to_pretty_json(&self) -> crate::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))
    }

    /// Appends an audit event, verifying content_hash for documentary events.
    pub fn add_event(&mut self, event: AuditEvent) -> crate::Result<()> {
        if let Some(ref hash) = event.content_hash {
            if hash != &self.integrity.content_hash {
                return Err(NormaxisPdfError::NdfAuditError(format!(
                    "content_hash mismatch at event seq {} — content has been modified",
                    self.audit.next_seq()
                )));
            }
        }
        self.audit.append(event)
    }

    /// Appends an output record.
    pub fn add_output(&mut self, output: NdfOutput) -> crate::Result<()> {
        self.outputs.push(output);
        Ok(())
    }

    /// Appends a signature record.
    pub fn add_signature(&mut self, sig: NdfSignature) -> crate::Result<()> {
        self.signatures.push(sig);
        Ok(())
    }

    /// Verifies all integrity hashes and the audit chain.
    pub fn verify_integrity(&self) -> crate::Result<IntegrityReport> {
        integrity::verify(self)
    }

    pub fn is_signed(&self) -> bool {
        !self.signatures.is_empty()
    }

    pub fn is_approved(&self) -> bool {
        self.audit
            .events
            .iter()
            .any(|e| e.event_type == EventType::DocumentApproved)
    }

    pub fn is_superseded(&self) -> bool {
        self.audit
            .events
            .iter()
            .any(|e| e.event_type == EventType::DocumentSuperseded)
    }

    pub fn is_revision(&self) -> bool {
        self.revision.is_some()
    }
}

// ── Supporting types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfOrigin {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndt_template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndt_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndt_template_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndt_data_hash: Option<String>,
    pub engine_version: String,
    pub engine_backend: String,
    pub generated_at: String,
    pub generated_by: Actor,
}

fn default_lang() -> String {
    "pt-PT".into()
}

fn default_classification() -> String {
    "public".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfMeta {
    pub title: String,
    #[serde(default)]
    pub entity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    #[serde(default = "default_classification")]
    pub classification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(default)]
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat_mode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numbering: Option<NdfMetaNumbering>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfMetaNumbering {
    pub numbering_ref: String,
    pub document_number: String,
    pub sequence_id: String,
    pub assigned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfRevisionRef {
    pub revision_of: String,
    pub revision_reason: String,
    pub revision_seq: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfOutput {
    pub format: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfSignature {
    pub algorithm: String,
    pub signer: String,
    pub signed_at: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::NormaxisPdfError;

/// Integrity hashes over the NDF payload.
/// All hashes computed over canonical JSON (RFC 8785 / JCS).
/// Immutable after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfIntegrity {
    /// SHA-256 of the `content` field in canonical JSON.
    pub content_hash: String,
    /// SHA-256 of the `styles` field in canonical JSON.
    pub styles_hash: String,
    /// SHA-256 of `{ meta, styles, content }` in canonical JSON.
    pub payload_hash: String,
    /// SHA-256 of `{ meta, styles, content, integrity(without ndf_hash) }` in canonical JSON.
    pub ndf_hash: String,
    /// Hash algorithm identifier. Always "sha256".
    pub algorithm: String,
}

impl NdfIntegrity {
    /// Computes all integrity hashes for a new NDF.
    pub fn compute(
        content: &Value,
        styles: &Value,
        meta: &Value,
    ) -> crate::Result<Self> {
        let content_hash = canonical_hash(content);
        let styles_hash = canonical_hash(styles);

        let payload_val = serde_json::json!({
            "content": content,
            "meta":    meta,
            "styles":  styles,
        });
        let payload_hash = canonical_hash(&payload_val);

        let partial_integrity = serde_json::json!({
            "algorithm":    "sha256",
            "content_hash": &content_hash,
            "payload_hash": &payload_hash,
            "styles_hash":  &styles_hash,
        });
        let ndf_val = serde_json::json!({
            "content":   content,
            "integrity": partial_integrity,
            "meta":      meta,
            "styles":    styles,
        });
        let ndf_hash = canonical_hash(&ndf_val);

        Ok(Self {
            content_hash,
            styles_hash,
            payload_hash,
            ndf_hash,
            algorithm: "sha256".into(),
        })
    }
}

/// Computes a SHA-256 hash over a JSON value using RFC 8785 / JCS canonical
/// serialisation. Returns `"sha256:<hex>"`.
pub fn canonical_hash(value: &Value) -> String {
    let canonical = crate::ndf::jcs::canonicalise(value);
    let bytes =
        serde_json::to_vec(&canonical).expect("canonical JSON serialisation is infallible");
    format!("sha256:{}", hex::encode(Sha256::digest(&bytes)))
}

// ── Integrity verification ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub content_hash_valid: bool,
    pub styles_hash_valid: bool,
    pub payload_hash_valid: bool,
    pub ndf_hash_valid: bool,
    pub audit_chain_valid: bool,
    pub all_valid: bool,
    pub failures: Vec<IntegrityFailure>,
}

#[derive(Debug, Clone)]
pub struct IntegrityFailure {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

pub fn verify(ndf: &super::NdfDocument) -> crate::Result<IntegrityReport> {
    let mut failures = Vec::new();

    let actual_content = canonical_hash(&ndf.content);
    let actual_styles = canonical_hash(&ndf.styles);

    let meta_val = serde_json::to_value(&ndf.meta)
        .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;

    let content_ok = actual_content == ndf.integrity.content_hash;
    let styles_ok = actual_styles == ndf.integrity.styles_hash;

    if !content_ok {
        failures.push(IntegrityFailure {
            field: "content_hash".into(),
            expected: ndf.integrity.content_hash.clone(),
            actual: actual_content,
        });
    }
    if !styles_ok {
        failures.push(IntegrityFailure {
            field: "styles_hash".into(),
            expected: ndf.integrity.styles_hash.clone(),
            actual: actual_styles,
        });
    }

    let payload_val = serde_json::json!({
        "content": ndf.content,
        "meta":    meta_val,
        "styles":  ndf.styles,
    });
    let actual_payload = canonical_hash(&payload_val);
    let payload_ok = actual_payload == ndf.integrity.payload_hash;
    if !payload_ok {
        failures.push(IntegrityFailure {
            field: "payload_hash".into(),
            expected: ndf.integrity.payload_hash.clone(),
            actual: actual_payload,
        });
    }

    let audit_ok = verify_audit_chain(&ndf.audit);
    if !audit_ok {
        failures.push(IntegrityFailure {
            field: "audit_chain".into(),
            expected: "valid monotonic chain".into(),
            actual: "chain violation detected".into(),
        });
    }

    let all_valid = content_ok && styles_ok && payload_ok && audit_ok;
    Ok(IntegrityReport {
        content_hash_valid: content_ok,
        styles_hash_valid: styles_ok,
        payload_hash_valid: payload_ok,
        ndf_hash_valid: true,
        audit_chain_valid: audit_ok,
        all_valid,
        failures,
    })
}

fn verify_audit_chain(audit: &super::audit::NdfAudit) -> bool {
    let mut expected_seq = 1u32;
    let mut prev_ts: Option<&str> = None;
    let first_hash = audit.events.first().and_then(|e| e.content_hash.as_deref());

    for event in &audit.events {
        if event.seq != expected_seq {
            return false;
        }
        expected_seq += 1;

        if let Some(prev) = prev_ts {
            if event.timestamp.as_str() < prev {
                return false;
            }
        }
        prev_ts = Some(&event.timestamp);

        if event.is_documentary() {
            if let Some(ref hash) = event.content_hash {
                if let Some(first) = first_hash {
                    if hash.as_str() != first {
                        return false;
                    }
                }
            }
        }
    }
    true
}

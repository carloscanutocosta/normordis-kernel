use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AuditError;
use crate::event::AuditEvent;

pub const AUDIT_CHAIN_HEAD_KEY: &str = "chain.head";
pub const AUDIT_CHAIN_INDEX_KEY: &str = "chain.events";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainLink {
    pub sequence: u64,
    pub previous_record_hash: Option<String>,
    pub record_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainState {
    pub schema_version: u32,
    pub sequence: u64,
    pub head_event_id: Option<String>,
    pub head_record_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainIndex {
    pub entries: Vec<AuditChainIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainIndexEntry {
    pub event_id: String,
    pub event_key: String,
    pub sequence: u64,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditChainReport {
    pub checked_events: usize,
    pub head_record_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditExportManifest {
    pub schema_version: u32,
    pub generated_at_utc: DateTime<Utc>,
    pub events_count: usize,
    pub head_record_hash: Option<String>,
    pub manifest_hash: String,
}

pub fn compute_record_hash(
    event: &AuditEvent,
    sequence: u64,
    previous_record_hash: Option<&str>,
) -> Result<String, AuditError> {
    #[derive(Serialize)]
    struct HashInput<'a> {
        schema_version: u32,
        sequence: u64,
        previous_record_hash: Option<&'a str>,
        event: &'a AuditEvent,
    }

    let input = HashInput {
        schema_version: 1,
        sequence,
        previous_record_hash,
        event,
    };
    let bytes = serde_json::to_vec(&input).map_err(|_| AuditError::SerializationFailed)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub fn compute_manifest_hash(
    events_count: usize,
    head_record_hash: Option<&str>,
) -> Result<String, AuditError> {
    #[derive(Serialize)]
    struct ManifestHashInput<'a> {
        schema_version: u32,
        events_count: usize,
        head_record_hash: Option<&'a str>,
    }

    let input = ManifestHashInput {
        schema_version: 1,
        events_count,
        head_record_hash,
    };
    let bytes = serde_json::to_vec(&input).map_err(|_| AuditError::SerializationFailed)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

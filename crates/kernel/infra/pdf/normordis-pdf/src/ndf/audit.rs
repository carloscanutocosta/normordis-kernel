use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::NormaxisPdfError;

/// Append-only audit chain for an NDF document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdfAudit {
    /// Unique, immutable document identifier.
    pub document_id: String,
    /// Append-only event list.
    pub events: Vec<AuditEvent>,
}

impl NdfAudit {
    pub fn next_seq(&self) -> u32 {
        self.events.len() as u32 + 1
    }

    /// Appends an event, enforcing seq monotonicity and timestamp monotonicity.
    pub fn append(&mut self, mut event: AuditEvent) -> crate::Result<()> {
        let expected = self.next_seq();
        if event.seq == 0 {
            event.seq = expected;
        } else if event.seq != expected {
            return Err(NormaxisPdfError::NdfAuditError(format!(
                "expected seq {expected}, got {}",
                event.seq
            )));
        }
        if let Some(last) = self.events.last() {
            if event.timestamp < last.timestamp {
                return Err(NormaxisPdfError::NdfAuditError(format!(
                    "non-monotonic timestamp at seq {} ({} < {})",
                    event.seq, event.timestamp, last.timestamp
                )));
            }
        }
        self.events.push(event);
        Ok(())
    }
}

/// A single event in the audit chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Strictly increasing sequence number starting at 1.
    pub seq: u32,
    /// Event type, namespaced by domain.
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// ISO 8601 UTC timestamp.
    pub timestamp: String,
    /// Actor responsible for this event.
    pub actor: Actor,
    /// content_hash at the time of this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Free-text note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Additional event-specific fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

impl AuditEvent {
    /// Returns true for documentary events that require a matching content_hash.
    pub fn is_documentary(&self) -> bool {
        matches!(
            self.event_type,
            EventType::DocumentGenerated
                | EventType::DocumentReviewed
                | EventType::DocumentApproved
                | EventType::DocumentRejected
                | EventType::SignaturePdfApplied
                | EventType::SignatureNdfApplied
        )
    }
}

/// Event types, namespaced by domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "document.generated")]
    DocumentGenerated,
    #[serde(rename = "document.reviewed")]
    DocumentReviewed,
    #[serde(rename = "document.approved")]
    DocumentApproved,
    #[serde(rename = "document.rejected")]
    DocumentRejected,
    #[serde(rename = "document.superseded")]
    DocumentSuperseded,
    #[serde(rename = "render.pdf.generated")]
    RenderPdfGenerated,
    #[serde(rename = "signature.pdf.applied")]
    SignaturePdfApplied,
    #[serde(rename = "signature.ndf.applied")]
    SignatureNdfApplied,
    #[serde(rename = "archive.stored")]
    ArchiveStored,
    #[serde(rename = "publication.sent")]
    PublicationSent,
}

/// Actor responsible for an audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor {
    System {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        instance_id: Option<String>,
    },
    User {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        entity: Option<String>,
    },
    Batch {
        job_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger: Option<String>,
    },
}

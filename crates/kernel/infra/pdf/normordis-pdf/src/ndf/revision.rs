use serde_json::Value;

use super::{
    audit::{Actor, AuditEvent, EventType, NdfAudit},
    integrity::NdfIntegrity,
    NdfDocument, NdfRevisionRef, NDF_VERSION,
};
use crate::NormaxisPdfError;

/// Creates a revised NDF from an existing one.
///
/// Never modifies the original. Returns a new `NdfDocument` with:
/// - `revision.revision_of` = original `document_id`
/// - `revision.revision_seq` = original_seq + 1 (minimum 2)
/// - New content and recomputed integrity hashes
/// - A fresh audit chain with a single `document.generated` event
pub struct NdfRevision;

impl NdfRevision {
    pub fn create_from(
        original: &NdfDocument,
        new_content: Value,
        actor: Actor,
        reason: &str,
        document_id: Option<String>,
    ) -> crate::Result<NdfDocument> {
        let revision_seq = original
            .revision
            .as_ref()
            .map(|r| r.revision_seq + 1)
            .unwrap_or(2);

        if revision_seq < 2 {
            return Err(NormaxisPdfError::NdfRevisionError(
                "revision_seq must be >= 2 — original is implicitly seq 1".into(),
            ));
        }

        let meta_val = serde_json::to_value(&original.meta)
            .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;
        let integrity =
            NdfIntegrity::compute(&new_content, &original.styles, &meta_val)?;

        let now = chrono::Utc::now().to_rfc3339();
        let doc_id = document_id.unwrap_or_else(|| {
            format!("{}-rev{}", original.audit.document_id, revision_seq)
        });

        let first_event = AuditEvent {
            seq: 1,
            event_type: EventType::DocumentGenerated,
            timestamp: now.clone(),
            actor,
            content_hash: Some(integrity.content_hash.clone()),
            note: Some(reason.to_string()),
            extra: Default::default(),
        };

        Ok(NdfDocument {
            ndf: NDF_VERSION.into(),
            origin: original.origin.clone(),
            revision: Some(NdfRevisionRef {
                revision_of: original.audit.document_id.clone(),
                revision_reason: reason.to_string(),
                revision_seq,
            }),
            meta: original.meta.clone(),
            output: original.output.clone(),
            styles: original.styles.clone(),
            content: new_content,
            integrity,
            audit: NdfAudit {
                document_id: doc_id,
                events: vec![first_event],
            },
            outputs: vec![],
            signatures: vec![],
        })
    }
}

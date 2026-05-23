use sha2::{Digest, Sha256};
use support_storage::StorageKey;

use crate::error::AuditError;
use crate::event::AuditEvent;

pub fn audit_event_key(event: &AuditEvent) -> Result<StorageKey, AuditError> {
    let epoch_ms = event.occurred_at_utc.timestamp_millis();
    StorageKey::new(format!("{epoch_ms}.{}", event.event_id))
        .map_err(|_| AuditError::OperationFailed)
}

pub(crate) fn audit_event_lookup_key(event_id: &str) -> Result<StorageKey, AuditError> {
    if event_id.trim().is_empty() || event_id != event_id.trim() {
        return Err(AuditError::OperationFailed);
    }
    StorageKey::new(format!("by-id.{event_id}")).map_err(|_| AuditError::OperationFailed)
}

pub(crate) fn audit_actor_index_key(actor_id: &str) -> Result<StorageKey, AuditError> {
    Ok(
        StorageKey::new(format!("by-actor.{}", hash_fragment(actor_id)))
            .map_err(|_| AuditError::OperationFailed)?,
    )
}

pub(crate) fn audit_target_index_key(
    target_type: &str,
    target_id: &str,
) -> Result<StorageKey, AuditError> {
    Ok(StorageKey::new(format!(
        "by-target.{}",
        hash_fragment(&format!("{target_type}\n{target_id}"))
    ))
    .map_err(|_| AuditError::OperationFailed)?)
}

fn hash_fragment(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

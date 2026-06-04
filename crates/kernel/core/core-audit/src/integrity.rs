use sha2::{Digest, Sha256};

use crate::error::AuditError;
use crate::event::AuditEvent;

pub fn event_hash(event: &AuditEvent) -> Result<String, AuditError> {
    let bytes = serde_json::to_vec(event).map_err(|_| AuditError::SerializationFailed)?;
    let digest = Sha256::digest(bytes);
    Ok(hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::{AuditActor, AuditOutcome, AuditTarget};

    #[test]
    fn event_hash_is_stable_for_same_event() {
        let event = AuditEvent::with_id_and_time(
            "event-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();

        assert_eq!(event_hash(&event).unwrap(), event_hash(&event).unwrap());
    }
}

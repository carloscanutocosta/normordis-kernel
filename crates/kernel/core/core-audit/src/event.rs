use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::actor::AuditActor;
use crate::error::AuditError;
use crate::policy::validate_event_policy;
use crate::target::AuditTarget;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub actor: AuditActor,
    pub target: AuditTarget,
    pub occurred_at_utc: DateTime<Utc>,
    pub details_json: Option<Value>,
}

impl AuditEvent {
    pub fn new(
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        details_json: Option<Value>,
    ) -> Result<Self, AuditError> {
        Self::with_id_and_time(
            Uuid::new_v4().to_string(),
            event_type,
            actor,
            target,
            Utc::now(),
            details_json,
        )
    }

    pub fn with_id_and_time(
        event_id: impl Into<String>,
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        occurred_at_utc: DateTime<Utc>,
        details_json: Option<Value>,
    ) -> Result<Self, AuditError> {
        let event = Self {
            event_id: event_id.into(),
            event_type: event_type.into(),
            actor,
            target,
            occurred_at_utc,
            details_json,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<(), AuditError> {
        if self.event_id.trim().is_empty() || self.event_id != self.event_id.trim() {
            return Err(AuditError::OperationFailed);
        }
        if self.event_type.trim().is_empty() || self.event_type != self.event_type.trim() {
            return Err(AuditError::InvalidEventType);
        }
        self.actor.validate()?;
        self.target.validate()?;
        validate_event_policy(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn creates_valid_event() {
        let event = AuditEvent::with_id_and_time(
            "event-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(event.event_id, "event-1");
        assert_eq!(event.event_type, "document.created");
    }

    #[test]
    fn rejects_empty_event_type() {
        let err = AuditEvent::new(
            "",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            None,
        )
        .unwrap_err();

        assert_eq!(err, AuditError::InvalidEventType);
    }
}

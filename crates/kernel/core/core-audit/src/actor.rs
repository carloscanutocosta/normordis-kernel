use serde::{Deserialize, Serialize};

use crate::error::AuditError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditActor {
    pub actor_id: String,
    pub actor_name: Option<String>,
    pub actor_type: Option<String>,
}

impl AuditActor {
    pub fn new(actor_id: impl Into<String>) -> Result<Self, AuditError> {
        Self::with_metadata(actor_id, None, None)
    }

    pub fn with_metadata(
        actor_id: impl Into<String>,
        actor_name: Option<String>,
        actor_type: Option<String>,
    ) -> Result<Self, AuditError> {
        let actor_id = actor_id.into();
        if actor_id.trim().is_empty() || actor_id != actor_id.trim() {
            return Err(AuditError::InvalidActor);
        }

        Ok(Self {
            actor_id,
            actor_name,
            actor_type,
        })
    }

    pub fn validate(&self) -> Result<(), AuditError> {
        if self.actor_id.trim().is_empty() || self.actor_id != self.actor_id.trim() {
            return Err(AuditError::InvalidActor);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_actor() {
        let actor = AuditActor::with_metadata(
            "user-1",
            Some("User One".to_string()),
            Some("local-user".to_string()),
        )
        .unwrap();

        assert_eq!(actor.actor_id, "user-1");
        assert_eq!(actor.actor_name.as_deref(), Some("User One"));
        assert_eq!(actor.actor_type.as_deref(), Some("local-user"));
    }

    #[test]
    fn rejects_empty_actor() {
        assert_eq!(AuditActor::new("").unwrap_err(), AuditError::InvalidActor);
        assert_eq!(
            AuditActor::new(" user-1").unwrap_err(),
            AuditError::InvalidActor
        );
    }
}

use serde_json::Value;

use crate::error::AuditError;
use crate::event::AuditEvent;

pub const DEFAULT_MAX_EVENT_TYPE_CHARS: usize = 128;
pub const DEFAULT_MAX_ACTOR_FIELD_CHARS: usize = 256;
pub const DEFAULT_MAX_TARGET_FIELD_CHARS: usize = 256;
pub const DEFAULT_MAX_DETAILS_BYTES: usize = 16 * 1024;

const SENSITIVE_KEYS: &[&str] = &[
    "authorization",
    "ciphertext",
    "cookie",
    "key",
    "passphrase",
    "password",
    "payload",
    "plaintext",
    "recovery",
    "secret",
    "token",
];

pub fn validate_event_policy(event: &AuditEvent) -> Result<(), AuditError> {
    if event.event_type.chars().count() > DEFAULT_MAX_EVENT_TYPE_CHARS {
        return Err(AuditError::InvalidEventType);
    }
    validate_actor_field(&event.actor.actor_id)?;
    validate_optional_actor_field(event.actor.actor_name.as_deref())?;
    validate_optional_actor_field(event.actor.actor_type.as_deref())?;
    validate_target_field(&event.target.target_type)?;
    validate_target_field(&event.target.target_id)?;

    if let Some(details) = &event.details_json {
        validate_details(details)?;
    }

    Ok(())
}

fn validate_actor_field(value: &str) -> Result<(), AuditError> {
    if value.chars().count() > DEFAULT_MAX_ACTOR_FIELD_CHARS {
        return Err(AuditError::InvalidActor);
    }
    Ok(())
}

fn validate_optional_actor_field(value: Option<&str>) -> Result<(), AuditError> {
    if let Some(value) = value {
        validate_actor_field(value)?;
    }
    Ok(())
}

fn validate_target_field(value: &str) -> Result<(), AuditError> {
    if value.chars().count() > DEFAULT_MAX_TARGET_FIELD_CHARS {
        return Err(AuditError::InvalidTarget);
    }
    Ok(())
}

fn validate_details(details: &Value) -> Result<(), AuditError> {
    let bytes = serde_json::to_vec(details).map_err(|_| AuditError::SerializationFailed)?;
    if bytes.len() > DEFAULT_MAX_DETAILS_BYTES {
        return Err(AuditError::DetailsTooLarge);
    }
    if contains_sensitive_key(details) {
        return Err(AuditError::SensitiveDetails);
    }
    Ok(())
}

fn contains_sensitive_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            SENSITIVE_KEYS
                .iter()
                .any(|sensitive| key.contains(sensitive))
                || contains_sensitive_key(value)
        }),
        Value::Array(values) => values.iter().any(contains_sensitive_key),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{AuditActor, AuditEvent, AuditTarget};

    #[test]
    fn rejects_sensitive_details() {
        let err = AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Some(json!({"password":"secret"})),
        )
        .unwrap_err();

        assert_eq!(err, AuditError::SensitiveDetails);
    }

    #[test]
    fn rejects_oversized_details() {
        let err = AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Some(json!({"note": "x".repeat(DEFAULT_MAX_DETAILS_BYTES)})),
        )
        .unwrap_err();

        assert_eq!(err, AuditError::DetailsTooLarge);
    }
}

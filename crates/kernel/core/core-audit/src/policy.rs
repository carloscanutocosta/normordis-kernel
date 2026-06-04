use serde_json::Value;

use crate::error::AuditError;
use crate::event::AuditEvent;

/// Comprimento máximo do `event_type`, em caracteres Unicode.
pub const DEFAULT_MAX_EVENT_TYPE_CHARS: usize = 128;

/// Comprimento máximo de campos do `AuditActor`, em caracteres Unicode.
pub const DEFAULT_MAX_ACTOR_FIELD_CHARS: usize = 256;

/// Comprimento máximo de campos do `AuditTarget`, em caracteres Unicode.
pub const DEFAULT_MAX_TARGET_FIELD_CHARS: usize = 256;

/// Comprimento máximo do `control_id`, em caracteres Unicode.
///
/// O `control_id` segue a convenção `CTRL-{CATEGORIA}-{NNN}` (ex.: `CTRL-AUTH-001`).
/// Deve ser compacto, estável e legível.
pub const DEFAULT_MAX_CONTROL_ID_CHARS: usize = 64;

/// Comprimento máximo do `name` de um [`ControlDefinition`], em caracteres Unicode.
///
/// [`ControlDefinition`]: crate::control_definition::ControlDefinition
pub const DEFAULT_MAX_CONTROL_NAME_CHARS: usize = 256;

/// Comprimento máximo de entradas em `implemented_by` e `references`
/// de um [`ControlDefinition`], em caracteres Unicode.
///
/// [`ControlDefinition`]: crate::control_definition::ControlDefinition
pub const DEFAULT_MAX_CONTROL_REFERENCE_CHARS: usize = 128;

/// Comprimento máximo do campo `notes` de um [`ControlExecution`], em caracteres Unicode.
///
/// [`ControlExecution`]: crate::control_execution::ControlExecution
pub const DEFAULT_MAX_CONTROL_NOTES_CHARS: usize = 1024;

/// Tamanho máximo do `details_json` serializado, em bytes.
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

/// Valida um [`AuditEvent`] segundo as políticas de aceitação do `core-audit`.
///
/// Esta função é chamada automaticamente pelos construtores de [`AuditEvent`] e
/// pelo método `record` do [`AuditStore`]. Não deve ser necessário chamá-la
/// directamente em código de aplicação.
///
/// # Regras aplicadas
///
/// | Campo        | Regra                                                             |
/// |--------------|-------------------------------------------------------------------|
/// | `event_type` | Não vazio; máx. [`DEFAULT_MAX_EVENT_TYPE_CHARS`] chars            |
/// | `actor.*`    | Campos não excedem [`DEFAULT_MAX_ACTOR_FIELD_CHARS`] chars        |
/// | `target.*`   | Campos não excedem [`DEFAULT_MAX_TARGET_FIELD_CHARS`] chars       |
/// | `control_id` | Se presente: não vazio, sem espaços nas extremidades, máx. [`DEFAULT_MAX_CONTROL_ID_CHARS`] chars |
/// | `details_json` | Se presente: máx. [`DEFAULT_MAX_DETAILS_BYTES`] bytes; sem chaves sensíveis |
///
/// [`AuditStore`]: crate::store::AuditStore
pub fn validate_event_policy(event: &AuditEvent) -> Result<(), AuditError> {
    if event.event_type.chars().count() > DEFAULT_MAX_EVENT_TYPE_CHARS {
        return Err(AuditError::InvalidEventType);
    }
    validate_actor_field(&event.actor.actor_id)?;
    validate_optional_actor_field(event.actor.actor_name.as_deref())?;
    validate_optional_actor_field(event.actor.actor_type.as_deref())?;
    validate_target_field(&event.target.target_type)?;
    validate_target_field(&event.target.target_id)?;

    if let Some(control_id) = &event.control_id {
        validate_control_id(control_id)?;
    }

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

/// Valida o `control_id` segundo as regras de formato do Registo de Controlos.
///
/// O `control_id` é uma referência externa — o `core-audit` não valida se o
/// controlo existe no registo, apenas que a referência tem um formato aceitável.
fn validate_control_id(control_id: &str) -> Result<(), AuditError> {
    if control_id.is_empty() || control_id != control_id.trim() {
        return Err(AuditError::InvalidControlId);
    }
    if control_id.chars().count() > DEFAULT_MAX_CONTROL_ID_CHARS {
        return Err(AuditError::InvalidControlId);
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
    use crate::{AuditActor, AuditEvent, AuditOutcome, AuditTarget};

    fn event_with_control(control_id: Option<&str>) -> Result<AuditEvent, AuditError> {
        AuditEvent::new(
            "document.approved",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::Success,
            control_id.map(str::to_string),
            None,
        )
    }

    #[test]
    fn rejects_sensitive_details() {
        let err = AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::Success,
            None,
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
            AuditOutcome::Success,
            None,
            Some(json!({"note": "x".repeat(DEFAULT_MAX_DETAILS_BYTES)})),
        )
        .unwrap_err();

        assert_eq!(err, AuditError::DetailsTooLarge);
    }

    #[test]
    fn accepts_valid_control_id() {
        assert!(event_with_control(Some("CTRL-014")).is_ok());
        assert!(event_with_control(Some("SOD-003")).is_ok());
        assert!(event_with_control(Some("COSO.CC6.1")).is_ok());
    }

    #[test]
    fn rejects_empty_control_id() {
        assert_eq!(
            event_with_control(Some("")).unwrap_err(),
            AuditError::InvalidControlId
        );
    }

    #[test]
    fn rejects_control_id_with_surrounding_whitespace() {
        assert_eq!(
            event_with_control(Some(" CTRL-001")).unwrap_err(),
            AuditError::InvalidControlId
        );
        assert_eq!(
            event_with_control(Some("CTRL-001 ")).unwrap_err(),
            AuditError::InvalidControlId
        );
    }

    #[test]
    fn rejects_oversized_control_id() {
        let long_id = "X".repeat(DEFAULT_MAX_CONTROL_ID_CHARS + 1);
        assert_eq!(
            event_with_control(Some(&long_id)).unwrap_err(),
            AuditError::InvalidControlId
        );
    }

    #[test]
    fn accepts_none_control_id() {
        assert!(event_with_control(None).is_ok());
    }
}

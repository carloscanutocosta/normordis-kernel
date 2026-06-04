use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é um UUID em formato canónico (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
///
/// Aceita qualquer versão de UUID válida (v1, v3, v4, v5, v7, nil) —
/// a validação é de formato, não de versão.
/// Para validar especificamente UUID v4, verificar a versão após parse.
pub fn validate_uuid(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    if uuid::Uuid::parse_str(value).is_ok() {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::UUID_FORMAT,
            field,
            "uuid format is invalid",
        ))
    }
}

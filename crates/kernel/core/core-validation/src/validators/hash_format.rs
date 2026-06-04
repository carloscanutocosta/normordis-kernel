use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é um hash SHA-256 em formato hexadecimal lowercase.
///
/// Critérios: exactamente 64 caracteres, todos dígitos ou letras 'a'–'f'.
/// Não verifica se o hash corresponde a qualquer conteúdo — apenas valida
/// o formato formal da string.
pub fn validate_sha256_hex(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let valid = value.len() == 64 && value.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if valid {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::HASH_SHA256_FORMAT,
            field,
            "sha256 hash must be exactly 64 lowercase hex characters",
        ))
    }
}

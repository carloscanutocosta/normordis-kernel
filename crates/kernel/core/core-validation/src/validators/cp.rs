use crate::{rules, Normalized, ValidationIssue, ValidationReport};

/// Normaliza um código postal português.
///
/// Aceita espaço como separador alternativo ao hífen: `"1000 001"` → `"1000-001"`.
pub fn normalize_cp(value: &str) -> Normalized<String> {
    let normalized = support_normalization::normalize_whitespace(value).replace(' ', "-");
    Normalized::new(value, normalized)
}

/// Valida um código postal português no formato `DDDD-DDD`.
///
/// Estrutura: 4 dígitos, hífen, 3 dígitos. Total: 8 caracteres normalizados.
/// Aceita espaço como separador alternativo ao hífen.
/// Não valida se o código postal existe na base de dados dos CTT.
pub fn validate_cp(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_cp(value);

    if is_cp_format(&normalized.normalized) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::CP_FORMAT,
            field,
            "postal code must be in DDDD-DDD format",
        ))
    }
}

fn is_cp_format(s: &str) -> bool {
    let b = s.as_bytes();
    s.len() == 8
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[4] == b'-'
        && b[5..].iter().all(|c| c.is_ascii_digit())
}

use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é um MIME type estruturalmente válido no formato `type/subtype`.
///
/// Critérios estruturais:
/// - Exactamente um `/` como separador.
/// - `type` e `subtype` não-vazios.
/// - `type`: letras ASCII, dígitos e hífenes.
/// - `subtype`: letras ASCII, dígitos, hífenes, pontos e sinais `+`.
/// - Sem espaços.
///
/// Parâmetros (ex: `"; charset=utf-8"`) não são aceites — o chamador deve
/// extrair apenas o `type/subtype` antes de validar.
///
/// Exemplos válidos: `"application/pdf"`, `"text/plain"`, `"image/svg+xml"`,
/// `"application/vnd.openxmlformats-officedocument.wordprocessingml.document"`.
pub fn validate_mime(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    if is_mime_format(value) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::MIME_FORMAT,
            field,
            "mime type must be in type/subtype format without parameters or spaces",
        ))
    }
}

fn is_mime_format(s: &str) -> bool {
    match s.split_once('/') {
        None => false,
        Some((t, st)) => {
            !t.is_empty()
                && !st.is_empty()
                && !s.contains(' ')
                && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && st
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '+'))
        }
    }
}

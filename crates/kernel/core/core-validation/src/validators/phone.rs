use crate::{rules, Normalized, ValidationIssue, ValidationReport};

/// Normaliza um número de telefone português.
///
/// Remove espaços, hífenes e parêntesis. Retira o prefixo internacional
/// `+351` ou `00351` se presente, deixando apenas os 9 dígitos nacionais.
pub fn normalize_phone_pt(value: &str) -> Normalized<String> {
    let cleaned =
        support_normalization::normalize_whitespace(value).replace([' ', '-', '(', ')'], "");
    let digits = if let Some(s) = cleaned.strip_prefix("+351") {
        s.to_string()
    } else if let Some(s) = cleaned.strip_prefix("00351") {
        s.to_string()
    } else {
        cleaned
    };
    Normalized::new(value, digits)
}

/// Valida um número de telefone português (formato nacional).
///
/// Após normalização e remoção do prefixo internacional (+351 / 00351),
/// o número deve ter exactamente 9 dígitos com primeiro dígito válido:
/// - `2`: linha fixa geográfica
/// - `3`: serviços não-geográficos (custo partilhado)
/// - `7`: serviços de informação / taxa premium
/// - `8`: não-geográfico (gratuito, custo partilhado)
/// - `9`: móvel
///
/// Não valida a existência do número nem a sua atribuição actual.
pub fn validate_phone_pt(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_phone_pt(value);
    let s = &normalized.normalized;

    let valid = s.len() == 9
        && s.chars().all(|c| c.is_ascii_digit())
        && matches!(s.as_bytes()[0], b'2' | b'3' | b'7' | b'8' | b'9');

    if valid {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::PHONE_PT_FORMAT,
            field,
            "phone number must be 9 digits with valid prefix (2, 3, 7, 8 or 9) — PT format",
        ))
    }
}

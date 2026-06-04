use crate::{rules, Normalized, ValidationIssue, ValidationReport};

pub fn normalize_niss(value: &str) -> Normalized<String> {
    Normalized::new(
        value,
        support_normalization::normalize_whitespace(value).replace(' ', ""),
    )
}

/// Valida um NISS (Número de Identificação da Segurança Social) português.
///
/// Estrutura: 11 dígitos.
/// - Posições 1–9: número base (primeiro dígito = categoria).
/// - Posição 10: dígito auxiliar do controlo (deve ser 0).
/// - Posição 11: dígito de controlo principal.
///
/// Categorias válidas para o primeiro dígito:
/// 1 = trabalhador por conta de outrem
/// 2 = beneficiário de desemprego
/// 3 = outro beneficiário
/// 5 = entidade empregadora
/// 6 = instituição financeira
/// 7 = organismo público
/// 9 = outros
///
/// Algoritmo de controlo (pesos [29,23,19,17,13,11,7,5,3]):
/// S = Σ(dígito[i] × peso[i]) para i ∈ 0..8
/// controlo = 9 − ((S − 1) mod 9)   → resultado ∈ {1..=9}
/// A posição 11 deve igualar `controlo`; a posição 10 deve ser '0'.
pub fn validate_niss(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_niss(value);
    let s = &normalized.normalized;

    if s.len() != 11 || !s.chars().all(|c| c.is_ascii_digit()) {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NISS_FORMAT,
            field,
            "niss must be 11 digits",
        ));
    }

    let bytes = s.as_bytes();

    if !matches!(bytes[0], b'1' | b'2' | b'3' | b'5' | b'6' | b'7' | b'9') {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NISS_FORMAT,
            field,
            "niss first digit is not a valid category",
        ));
    }

    if bytes[9] != b'0' {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NISS_CHECKSUM,
            field,
            "niss auxiliary check digit (position 10) must be zero",
        ));
    }

    let weights: [u32; 9] = [29, 23, 19, 17, 13, 11, 7, 5, 3];
    let sum: u32 = bytes[..9]
        .iter()
        .zip(weights.iter())
        .map(|(b, w)| (b - b'0') as u32 * w)
        .sum();

    let expected_check = 9 - ((sum - 1) % 9);
    let actual_check = (bytes[10] - b'0') as u32;

    if expected_check != actual_check {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NISS_CHECKSUM,
            field,
            "niss checksum is invalid",
        ));
    }

    ValidationReport::ok()
}

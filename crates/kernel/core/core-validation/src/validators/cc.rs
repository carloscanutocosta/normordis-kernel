use crate::{rules, Normalized, ValidationIssue, ValidationReport};

pub fn normalize_cc(value: &str) -> Normalized<String> {
    Normalized::new(
        value,
        support_normalization::normalize_whitespace(value)
            .replace([' ', '-'], "")
            .to_ascii_uppercase(),
    )
}

/// Valida um número de documento do Cartão de Cidadão português.
///
/// Formato normalizado: 8 dígitos + 1 letra maiúscula + 1 dígito de controlo = 10 caracteres.
/// Exemplos aceites na entrada: "12345678 A 4", "12345678-A-4", "12345678A4".
///
/// Algoritmo de controlo (variante Luhn para alfanumérico):
/// - Valor de cada caracter: dígito → 0–9; letra → A=10, B=11, …, Z=35.
/// - Índices pares (0, 2, 4, 6, 8): multiplicar por 2 e reduzir por soma de algarismos.
/// - Índices ímpares (1, 3, 5, 7): usar valor directo.
/// - Controlo = (10 − (soma % 10)) % 10.
///
/// Fonte: especificação técnica do IRN para o documento de identificação nacional.
/// O conjunto de letras de série válidas em produção é gerido pelo IRN e pode mudar;
/// este validador aceita qualquer letra A–Z.
pub fn validate_cc(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_cc(value);
    let s = &normalized.normalized;

    if s.len() != 10 {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::CC_FORMAT,
            field,
            "cc must be 10 characters after normalization (8 digits, 1 letter, 1 check digit)",
        ));
    }

    let bytes = s.as_bytes();

    if !bytes[..8].iter().all(|b| b.is_ascii_digit()) {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::CC_FORMAT,
            field,
            "cc first 8 characters must be digits",
        ));
    }

    if !bytes[8].is_ascii_uppercase() {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::CC_FORMAT,
            field,
            "cc 9th character must be an uppercase letter (series indicator)",
        ));
    }

    if !bytes[9].is_ascii_digit() {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::CC_FORMAT,
            field,
            "cc 10th character must be the numeric check digit",
        ));
    }

    let char_value = |b: u8| -> u32 {
        if b.is_ascii_digit() {
            (b - b'0') as u32
        } else {
            (b - b'A' + 10) as u32
        }
    };

    let reduce = |mut v: u32| -> u32 {
        let mut s = 0;
        while v > 0 {
            s += v % 10;
            v /= 10;
        }
        s
    };

    let sum: u32 = bytes[..9]
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let val = char_value(b);
            if i % 2 == 0 {
                reduce(val * 2)
            } else {
                val
            }
        })
        .sum();

    let expected_check = ((10 - (sum % 10)) % 10) as u8;
    let actual_check = bytes[9] - b'0';

    if expected_check != actual_check {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::CC_CHECKSUM,
            field,
            "cc checksum is invalid",
        ));
    }

    ValidationReport::ok()
}

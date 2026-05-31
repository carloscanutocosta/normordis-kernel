use crate::{rules, Normalized, ValidationIssue, ValidationReport};

pub fn normalize_iban(value: &str) -> Normalized<String> {
    Normalized::new(
        value,
        support_normalization::normalize_whitespace(value)
            .replace(' ', "")
            .to_ascii_uppercase(),
    )
}

pub fn validate_iban(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_iban(value);

    if !has_minimum_iban_shape(&normalized.normalized)
        || !iban_mod97_is_valid(&normalized.normalized)
    {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::IBAN_FORMAT,
            field,
            "iban format is invalid",
        ));
    }

    ValidationReport::ok()
}

fn has_minimum_iban_shape(value: &str) -> bool {
    let len = value.len();
    (15..=34).contains(&len)
        && value[..2].chars().all(|ch| ch.is_ascii_uppercase())
        && value[2..4].chars().all(|ch| ch.is_ascii_digit())
        && value[4..].chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn iban_mod97_is_valid(value: &str) -> bool {
    let rearranged = value[4..].chars().chain(value[..4].chars());
    let mut remainder = 0_u32;

    for ch in rearranged {
        if ch.is_ascii_digit() {
            remainder = (remainder * 10 + ch.to_digit(10).unwrap()) % 97;
            continue;
        }

        if ch.is_ascii_uppercase() {
            let number = (ch as u32) - ('A' as u32) + 10;
            remainder = (remainder * 10 + (number / 10)) % 97;
            remainder = (remainder * 10 + (number % 10)) % 97;
            continue;
        }

        return false;
    }

    remainder == 1
}

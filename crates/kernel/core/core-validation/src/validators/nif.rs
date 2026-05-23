use crate::{rules, Normalized, ValidationIssue, ValidationReport};

pub fn normalize_nif(value: &str) -> Normalized<String> {
    Normalized::new(
        value,
        support_normalization::normalize_whitespace(value).replace(' ', ""),
    )
}

pub fn validate_nif(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let normalized = normalize_nif(value);

    if normalized.normalized.len() != 9
        || !normalized.normalized.chars().all(|ch| ch.is_ascii_digit())
    {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NIF_FORMAT,
            field,
            "nif format is invalid",
        ));
    }

    if nif_checksum_is_valid(&normalized.normalized) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::NIF_CHECKSUM,
            field,
            "nif checksum is invalid",
        ))
    }
}

fn nif_checksum_is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !matches!(
        bytes[0],
        b'1' | b'2' | b'3' | b'5' | b'6' | b'7' | b'8' | b'9'
    ) {
        return false;
    }

    let mut sum = 0_u32;
    for (index, byte) in bytes.iter().take(8).enumerate() {
        let digit = (byte - b'0') as u32;
        sum += digit * (9 - index as u32);
    }
    let check = 11 - (sum % 11);
    let expected = if check >= 10 { 0 } else { check };
    expected == (bytes[8] - b'0') as u32
}

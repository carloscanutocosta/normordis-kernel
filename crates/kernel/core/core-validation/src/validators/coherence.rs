use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `start` não é posterior a `end`, para datas no formato `YYYY-MM-DD`.
///
/// Valida formato e valores semânticos: mês [1–12], dia por mês com ano bissexto.
/// Formatos mistos (data + datetime) são rejeitados antes de comparar.
///
/// Para datetimes RFC 3339, usar `validate_datetime_range`.
pub fn validate_date_range(field: impl Into<String>, start: &str, end: &str) -> ValidationReport {
    let field = field.into();

    if !is_date_format(start) || !is_date_format(end) {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::DATE_FORMAT_INVALID,
            field,
            "date must be in YYYY-MM-DD format",
        ));
    }

    if start <= end {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::DATE_RANGE_INVALID,
            field,
            format!("start '{start}' must not be after end '{end}'"),
        ))
    }
}

/// Valida que `start` não é posterior a `end`, para datetimes RFC 3339.
///
/// Aceita strings RFC 3339 completas com offset (`Z`, `+HH:MM`, `-HH:MM`).
/// A comparação é feita em UTC — correcta para qualquer combinação de offsets.
/// Dois instantes com offsets diferentes que representam o mesmo momento são iguais.
/// Fractional seconds são aceites (`"2026-01-01T08:00:00.123Z"`).
pub fn validate_datetime_range(
    field: impl Into<String>,
    start: &str,
    end: &str,
) -> ValidationReport {
    let field = field.into();

    let parse = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok();

    let (Some(start_dt), Some(end_dt)) = (parse(start), parse(end)) else {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::DATE_FORMAT_INVALID,
            field,
            "datetime must be a valid RFC 3339 string (e.g. 2026-01-01T08:00:00Z)",
        ));
    };

    if start_dt <= end_dt {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::DATE_RANGE_INVALID,
            field,
            format!("start '{start}' must not be after end '{end}'"),
        ))
    }
}

/// Valida que a transição de estado de `from` para `to` é permitida.
///
/// `allowed` é a lista de pares (from, to) considerados válidos.
/// O chamador define o contrato de transição; este validador verifica-o.
/// Não modela a semântica de negócio — apenas confirma a conformidade formal.
pub fn validate_state_transition<'a>(
    field: impl Into<String>,
    from: &str,
    to: &str,
    allowed: &[(&'a str, &'a str)],
) -> ValidationReport {
    let field = field.into();
    if allowed.iter().any(|(f, t)| *f == from && *t == to) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::STATE_TRANSITION_INVALID,
            field,
            format!("transition '{from}' → '{to}' is not allowed"),
        ))
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn parse_two_digits(b: &[u8], offset: usize) -> u8 {
    (b[offset] - b'0') * 10 + (b[offset + 1] - b'0')
}

fn parse_four_digits(b: &[u8], offset: usize) -> u16 {
    (b[offset] - b'0') as u16 * 1000
        + (b[offset + 1] - b'0') as u16 * 100
        + (b[offset + 2] - b'0') as u16 * 10
        + (b[offset + 3] - b'0') as u16
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100))
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_date_format(s: &str) -> bool {
    let b = s.as_bytes();
    if s.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b[..4].iter().all(|c| c.is_ascii_digit())
        || !b[5..7].iter().all(|c| c.is_ascii_digit())
        || !b[8..10].iter().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    let year = parse_four_digits(b, 0);
    let month = parse_two_digits(b, 5);
    let day = parse_two_digits(b, 8);
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

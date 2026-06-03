use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `start` não é posterior a `end`, para datas no formato `YYYY-MM-DD`.
///
/// Valida primeiro o formato de ambas as strings — ambas devem ter exactamente
/// 10 caracteres com a estrutura `\d{4}-\d{2}-\d{2}`. Formatos mistos (ex: uma
/// data e uma datetime) são rejeitados com `DATE_FORMAT_INVALID` antes de comparar,
/// eliminando o risco de resultados incorrectos por comparação lexicográfica entre
/// strings de comprimentos distintos.
///
/// Para datetimes (`YYYY-MM-DDTHH:MM:SSZ`), usar `validate_datetime_range`.
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

/// Valida que `start` não é posterior a `end`, para datetimes ISO 8601.
///
/// Ambas as strings devem começar com `YYYY-MM-DDTHH:MM:SS` (pelo menos 19 chars,
/// separador `T` na posição 10). A comparação é lexicográfica — correcta para
/// datetimes com o mesmo offset.
///
/// Quando os offsets são detectavelmente distintos (ex: `+01:00` vs `Z`), emite
/// `DATETIME_OFFSET_MISMATCH` como **Warning** — a comparação é feita na mesma,
/// mas o resultado pode estar errado. `Z` e `+00:00` são normalizados para o mesmo
/// offset e não geram aviso. Para comparação cross-offset correcta, normalizar para
/// UTC antes de chamar este validator.
pub fn validate_datetime_range(
    field: impl Into<String>,
    start: &str,
    end: &str,
) -> ValidationReport {
    let field = field.into();
    let mut report = ValidationReport::ok();

    if !is_datetime_format(start) || !is_datetime_format(end) {
        report.push(ValidationIssue::error(
            rules::DATE_FORMAT_INVALID,
            field,
            "datetime must start with YYYY-MM-DDTHH:MM:SS",
        ));
        return report;
    }

    let start_offset = normalize_offset(&start[19..]);
    let end_offset = normalize_offset(&end[19..]);
    if start_offset != end_offset {
        report.push(ValidationIssue::warning(
            rules::DATETIME_OFFSET_MISMATCH,
            &field,
            format!(
                "start offset '{}' differs from end offset '{}' — \
                 lexicographic comparison may be incorrect; normalize to UTC before comparing",
                &start[19..],
                &end[19..]
            ),
        ));
    }

    if start > end {
        report.push(ValidationIssue::error(
            rules::DATE_RANGE_INVALID,
            field,
            format!("start '{start}' must not be after end '{end}'"),
        ));
    }

    report
}

fn normalize_offset(offset: &str) -> &str {
    if offset == "Z" {
        "+00:00"
    } else {
        offset
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
    let month = parse_two_digits(b, 5);
    let day = parse_two_digits(b, 8);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn is_datetime_format(s: &str) -> bool {
    let b = s.as_bytes();
    if s.len() < 19
        || !is_date_format(&s[..10])
        || b[10] != b'T'
        || !b[11..13].iter().all(|c| c.is_ascii_digit())
        || b[13] != b':'
        || !b[14..16].iter().all(|c| c.is_ascii_digit())
        || b[16] != b':'
        || !b[17..19].iter().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    let hour = parse_two_digits(b, 11);
    let minute = parse_two_digits(b, 14);
    let second = parse_two_digits(b, 17);
    hour <= 23 && minute <= 59 && second <= 59
}

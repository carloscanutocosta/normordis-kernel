use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` está no intervalo `[min, max]` (inclusive em ambos os extremos).
///
/// Rejeita valores não-finitos (NaN, infinito).
/// Os parâmetros `min` e `max` devem ser finitos com `min <= max`.
/// Se não forem, é um erro de programação: `debug_assert!` em debug, erro em release.
///
/// Para percentagens: `validate_in_range(field, value, 0.0, 100.0)`.
/// Para scores normalizados: `validate_in_range(field, value, 0.0, 1.0)`.
pub fn validate_in_range(
    field: impl Into<String>,
    value: f64,
    min: f64,
    max: f64,
) -> ValidationReport {
    let field = field.into();

    if !min.is_finite() || !max.is_finite() || min > max {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NUMERIC_RANGE_INVALID,
            field,
            "range bounds must be finite numbers with min <= max",
        ));
    }

    if !value.is_finite() {
        return ValidationReport::with_issue(ValidationIssue::error(
            rules::NUMERIC_RANGE_INVALID,
            field,
            "value must be a finite number",
        ));
    }

    if value < min || value > max {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::NUMERIC_RANGE_INVALID,
            field,
            format!("value {value} must be between {min} and {max} (inclusive)"),
        ))
    } else {
        ValidationReport::ok()
    }
}

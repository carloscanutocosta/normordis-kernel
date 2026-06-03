use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` está no intervalo `[min, max]` (inclusive em ambos os extremos).
///
/// Rejeita valores não-finitos (NaN, infinito) com `NUMERIC_RANGE_INVALID`.
/// Os parâmetros `min` e `max` devem ser finitos e satisfazer `min <= max` —
/// caso contrário o comportamento é indeterminado (use `debug_assert!` em chamadores).
///
/// Para percentagens: `validate_in_range(field, value, 0.0, 100.0)`.
/// Para scores normalizados: `validate_in_range(field, value, 0.0, 1.0)`.
pub fn validate_in_range(
    field: impl Into<String>,
    value: f64,
    min: f64,
    max: f64,
) -> ValidationReport {
    debug_assert!(min <= max, "validate_in_range: min must be <= max");

    let field = field.into();

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

use crate::{rules, ValidationIssue, ValidationReport};

pub fn required(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    if value.trim().is_empty() {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::STRING_REQUIRED,
            field,
            "field is required",
        ))
    } else {
        ValidationReport::ok()
    }
}

pub fn min_length(field: impl Into<String>, value: &str, min: usize) -> ValidationReport {
    let field = field.into();
    if value.chars().count() < min {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::STRING_MIN_LENGTH,
            field,
            format!("field must have at least {min} characters"),
        ))
    } else {
        ValidationReport::ok()
    }
}

pub fn max_length(field: impl Into<String>, value: &str, max: usize) -> ValidationReport {
    let field = field.into();
    if value.chars().count() > max {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::STRING_MAX_LENGTH,
            field,
            format!("field must have at most {max} characters"),
        ))
    } else {
        ValidationReport::ok()
    }
}

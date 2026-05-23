use crate::{rules, ValidationIssue, ValidationReport};

pub fn validate_email(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    let trimmed = value.trim();
    let has_outer_whitespace = trimmed.len() != value.len();
    let structurally_valid = !has_outer_whitespace
        && !trimmed.chars().any(char::is_whitespace)
        && support_normalization::is_valid_email(trimmed);

    if structurally_valid {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::EMAIL_FORMAT,
            field,
            "email format is invalid",
        ))
    }
}

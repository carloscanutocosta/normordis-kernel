use crate::{rules, ValidationIssue, ValidationReport};

pub fn validate_uuid(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    if uuid::Uuid::parse_str(value).is_ok() {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::UUID_FORMAT,
            field,
            "uuid format is invalid",
        ))
    }
}

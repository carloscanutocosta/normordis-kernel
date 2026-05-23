use crate::{rules, ValidationIssue, ValidationReport};
use serde_json::Value;

pub fn require_object(field: impl Into<String>, value: &Value) -> ValidationReport {
    let field = field.into();
    if value.is_object() {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::JSON_OBJECT,
            field,
            "json value must be an object",
        ))
    }
}

pub fn require_field(field: impl Into<String>, object: &Value, key: &str) -> ValidationReport {
    let field = field.into();
    if object.as_object().is_some_and(|map| map.contains_key(key)) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::JSON_REQUIRED_FIELD,
            field,
            format!("json object must contain field {key}"),
        ))
    }
}

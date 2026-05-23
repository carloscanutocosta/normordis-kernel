use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub rule_id: String,
    pub field: Option<String>,
    pub severity: ValidationSeverity,
    pub message: String,
}

impl ValidationIssue {
    pub fn new(
        rule_id: impl Into<String>,
        field: Option<String>,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            field,
            severity,
            message: message.into(),
        }
    }

    pub fn error(
        rule_id: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            rule_id,
            Some(field.into()),
            ValidationSeverity::Error,
            message,
        )
    }

    pub fn warning(
        rule_id: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            rule_id,
            Some(field.into()),
            ValidationSeverity::Warning,
            message,
        )
    }
}

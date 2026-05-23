use crate::{ValidationIssue, ValidationSeverity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }

    pub fn with_issue(issue: ValidationIssue) -> Self {
        let mut report = Self::ok();
        report.push(issue);
        report
    }

    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
        self.refresh_valid();
    }

    pub fn merge(&mut self, other: ValidationReport) {
        self.issues.extend(other.issues);
        self.refresh_valid();
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    fn refresh_valid(&mut self) {
        self.valid = !self
            .issues
            .iter()
            .any(|issue| issue.severity == ValidationSeverity::Error);
    }
}

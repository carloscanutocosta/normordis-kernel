use serde::{Deserialize, Serialize};

use crate::error::AuditError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTarget {
    pub target_type: String,
    pub target_id: String,
}

impl AuditTarget {
    pub fn new(
        target_type: impl Into<String>,
        target_id: impl Into<String>,
    ) -> Result<Self, AuditError> {
        let target_type = target_type.into();
        let target_id = target_id.into();
        if target_type.trim().is_empty()
            || target_type != target_type.trim()
            || target_id.trim().is_empty()
            || target_id != target_id.trim()
        {
            return Err(AuditError::InvalidTarget);
        }

        Ok(Self {
            target_type,
            target_id,
        })
    }

    pub fn validate(&self) -> Result<(), AuditError> {
        if self.target_type.trim().is_empty()
            || self.target_type != self.target_type.trim()
            || self.target_id.trim().is_empty()
            || self.target_id != self.target_id.trim()
        {
            return Err(AuditError::InvalidTarget);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_target() {
        let target = AuditTarget::new("document", "doc-1").unwrap();

        assert_eq!(target.target_type, "document");
        assert_eq!(target.target_id, "doc-1");
    }

    #[test]
    fn rejects_empty_target() {
        assert_eq!(
            AuditTarget::new("", "doc-1").unwrap_err(),
            AuditError::InvalidTarget
        );
        assert_eq!(
            AuditTarget::new("document", "").unwrap_err(),
            AuditError::InvalidTarget
        );
    }
}

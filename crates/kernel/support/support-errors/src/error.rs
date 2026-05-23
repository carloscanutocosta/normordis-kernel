use crate::{Component, ErrorCode, PublicError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("{component}: {code}: {message}")]
pub struct MiniError {
    pub code: ErrorCode,
    pub component: Component,
    pub message: String,
    pub details: Value,
}

impl MiniError {
    pub fn new(code: ErrorCode, component: Component, message: impl Into<String>) -> Self {
        Self {
            code,
            component,
            message: message.into(),
            details: Value::Null,
        }
    }

    pub fn with_details(
        code: ErrorCode,
        component: Component,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            code,
            component,
            message: message.into(),
            details,
        }
    }

    pub fn to_public(&self) -> PublicError {
        PublicError {
            code: self.code.as_str().to_owned(),
            message: self.message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_mini_error_to_public_error() {
        let err = MiniError::new(
            ErrorCode::new("MINI.SQLITE.OPEN_FAILED").unwrap(),
            Component::new("adapter-sqlite").unwrap(),
            "failed to open sqlite database",
        );

        let public = err.to_public();

        assert_eq!(public.code, "MINI.SQLITE.OPEN_FAILED");
        assert_eq!(public.message, "failed to open sqlite database");
    }

    #[test]
    fn public_error_does_not_include_details() {
        let err = MiniError::with_details(
            ErrorCode::new("MINI.SQLITE.OPEN_FAILED").unwrap(),
            Component::new("adapter-sqlite").unwrap(),
            "failed to open sqlite database",
            json!({ "path": "/private/database.sqlite" }),
        );

        let serialized = serde_json::to_value(err.to_public()).unwrap();

        assert_eq!(
            serialized,
            json!({
                "code": "MINI.SQLITE.OPEN_FAILED",
                "message": "failed to open sqlite database"
            })
        );
        assert!(serialized.get("details").is_none());
    }
}

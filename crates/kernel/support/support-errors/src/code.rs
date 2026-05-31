use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ErrorCode(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorCodeError {
    #[error("error code cannot be empty")]
    Empty,
    #[error("error code must start with MINI.")]
    MissingMiniPrefix,
    #[error("error code cannot contain spaces")]
    ContainsSpaces,
}

impl ErrorCode {
    /// Creates an `ErrorCode` from a compile-time `&'static str` without allocation overhead.
    /// In debug builds, panics if the value does not meet `ErrorCode` requirements.
    /// Only use with string literals whose validity is guaranteed by the caller.
    pub fn new_static(value: &'static str) -> Self {
        debug_assert!(!value.is_empty(), "error code cannot be empty");
        debug_assert!(value.starts_with("MINI."), "error code must start with MINI.");
        debug_assert!(
            !value.chars().any(char::is_whitespace),
            "error code cannot contain spaces"
        );
        Self(value.to_owned())
    }

    pub fn new(value: impl Into<String>) -> Result<Self, ErrorCodeError> {
        let value = value.into();

        if value.is_empty() {
            return Err(ErrorCodeError::Empty);
        }

        if !value.starts_with("MINI.") {
            return Err(ErrorCodeError::MissingMiniPrefix);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(ErrorCodeError::ContainsSpaces);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_code() {
        let code = ErrorCode::new("MINI.SQLITE.OPEN_FAILED").unwrap();

        assert_eq!(code.as_str(), "MINI.SQLITE.OPEN_FAILED");
    }

    #[test]
    fn rejects_empty_code() {
        let err = ErrorCode::new("").unwrap_err();

        assert_eq!(err, ErrorCodeError::Empty);
    }

    #[test]
    fn rejects_code_without_mini_prefix() {
        let err = ErrorCode::new("SQLITE.OPEN_FAILED").unwrap_err();

        assert_eq!(err, ErrorCodeError::MissingMiniPrefix);
    }

    #[test]
    fn rejects_code_with_spaces() {
        let err = ErrorCode::new("MINI.SQLITE.OPEN FAILED").unwrap_err();

        assert_eq!(err, ErrorCodeError::ContainsSpaces);
    }

    #[test]
    fn rejects_invalid_deserialized_code() {
        let err = serde_json::from_str::<ErrorCode>(r#""SQLITE.OPEN_FAILED""#).unwrap_err();

        assert!(err.to_string().contains("MINI."));
    }

    #[test]
    fn new_static_accepts_valid_code() {
        let code = ErrorCode::new_static("MINI.CONFIG.INVALID_APP");

        assert_eq!(code.as_str(), "MINI.CONFIG.INVALID_APP");
    }
}

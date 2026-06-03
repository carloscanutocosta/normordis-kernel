use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const VALIDATION_COMPONENT: &str = "core-validation";
pub const INVALID_INPUT: &str = "MINI.VALIDATION.INVALID_INPUT";
pub const INVALID_RULE: &str = "MINI.VALIDATION.INVALID_RULE";
pub const NORMALIZATION_FAILED: &str = "MINI.VALIDATION.NORMALIZATION_FAILED";
pub const JSON_FAILED: &str = "MINI.VALIDATION.JSON_FAILED";
pub const OPERATION_FAILED: &str = "MINI.VALIDATION.OPERATION_FAILED";
pub const FILE_NOT_FOUND: &str = "MINI.VALIDATION.FILE_NOT_FOUND";
pub const NOT_REGULAR_FILE: &str = "MINI.VALIDATION.NOT_REGULAR_FILE";
pub const FILE_READ_FAILED: &str = "MINI.VALIDATION.FILE_READ_FAILED";
pub const MANIFEST_FAILED: &str = "MINI.VALIDATION.MANIFEST_FAILED";
pub const HASH_FAILED: &str = "MINI.VALIDATION.HASH_FAILED";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("validation input invalid")]
    InvalidInput,
    #[error("validation rule invalid")]
    InvalidRule,
    #[error("validation normalization failed")]
    NormalizationFailed,
    #[error("validation json failed")]
    JsonFailed,
    #[error("validation operation failed")]
    OperationFailed,
    #[error("validation file not found")]
    FileNotFound,
    #[error("validation path is not a regular file")]
    NotRegularFile,
    #[error("validation file read failed")]
    FileReadFailed,
    #[error("validation manifest failed")]
    ManifestFailed,
    /// Reservado para falhas internas de hash (SHA-256 não falha em condições normais;
    /// este variant existe para adapters externos que possam delegar hashing a hardware
    /// ou serviços que podem falhar).
    #[error("validation hash failed")]
    HashFailed,
}

impl ValidationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput => INVALID_INPUT,
            Self::InvalidRule => INVALID_RULE,
            Self::NormalizationFailed => NORMALIZATION_FAILED,
            Self::JsonFailed => JSON_FAILED,
            Self::OperationFailed => OPERATION_FAILED,
            Self::FileNotFound => FILE_NOT_FOUND,
            Self::NotRegularFile => NOT_REGULAR_FILE,
            Self::FileReadFailed => FILE_READ_FAILED,
            Self::ManifestFailed => MANIFEST_FAILED,
            Self::HashFailed => HASH_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::InvalidInput => "validation input is invalid",
            Self::InvalidRule => "validation rule is invalid",
            Self::NormalizationFailed => "validation normalization failed",
            Self::JsonFailed => "validation json operation failed",
            Self::OperationFailed => "validation operation failed",
            Self::FileNotFound => "validation file was not found",
            Self::NotRegularFile => "validation path is not a regular file",
            Self::FileReadFailed => "validation file read failed",
            Self::ManifestFailed => "validation manifest operation failed",
            Self::HashFailed => "validation hash operation failed",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-validation error codes must be valid"),
            Component::new(VALIDATION_COMPONENT).expect("core-validation component must be valid"),
            self.public_message(),
        )
    }
}

impl From<ValidationError> for MiniError {
    fn from(value: ValidationError) -> Self {
        value.to_mini_error()
    }
}

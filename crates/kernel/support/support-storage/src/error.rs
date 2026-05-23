use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const STORAGE_COMPONENT: &str = "support-storage";
pub const INVALID_NAMESPACE: &str = "MINI.STORAGE.INVALID_NAMESPACE";
pub const INVALID_KEY: &str = "MINI.STORAGE.INVALID_KEY";
pub const SERIALIZATION_FAILED: &str = "MINI.STORAGE.SERIALIZATION_FAILED";
pub const DESERIALIZATION_FAILED: &str = "MINI.STORAGE.DESERIALIZATION_FAILED";
pub const PROTECT_FAILED: &str = "MINI.STORAGE.PROTECT_FAILED";
pub const UNPROTECT_FAILED: &str = "MINI.STORAGE.UNPROTECT_FAILED";
pub const BACKEND_FAILED: &str = "MINI.STORAGE.BACKEND_FAILED";
pub const OPERATION_FAILED: &str = "MINI.STORAGE.OPERATION_FAILED";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("storage namespace invalid")]
    InvalidNamespace,
    #[error("storage key invalid")]
    InvalidKey,
    #[error("storage serialization failed")]
    SerializationFailed,
    #[error("storage deserialization failed")]
    DeserializationFailed,
    #[error("storage protection failed")]
    ProtectFailed,
    #[error("storage unprotection failed")]
    UnprotectFailed,
    #[error("storage backend failed")]
    BackendFailed,
    #[error("storage operation failed")]
    OperationFailed,
}

impl StorageError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidNamespace => INVALID_NAMESPACE,
            Self::InvalidKey => INVALID_KEY,
            Self::SerializationFailed => SERIALIZATION_FAILED,
            Self::DeserializationFailed => DESERIALIZATION_FAILED,
            Self::ProtectFailed => PROTECT_FAILED,
            Self::UnprotectFailed => UNPROTECT_FAILED,
            Self::BackendFailed => BACKEND_FAILED,
            Self::OperationFailed => OPERATION_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::InvalidNamespace => "storage namespace is invalid",
            Self::InvalidKey => "storage key is invalid",
            Self::SerializationFailed => "failed to serialize storage value",
            Self::DeserializationFailed => "failed to deserialize storage value",
            Self::ProtectFailed => "failed to protect storage value",
            Self::UnprotectFailed => "failed to unprotect storage value",
            Self::BackendFailed => "storage backend failed",
            Self::OperationFailed => "storage operation failed",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("support-storage error codes must be valid"),
            Component::new(STORAGE_COMPONENT).expect("support-storage component must be valid"),
            self.public_message(),
        )
    }
}

impl From<StorageError> for MiniError {
    fn from(value: StorageError) -> Self {
        value.to_mini_error()
    }
}

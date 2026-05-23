use std::fmt;
use support_errors::{Component, ErrorCode, MiniError};

pub const RUNTIME_COMPONENT: &str = "runtime-bootstrap";
pub const INVALID_STORAGE_PROFILE: &str = "MINI.RUNTIME.INVALID_STORAGE_PROFILE";
pub const UNSUPPORTED_STORAGE_BACKEND: &str = "MINI.RUNTIME.UNSUPPORTED_STORAGE_BACKEND";
pub const RUNTIME_OPEN_FAILED: &str = "MINI.RUNTIME.RUNTIME_OPEN_FAILED";
pub const AUDIT_RUNTIME_FAILED: &str = "MINI.RUNTIME.AUDIT_RUNTIME_FAILED";
pub const LOGGING_RUNTIME_FAILED: &str = "MINI.RUNTIME.LOGGING_RUNTIME_FAILED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidStorageProfile,
    UnsupportedStorageBackend,
    RuntimeOpenFailed,
    AuditRuntimeFailed,
    LoggingRuntimeFailed,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.public_message())
    }
}

impl std::error::Error for RuntimeError {}

impl RuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidStorageProfile => INVALID_STORAGE_PROFILE,
            Self::UnsupportedStorageBackend => UNSUPPORTED_STORAGE_BACKEND,
            Self::RuntimeOpenFailed => RUNTIME_OPEN_FAILED,
            Self::AuditRuntimeFailed => AUDIT_RUNTIME_FAILED,
            Self::LoggingRuntimeFailed => LOGGING_RUNTIME_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::InvalidStorageProfile => "runtime storage profile is invalid",
            Self::UnsupportedStorageBackend => "runtime storage backend is not supported",
            Self::RuntimeOpenFailed => "runtime failed to open",
            Self::AuditRuntimeFailed => "audit runtime failed to open",
            Self::LoggingRuntimeFailed => "logging runtime failed to open",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("runtime error codes must be valid"),
            Component::new(RUNTIME_COMPONENT).expect("runtime component must be valid"),
            self.public_message(),
        )
    }
}

impl From<RuntimeError> for MiniError {
    fn from(value: RuntimeError) -> Self {
        value.to_mini_error()
    }
}

use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const LOGGING_COMPONENT: &str = "support-logging";
pub const CONFIG_INVALID: &str = "MINI.LOGGING.CONFIG_INVALID";
pub const WRITE_FAILED: &str = "MINI.LOGGING.WRITE_FAILED";
pub const ROTATION_FAILED: &str = "MINI.LOGGING.ROTATION_FAILED";
pub const RETENTION_FAILED: &str = "MINI.LOGGING.RETENTION_FAILED";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LogError {
    #[error("logging config invalid")]
    ConfigInvalid,
    #[error("logging write failed")]
    WriteFailed,
    #[error("logging rotation failed")]
    RotationFailed,
    #[error("logging retention failed")]
    RetentionFailed,
}

impl LogError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfigInvalid => CONFIG_INVALID,
            Self::WriteFailed => WRITE_FAILED,
            Self::RotationFailed => ROTATION_FAILED,
            Self::RetentionFailed => RETENTION_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::ConfigInvalid => "logging configuration is invalid",
            Self::WriteFailed => "failed to write technical log",
            Self::RotationFailed => "failed to rotate technical log",
            Self::RetentionFailed => "failed to apply technical log retention",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("support-logging error code must be valid"),
            Component::new(LOGGING_COMPONENT).expect("support-logging component must be valid"),
            self.public_message(),
        )
    }
}

impl From<LogError> for MiniError {
    fn from(value: LogError) -> Self {
        value.to_mini_error()
    }
}

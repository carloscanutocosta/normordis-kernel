use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "infra-backup";
pub const POLICY_INVALID: &str = "MINI.BACKUP.POLICY_INVALID";
pub const IO_FAILED: &str = "MINI.BACKUP.IO_FAILED";
pub const HEALTH_FAILED: &str = "MINI.BACKUP.HEALTH_FAILED";
pub const ARCHIVE_FAILED: &str = "MINI.BACKUP.ARCHIVE_FAILED";
pub const RETENTION_FAILED: &str = "MINI.BACKUP.RETENTION_FAILED";
pub const LOCK_HELD: &str = "MINI.BACKUP.LOCK_HELD";
pub const CONTROL_DB_FAILED: &str = "MINI.BACKUP.CONTROL_DB_FAILED";

#[derive(Debug, Error)]
pub enum BackupServiceError {
    #[error("backup policy invalid: {0}")]
    PolicyInvalid(String),
    #[error("backup io failed: {0}")]
    IoFailed(String),
    #[error("health check failed for {path}: {reason}")]
    HealthFailed { path: String, reason: String },
    #[error("backup archive failed: {0}")]
    ArchiveFailed(String),
    #[error("backup retention failed: {0}")]
    RetentionFailed(String),
    #[error("maintenance lock is held by another process (already ran today)")]
    LockHeld,
    #[error("control.db error: {0}")]
    ControlDbFailed(String),
}

impl BackupServiceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PolicyInvalid(_) => POLICY_INVALID,
            Self::IoFailed(_) => IO_FAILED,
            Self::HealthFailed { .. } => HEALTH_FAILED,
            Self::ArchiveFailed(_) => ARCHIVE_FAILED,
            Self::RetentionFailed(_) => RETENTION_FAILED,
            Self::LockHeld => LOCK_HELD,
            Self::ControlDbFailed(_) => CONTROL_DB_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::PolicyInvalid(_) => "backup policy configuration is invalid",
            Self::IoFailed(_) => "backup io operation failed",
            Self::HealthFailed { .. } => "database health check failed",
            Self::ArchiveFailed(_) => "backup archive operation failed",
            Self::RetentionFailed(_) => "backup retention cleanup failed",
            Self::LockHeld => "maintenance already ran today",
            Self::ControlDbFailed(_) => "maintenance control database error",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("infra-backup error code must be valid"),
            Component::new(COMPONENT).expect("infra-backup component must be valid"),
            self.public_message(),
        )
    }
}

impl From<BackupServiceError> for MiniError {
    fn from(value: BackupServiceError) -> Self {
        value.to_mini_error()
    }
}

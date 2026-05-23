use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "support-backup";
pub const COMPRESS_FAILED: &str = "MINI.BACKUP.COMPRESS_FAILED";
pub const DECOMPRESS_FAILED: &str = "MINI.BACKUP.DECOMPRESS_FAILED";
pub const ENCRYPT_FAILED: &str = "MINI.BACKUP.ENCRYPT_FAILED";
pub const DECRYPT_FAILED: &str = "MINI.BACKUP.DECRYPT_FAILED";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BackupError {
    #[error("backup compress failed")]
    CompressFailed,
    #[error("backup decompress failed")]
    DecompressFailed,
    #[error("backup encrypt failed")]
    EncryptFailed,
    #[error("backup decrypt failed")]
    DecryptFailed,
}

impl BackupError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CompressFailed => COMPRESS_FAILED,
            Self::DecompressFailed => DECOMPRESS_FAILED,
            Self::EncryptFailed => ENCRYPT_FAILED,
            Self::DecryptFailed => DECRYPT_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::CompressFailed => "failed to compress backup data",
            Self::DecompressFailed => "failed to decompress backup data",
            Self::EncryptFailed => "failed to encrypt backup archive",
            Self::DecryptFailed => "failed to decrypt backup archive",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("support-backup error code must be valid"),
            Component::new(COMPONENT).expect("support-backup component must be valid"),
            self.public_message(),
        )
    }
}

impl From<BackupError> for MiniError {
    fn from(value: BackupError) -> Self {
        value.to_mini_error()
    }
}

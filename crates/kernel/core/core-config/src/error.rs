use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const CONFIG_COMPONENT: &str = "core-config";
pub const INVALID_APP_PROFILE: &str = "MINI.CONFIG.INVALID_APP_PROFILE";
pub const INVALID_RUNTIME_PROFILE: &str = "MINI.CONFIG.INVALID_RUNTIME_PROFILE";
pub const INVALID_STORAGE_PROFILE: &str = "MINI.CONFIG.INVALID_STORAGE_PROFILE";
pub const DUPLICATE_STORAGE_PROFILE: &str = "MINI.CONFIG.DUPLICATE_STORAGE_PROFILE";
pub const MISSING_STORAGE_PROFILE: &str = "MINI.CONFIG.MISSING_STORAGE_PROFILE";
pub const INVALID_CRYPTO_PROFILE: &str = "MINI.CONFIG.INVALID_CRYPTO_PROFILE";
pub const INVALID_LOGGING_PROFILE: &str = "MINI.CONFIG.INVALID_LOGGING_PROFILE";
pub const INVALID_AUDIT_PROFILE: &str = "MINI.CONFIG.INVALID_AUDIT_PROFILE";
pub const INCONSISTENT_PROFILE: &str = "MINI.CONFIG.INCONSISTENT_PROFILE";
pub const MALFORMED_JSON: &str = "MINI.CONFIG.MALFORMED_JSON";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("app profile invalid: {reason}")]
    InvalidAppProfile { reason: String },
    #[error("runtime profile invalid: {reason}")]
    InvalidRuntimeProfile { reason: String },
    #[error("storage profile invalid: {reason}")]
    InvalidStorageProfile { reason: String },
    #[error("duplicate storage profile: {name}")]
    DuplicateStorageProfile { name: String },
    #[error("storage profile missing: {name}")]
    MissingStorageProfile { name: String },
    #[error("crypto profile invalid: {reason}")]
    InvalidCryptoProfile { reason: String },
    #[error("logging profile invalid: {reason}")]
    InvalidLoggingProfile { reason: String },
    #[error("audit profile invalid: {reason}")]
    InvalidAuditProfile { reason: String },
    #[error("profile inconsistent: {reason}")]
    InconsistentProfile { reason: String },
    #[error("profile json malformed: {reason}")]
    MalformedJson { reason: String },
}

impl ConfigError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidAppProfile { .. } => INVALID_APP_PROFILE,
            Self::InvalidRuntimeProfile { .. } => INVALID_RUNTIME_PROFILE,
            Self::InvalidStorageProfile { .. } => INVALID_STORAGE_PROFILE,
            Self::DuplicateStorageProfile { .. } => DUPLICATE_STORAGE_PROFILE,
            Self::MissingStorageProfile { .. } => MISSING_STORAGE_PROFILE,
            Self::InvalidCryptoProfile { .. } => INVALID_CRYPTO_PROFILE,
            Self::InvalidLoggingProfile { .. } => INVALID_LOGGING_PROFILE,
            Self::InvalidAuditProfile { .. } => INVALID_AUDIT_PROFILE,
            Self::InconsistentProfile { .. } => INCONSISTENT_PROFILE,
            Self::MalformedJson { .. } => MALFORMED_JSON,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::InvalidAppProfile { .. } => "app configuration profile is invalid",
            Self::InvalidRuntimeProfile { .. } => "runtime configuration profile is invalid",
            Self::InvalidStorageProfile { .. } => "storage configuration profile is invalid",
            Self::DuplicateStorageProfile { .. } => "storage configuration profile is duplicated",
            Self::MissingStorageProfile { .. } => "storage configuration profile is missing",
            Self::InvalidCryptoProfile { .. } => "crypto configuration profile is invalid",
            Self::InvalidLoggingProfile { .. } => "logging configuration profile is invalid",
            Self::InvalidAuditProfile { .. } => "audit configuration profile is invalid",
            Self::InconsistentProfile { .. } => "configuration profiles are inconsistent",
            Self::MalformedJson { .. } => "configuration profile json is malformed",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new_static(self.code()),
            Component::new_static(CONFIG_COMPONENT),
            self.public_message(),
        )
    }
}

impl From<ConfigError> for MiniError {
    fn from(value: ConfigError) -> Self {
        value.to_mini_error()
    }
}

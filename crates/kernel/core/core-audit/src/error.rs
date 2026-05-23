use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const AUDIT_COMPONENT: &str = "core-audit";
pub const INVALID_EVENT_TYPE: &str = "MINI.AUDIT.INVALID_EVENT_TYPE";
pub const INVALID_ACTOR: &str = "MINI.AUDIT.INVALID_ACTOR";
pub const INVALID_TARGET: &str = "MINI.AUDIT.INVALID_TARGET";
pub const DETAILS_TOO_LARGE: &str = "MINI.AUDIT.DETAILS_TOO_LARGE";
pub const SENSITIVE_DETAILS: &str = "MINI.AUDIT.SENSITIVE_DETAILS";
pub const DUPLICATE_EVENT: &str = "MINI.AUDIT.DUPLICATE_EVENT";
pub const INTEGRITY_FAILED: &str = "MINI.AUDIT.INTEGRITY_FAILED";
pub const CHAIN_VERIFICATION_FAILED: &str = "MINI.AUDIT.CHAIN_VERIFICATION_FAILED";
pub const SIGN_FAILED: &str = "MINI.AUDIT.SIGN_FAILED";
pub const SIGNATURE_VERIFICATION_FAILED: &str = "MINI.AUDIT.SIGNATURE_VERIFICATION_FAILED";
pub const SERIALIZATION_FAILED: &str = "MINI.AUDIT.SERIALIZATION_FAILED";
pub const DESERIALIZATION_FAILED: &str = "MINI.AUDIT.DESERIALIZATION_FAILED";
pub const STORE_FAILED: &str = "MINI.AUDIT.STORE_FAILED";
pub const OPERATION_FAILED: &str = "MINI.AUDIT.OPERATION_FAILED";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuditError {
    #[error("audit event type invalid")]
    InvalidEventType,
    #[error("audit actor invalid")]
    InvalidActor,
    #[error("audit target invalid")]
    InvalidTarget,
    #[error("audit details too large")]
    DetailsTooLarge,
    #[error("audit details contain sensitive keys")]
    SensitiveDetails,
    #[error("audit event already exists")]
    DuplicateEvent,
    #[error("audit event integrity check failed")]
    IntegrityFailed,
    #[error("audit chain verification failed")]
    ChainVerificationFailed,
    #[error("audit manifest signing failed")]
    SignFailed,
    #[error("audit manifest signature verification failed")]
    SignatureVerificationFailed,
    #[error("audit event serialization failed")]
    SerializationFailed,
    #[error("audit event deserialization failed")]
    DeserializationFailed,
    #[error("audit store failed")]
    StoreFailed,
    #[error("audit operation failed")]
    OperationFailed,
}

impl AuditError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidEventType => INVALID_EVENT_TYPE,
            Self::InvalidActor => INVALID_ACTOR,
            Self::InvalidTarget => INVALID_TARGET,
            Self::DetailsTooLarge => DETAILS_TOO_LARGE,
            Self::SensitiveDetails => SENSITIVE_DETAILS,
            Self::DuplicateEvent => DUPLICATE_EVENT,
            Self::IntegrityFailed => INTEGRITY_FAILED,
            Self::ChainVerificationFailed => CHAIN_VERIFICATION_FAILED,
            Self::SignFailed => SIGN_FAILED,
            Self::SignatureVerificationFailed => SIGNATURE_VERIFICATION_FAILED,
            Self::SerializationFailed => SERIALIZATION_FAILED,
            Self::DeserializationFailed => DESERIALIZATION_FAILED,
            Self::StoreFailed => STORE_FAILED,
            Self::OperationFailed => OPERATION_FAILED,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::InvalidEventType => "audit event type is invalid",
            Self::InvalidActor => "audit actor is invalid",
            Self::InvalidTarget => "audit target is invalid",
            Self::DetailsTooLarge => "audit details are too large",
            Self::SensitiveDetails => "audit details contain sensitive fields",
            Self::DuplicateEvent => "audit event already exists",
            Self::IntegrityFailed => "audit event integrity check failed",
            Self::ChainVerificationFailed => "audit chain verification failed",
            Self::SignFailed => "audit manifest signing failed",
            Self::SignatureVerificationFailed => "audit manifest signature verification failed",
            Self::SerializationFailed => "failed to serialize audit event",
            Self::DeserializationFailed => "failed to deserialize audit event",
            Self::StoreFailed => "audit store failed",
            Self::OperationFailed => "audit operation failed",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-audit error codes must be valid"),
            Component::new(AUDIT_COMPONENT).expect("core-audit component must be valid"),
            self.public_message(),
        )
    }
}

impl From<AuditError> for MiniError {
    fn from(value: AuditError) -> Self {
        value.to_mini_error()
    }
}

impl From<support_storage::StorageError> for AuditError {
    fn from(_: support_storage::StorageError) -> Self {
        Self::StoreFailed
    }
}

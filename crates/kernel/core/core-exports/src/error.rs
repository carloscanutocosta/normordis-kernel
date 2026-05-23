use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const EXPORTS_COMPONENT: &str = "core-exports";
pub const MISSING_FIELD: &str = "MINI.EXPORTS.MISSING_FIELD";
pub const INVALID_SNAPSHOT: &str = "MINI.EXPORTS.INVALID_SNAPSHOT";
pub const MARSHAL_FAILED: &str = "MINI.EXPORTS.MARSHAL_FAILED";
pub const INVALID_PACKAGE: &str = "MINI.EXPORTS.INVALID_PACKAGE";
pub const AUDIT_ERROR: &str = "MINI.EXPORTS.AUDIT_ERROR";
pub const MATERIALIZE_FAILED: &str = "MINI.EXPORTS.MATERIALIZE_FAILED";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExportError {
    #[error("campo obrigatório em falta: {field}")]
    MissingField { field: String },
    #[error("snapshot inválido: {message}")]
    InvalidSnapshot { message: String },
    #[error("falha a serializar: {0}")]
    MarshalFailed(String),
    #[error("pacote documental inválido: {0}")]
    InvalidPackage(String),
    #[error("erro de audit: {0}")]
    AuditError(String),
    #[error("falha ao materializar export: {0}")]
    MaterializeFailed(String),
}

impl ExportError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingField { .. } => MISSING_FIELD,
            Self::InvalidSnapshot { .. } => INVALID_SNAPSHOT,
            Self::MarshalFailed(_) => MARSHAL_FAILED,
            Self::InvalidPackage(_) => INVALID_PACKAGE,
            Self::AuditError(_) => AUDIT_ERROR,
            Self::MaterializeFailed(_) => MATERIALIZE_FAILED,
        }
    }

    pub fn public_message(&self) -> String {
        match self {
            Self::MissingField { field } => format!("campo obrigatório em falta: {field}"),
            Self::InvalidSnapshot { message } => format!("snapshot inválido: {message}"),
            Self::MarshalFailed(_) => "falha a serializar snapshot".to_string(),
            Self::InvalidPackage(_) => "pacote documental inválido".to_string(),
            Self::AuditError(_) => "erro ao gerar evento de audit".to_string(),
            Self::MaterializeFailed(_) => "falha ao materializar export".to_string(),
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-exports error codes must be valid"),
            Component::new(EXPORTS_COMPONENT).expect("core-exports component must be valid"),
            self.public_message(),
        )
    }
}

impl From<ExportError> for MiniError {
    fn from(value: ExportError) -> Self {
        value.to_mini_error()
    }
}

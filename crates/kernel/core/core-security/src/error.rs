//! Tipos de erro de domínio de `core-security` e mapeamento para `MiniError`.

use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "core-security";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecurityError {
    #[error("campo obrigatório vazio: {0}")]
    MissingField(String),
    #[error("política de segurança inválida: {0}")]
    InvalidPolicy(String),
    #[error("invariante zero-trust violada: {0}")]
    InvariantViolated(String),
    #[error("política não encontrada: {0}")]
    PolicyNotFound(String),
    #[error("delegação não encontrada: {0}")]
    DelegationNotFound(String),
    #[error("entidade já existe: {0}")]
    AlreadyExists(String),
    #[error("repositório indisponível: {0}")]
    RepoUnavailable(String),
    #[error("delegação inválida: {0}")]
    InvalidDelegation(String),
    #[error("operação falhou: {0}")]
    OperationFailed(String),
}

impl SecurityError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingField(_) => "MINI.SECURITY.MISSING_FIELD",
            Self::InvalidPolicy(_) => "MINI.SECURITY.INVALID_POLICY",
            Self::InvariantViolated(_) => "MINI.SECURITY.INVARIANT_VIOLATED",
            Self::PolicyNotFound(_) => "MINI.SECURITY.POLICY_NOT_FOUND",
            Self::DelegationNotFound(_) => "MINI.SECURITY.DELEGATION_NOT_FOUND",
            Self::AlreadyExists(_) => "MINI.SECURITY.ALREADY_EXISTS",
            Self::RepoUnavailable(_) => "MINI.SECURITY.REPO_UNAVAILABLE",
            Self::InvalidDelegation(_) => "MINI.SECURITY.INVALID_DELEGATION",
            Self::OperationFailed(_) => "MINI.SECURITY.OPERATION_FAILED",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-security error codes must be valid"),
            Component::new(COMPONENT).expect("core-security component must be valid"),
            self.to_string(),
        )
    }
}

impl From<SecurityError> for MiniError {
    fn from(e: SecurityError) -> Self {
        e.to_mini_error()
    }
}

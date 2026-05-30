//! Tipos de erro de domínio de `core-rh` e mapeamento para `MiniError`.

use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "core-rh";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RhError {
    #[error("invalid user id")]
    InvalidUserId,
    #[error("invalid role")]
    InvalidRole,
    #[error("invalid user profile")]
    InvalidProfile,
    #[error("invalid organization unit reference")]
    InvalidOrgRef,
    #[error("invalid session")]
    InvalidSession,
    #[error("role não encontrado no catálogo: {0}")]
    RoleNotFound(String),
    #[error("role inactivo: {0}")]
    RoleInactive(String),
    #[error("RH operation failed: {0}")]
    OperationFailed(String),
}

impl RhError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUserId => "MINI.RH.INVALID_USER_ID",
            Self::InvalidRole => "MINI.RH.INVALID_ROLE",
            Self::InvalidProfile => "MINI.RH.INVALID_PROFILE",
            Self::InvalidOrgRef => "MINI.RH.INVALID_ORG_REF",
            Self::InvalidSession => "MINI.RH.INVALID_SESSION",
            Self::RoleNotFound(_) => "MINI.RH.ROLE_NOT_FOUND",
            Self::RoleInactive(_) => "MINI.RH.ROLE_INACTIVE",
            Self::OperationFailed(_) => "MINI.RH.OPERATION_FAILED",
        }
    }

    pub fn public_message(&self) -> String {
        match self {
            Self::InvalidUserId => "invalid user id".to_owned(),
            Self::InvalidRole => "invalid role".to_owned(),
            Self::InvalidProfile => "invalid user profile".to_owned(),
            Self::InvalidOrgRef => "invalid organization unit reference".to_owned(),
            Self::InvalidSession => "invalid session".to_owned(),
            Self::RoleNotFound(id) => format!("role não encontrado: {id}"),
            Self::RoleInactive(id) => format!("role inactivo: {id}"),
            Self::OperationFailed(message) => message.clone(),
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-rh error codes must be valid"),
            Component::new(COMPONENT).expect("core-rh component must be valid"),
            self.public_message(),
        )
    }
}

impl From<RhError> for MiniError {
    fn from(value: RhError) -> Self {
        value.to_mini_error()
    }
}

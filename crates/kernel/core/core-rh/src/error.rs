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
    #[error("utilizador não encontrado: {0}")]
    UserNotFound(String),
    #[error("role não encontrado no catálogo: {0}")]
    RoleNotFound(String),
    #[error("role inactivo: {0}")]
    RoleInactive(String),
    #[error("afetação inválida: {0}")]
    InvalidAssignment(String),
    #[error("afetação não encontrada: {0}")]
    AssignmentNotFound(String),
    #[error("sobreposição temporal de afetações para a pessoa '{0}'")]
    AssignmentOverlap(String),
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
            Self::UserNotFound(_) => "MINI.RH.USER_NOT_FOUND",
            Self::RoleNotFound(_) => "MINI.RH.ROLE_NOT_FOUND",
            Self::RoleInactive(_) => "MINI.RH.ROLE_INACTIVE",
            Self::InvalidAssignment(_) => "MINI.RH.INVALID_ASSIGNMENT",
            Self::AssignmentNotFound(_) => "MINI.RH.ASSIGNMENT_NOT_FOUND",
            Self::AssignmentOverlap(_) => "MINI.RH.ASSIGNMENT_OVERLAP",
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
            Self::UserNotFound(id) => format!("utilizador não encontrado: {id}"),
            Self::RoleNotFound(id) => format!("role não encontrado: {id}"),
            Self::RoleInactive(id) => format!("role inactivo: {id}"),
            Self::InvalidAssignment(msg) => msg.clone(),
            Self::AssignmentNotFound(id) => format!("afetação não encontrada: {id}"),
            Self::AssignmentOverlap(person) => {
                format!("sobreposição temporal de afetações para a pessoa '{person}'")
            }
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

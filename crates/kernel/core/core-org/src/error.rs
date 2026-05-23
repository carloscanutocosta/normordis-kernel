//! Tipos de erro de domínio de `core-org` e mapeamento para `MiniError`.

use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "core-org";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrgError {
    #[error("nível hierárquico inválido: {0}")]
    InvalidLevel(String),
    #[error("intervalo temporal inválido: valid_from deve preceder valid_until")]
    InvalidTemporalRange,
    #[error("hierarquia circular detectada")]
    CircularHierarchy,
    #[error("nível da unidade inconsistente com o nível do pai")]
    InconsistentLevel,
    #[error("unidade orgânica extinta não pode receber novas posições")]
    ExtinctUnit,
    #[error("unidade orgânica não encontrada: {0}")]
    UnitNotFound(String),
    #[error("posição orgânica não encontrada: {0}")]
    PositionNotFound(String),
    #[error("instrumento jurídico não encontrado: {0}")]
    InstrumentNotFound(String),
    #[error("competência não encontrada: {0}")]
    CompetencyNotFound(String),
    #[error("delegação não encontrada: {0}")]
    DelegationNotFound(String),
    #[error("entidade já existe: {0}")]
    AlreadyExists(String),
    #[error("não é possível desactivar unidade com filhos activos")]
    CannotDeactivateWithActiveChildren,
    #[error("não é possível desactivar unidade com posições activas")]
    CannotDeactivateWithActivePositions,
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(String),
    #[error("operação falhou: {0}")]
    OperationFailed(String),
}

impl OrgError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidLevel(_) => "MINI.ORG.INVALID_LEVEL",
            Self::InvalidTemporalRange => "MINI.ORG.INVALID_TEMPORAL_RANGE",
            Self::CircularHierarchy => "MINI.ORG.CIRCULAR_HIERARCHY",
            Self::InconsistentLevel => "MINI.ORG.INCONSISTENT_LEVEL",
            Self::ExtinctUnit => "MINI.ORG.EXTINCT_UNIT",
            Self::UnitNotFound(_) => "MINI.ORG.UNIT_NOT_FOUND",
            Self::PositionNotFound(_) => "MINI.ORG.POSITION_NOT_FOUND",
            Self::InstrumentNotFound(_) => "MINI.ORG.INSTRUMENT_NOT_FOUND",
            Self::CompetencyNotFound(_) => "MINI.ORG.COMPETENCY_NOT_FOUND",
            Self::DelegationNotFound(_) => "MINI.ORG.DELEGATION_NOT_FOUND",
            Self::AlreadyExists(_) => "MINI.ORG.ALREADY_EXISTS",
            Self::CannotDeactivateWithActiveChildren => "MINI.ORG.CANNOT_DEACTIVATE_WITH_CHILDREN",
            Self::CannotDeactivateWithActivePositions => {
                "MINI.ORG.CANNOT_DEACTIVATE_WITH_POSITIONS"
            }
            Self::EmptyField(_) => "MINI.ORG.EMPTY_FIELD",
            Self::OperationFailed(_) => "MINI.ORG.OPERATION_FAILED",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-org error codes must be valid"),
            Component::new(COMPONENT).expect("core-org component must be valid"),
            self.to_string(),
        )
    }
}

impl From<OrgError> for MiniError {
    fn from(e: OrgError) -> Self {
        e.to_mini_error()
    }
}

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InteroperabilityError {
    #[error("campo obrigatorio vazio: {0}")]
    EmptyField(&'static str),
    #[error("pedido de export invalido: {0}")]
    InvalidExportRequest(String),
    #[error("export nao autorizado: {0}")]
    Unauthorized(String),
    #[error("falha ao materializar export: {0}")]
    MaterializationFailed(String),
}

pub type Result<T> = std::result::Result<T, InteroperabilityError>;

impl From<core_exports::ExportError> for InteroperabilityError {
    fn from(value: core_exports::ExportError) -> Self {
        Self::InvalidExportRequest(value.to_string())
    }
}

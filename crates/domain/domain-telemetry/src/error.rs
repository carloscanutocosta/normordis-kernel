use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TelemetryError {
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(&'static str),
    #[error("período inválido: {0}")]
    InvalidPeriod(String),
    #[error("erro de armazenamento: {0}")]
    Storage(String),
}

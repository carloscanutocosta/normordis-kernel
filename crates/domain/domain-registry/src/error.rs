use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(&'static str),
    #[error("app não encontrada: {0}")]
    AppNotFound(String),
    #[error("app já registada: {0}")]
    AppAlreadyRegistered(String),
    #[error("transição de estado inválida: {from} → {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("app em estado terminal, não pode transitar: {0}")]
    TerminalState(String),
    #[error("role não encontrado no catálogo: {0}")]
    RoleNotFound(String),
    #[error("role existe mas está inactivo: {0}")]
    RoleInactive(String),
    #[error("erro de armazenamento: {0}")]
    Storage(String),
}

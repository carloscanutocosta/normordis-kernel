use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MefError {
    #[error("código MEF não pode estar vazio")]
    EmptyCode,
    #[error("referência de diploma não pode estar vazia")]
    EmptyDiplomaRef,
    #[error("utilizador responsável pela alteração não pode estar vazio")]
    EmptyChangedBy,
    #[error("entrada MEF não encontrada para código '{0}'")]
    EntryNotFound(String),
    #[error("código MEF '{0}' não está marcado como utilizável")]
    NotUsable(String),
}

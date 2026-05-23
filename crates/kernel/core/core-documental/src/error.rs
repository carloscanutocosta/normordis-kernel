//! Erros de domínio de custódia documental.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DocumentalError {
    #[error("documento não encontrado: {0}")]
    DocumentNotFound(String),
    #[error("template não encontrado: {0}")]
    TemplateNotFound(String),
    #[error("registo NDF não encontrado: {0}")]
    NdfRecordNotFound(String),
    #[error("anexo não encontrado: {0}")]
    AttachmentNotFound(String),
    #[error("transição de estado inválida: {0} → {1}")]
    InvalidStatusTransition(String, String),
    #[error("documento já finalizado — não pode ser modificado")]
    DocumentFinalized,
    #[error("número já atribuído")]
    NumberAlreadyAssigned,
    #[error("número inválido: campo vazio")]
    EmptyDocumentNumber,
    #[error("número de documento obrigatório para finalização")]
    MissingDocumentNumber,
    #[error("contexto de autoridade em falta para finalização")]
    MissingAuthorityContext,
    #[error("template activo já existe para este tipo: {0}")]
    ActiveTemplateExists(String),
    #[error("template imutável após activação")]
    TemplateImmutable,
    #[error("template não pode ser activado: estado actual não é 'draft'")]
    TemplateNotActivatable,
    #[error("registo NDF write-once: já existe registo com id {0}")]
    NdfRecordAlreadyExists(String),
    #[error("hash do NDF não coincide com o registado")]
    NdfHashMismatch,
    #[error("hash do conteúdo não coincide com o registado")]
    ContentHashMismatch,
    #[error("cadeia de eventos quebrada: {0}")]
    EventChainBroken(String),
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(String),
    #[error("identificador inválido em {field}: {reason}")]
    InvalidIdentifier { field: String, reason: String },
    #[error("operação falhou: {0}")]
    OperationFailed(String),
    #[error("pacote documental inválido: {0}")]
    InvalidPackage(String),
}

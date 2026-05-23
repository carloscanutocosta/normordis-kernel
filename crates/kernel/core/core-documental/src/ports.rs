//! Ports de persistência do domínio documental (hexagonal architecture).
//!
//! Cada trait define o contrato que os adapters de infra devem implementar.
//! Nenhum port conhece SQLite, filesystem ou outro backend concreto.

use crate::{
    AttachmentId, DocumentAttachment, DocumentCustody, DocumentEvent, DocumentId, DocumentRelation,
    DocumentStatus, DocumentTemplate, DocumentalError, NdfRecord, NdfRecordId, TemplateId,
};

pub trait DocumentCustodyRepository {
    fn get(&self, id: &DocumentId) -> Result<Option<DocumentCustody>, DocumentalError>;
    fn create(&self, doc: &DocumentCustody) -> Result<(), DocumentalError>;
    fn update_status(&self, id: &DocumentId, status: DocumentStatus)
        -> Result<(), DocumentalError>;
    fn assign_number(&self, id: &DocumentId, number: &str) -> Result<(), DocumentalError>;
    fn update_payload(
        &self,
        id: &DocumentId,
        payload_json: &serde_json::Value,
    ) -> Result<(), DocumentalError>;
    fn set_authority(
        &self,
        id: &DocumentId,
        authority: &crate::AuthorityContext,
    ) -> Result<(), DocumentalError>;
    fn add_relation(&self, relation: &DocumentRelation) -> Result<(), DocumentalError>;
    fn list_relations(&self, id: &DocumentId) -> Result<Vec<DocumentRelation>, DocumentalError>;
}

pub trait TemplateRepository {
    fn get(&self, id: &TemplateId) -> Result<Option<DocumentTemplate>, DocumentalError>;
    fn get_active_for_type(
        &self,
        document_type: &str,
    ) -> Result<Option<DocumentTemplate>, DocumentalError>;
    fn list_versions_for_type(
        &self,
        document_type: &str,
    ) -> Result<Vec<DocumentTemplate>, DocumentalError>;
    /// Write-once para templates: não há update — versões novas criam novo registo.
    fn create_version(&self, template: &DocumentTemplate) -> Result<(), DocumentalError>;
    /// Activa um template `Draft`. Falha se o template não existir ou não for `Draft`.
    /// Antes de chamar, o chamador deve verificar `template.activate()`.
    fn activate(&self, id: &TemplateId) -> Result<(), DocumentalError>;
    fn deprecate(&self, id: &TemplateId) -> Result<(), DocumentalError>;
}

/// Archive write-once para registos NDF.
///
/// Implementações DEVEM garantir que `write_once` falha com
/// `NdfRecordAlreadyExists` se um registo com o mesmo id já existe.
pub trait NdfArchive {
    fn write_once(&self, record: &NdfRecord) -> Result<(), DocumentalError>;
    fn read(&self, id: &NdfRecordId) -> Result<Option<NdfRecord>, DocumentalError>;
    fn read_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<NdfRecord>, DocumentalError>;
    fn exists(&self, id: &NdfRecordId) -> Result<bool, DocumentalError>;
}

/// Log de eventos append-only.
///
/// Implementações DEVEM proibir UPDATE ou DELETE nos eventos persistidos.
pub trait DocumentEventLog {
    fn append(&self, event: &DocumentEvent) -> Result<(), DocumentalError>;
    fn read_chain(&self, document_id: &DocumentId) -> Result<Vec<DocumentEvent>, DocumentalError>;
    fn last_event(
        &self,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentEvent>, DocumentalError>;
}

/// Guarda de documentos binários — anexos e documentos entrados.
///
/// O conteúdo é endereçado pelo hash SHA-256 (content_hash = nome do ficheiro).
/// Implementações DEVEM verificar `sha256(content) == attachment.content_hash`
/// antes de persistir, devolvendo `ContentHashMismatch` se não coincidir.
///
/// A deduplicação de conteúdo (mesmo hash → mesma blob) é decisão de implementação;
/// o port garante apenas a semântica de store/retrieve por `AttachmentId`.
pub trait AttachmentStore {
    fn store(&self, attachment: &DocumentAttachment, content: &[u8])
        -> Result<(), DocumentalError>;
    fn retrieve_content(&self, id: &AttachmentId) -> Result<Option<Vec<u8>>, DocumentalError>;
    fn get_metadata(
        &self,
        id: &AttachmentId,
    ) -> Result<Option<DocumentAttachment>, DocumentalError>;
    fn list_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAttachment>, DocumentalError>;
    /// Remove o anexo. Se o conteúdo não tiver outras referências, apaga-o também.
    /// Retorna `true` se o conteúdo foi de facto eliminado do armazenamento.
    fn delete_if_unreferenced(&self, id: &AttachmentId) -> Result<bool, DocumentalError>;
}

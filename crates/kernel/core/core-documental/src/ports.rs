//! Ports de persistência do domínio documental (hexagonal architecture).
//!
//! Cada trait define o contrato que os adapters de infra devem implementar.
//! Nenhum port conhece SQLite, filesystem ou outro backend concreto.
//!
//! # Atomicidade
//!
//! Todos os métodos de escrita aceitam um `&DocumentEvent` que deve ser
//! persistido na mesma transacção que a operação principal. Os adapters
//! DEVEM garantir esta atomicidade — documento e evento nunca podem divergir.

use crate::{
    AttachmentId, DocumentAttachment, DocumentCustody, DocumentEvent, DocumentId, DocumentRelation,
    DocumentStatus, DocumentTemplate, DocumentTypeCode, DocumentalError, EventFilter, NdfRecord,
    NdfRecordId, TemplateId, ValidationCode,
};

/// Port de custódia documental — intake, lookup e transições de estado.
///
/// Todos os métodos de escrita são atómicos: o adapter persiste o documento
/// e o evento correspondente numa única transacção.
pub trait DocumentCustodyRepository {
    // ── Intake ────────────────────────────────────────────────────────────────

    /// Persiste o documento em custódia e o evento `CustodyAccepted` atomicamente.
    fn accept(&self, doc: &DocumentCustody, event: &DocumentEvent) -> Result<(), DocumentalError>;

    // ── Lookup ────────────────────────────────────────────────────────────────

    fn get(&self, id: &DocumentId) -> Result<Option<DocumentCustody>, DocumentalError>;

    /// Lookup por código de validação público (ex: leitura de QR code em fiscalização).
    fn lookup_by_validation_code(
        &self,
        code: &ValidationCode,
    ) -> Result<Option<DocumentCustody>, DocumentalError>;

    fn lookup_by_document_number(
        &self,
        number: &str,
    ) -> Result<Option<DocumentCustody>, DocumentalError>;

    fn list_by_type(
        &self,
        document_type: &DocumentTypeCode,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DocumentCustody>, DocumentalError>;

    fn list_by_status(
        &self,
        status: &DocumentStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DocumentCustody>, DocumentalError>;

    fn count_by_status(&self, status: &DocumentStatus) -> Result<u64, DocumentalError>;

    // ── Transições custodiais atómicas ────────────────────────────────────────

    /// Transição de estado + evento `StatusChanged` numa transacção.
    fn transition_status(
        &self,
        id: &DocumentId,
        status: DocumentStatus,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError>;

    /// Atribuição de número + evento `NumberAssigned` numa transacção.
    /// O adapter deve falhar com `NumberAlreadyAssigned` se já existir número.
    fn assign_number(
        &self,
        id: &DocumentId,
        number: &str,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError>;

    /// Substituição atómica: transição para `Superseded` + relação `Supersedes`.
    ///
    /// Persiste atomicamente: alteração de estado, relação inter-documental,
    /// evento `StatusChanged` e evento `RelationAdded`.
    fn supersede(
        &self,
        id: &DocumentId,
        relation: &DocumentRelation,
        status_event: &DocumentEvent,
        relation_event: &DocumentEvent,
    ) -> Result<(), DocumentalError>;

    // ── Relações ──────────────────────────────────────────────────────────────

    /// Regista relação + evento `RelationAdded` atomicamente.
    fn add_relation(
        &self,
        relation: &DocumentRelation,
        event: &DocumentEvent,
    ) -> Result<(), DocumentalError>;

    fn list_relations(&self, id: &DocumentId) -> Result<Vec<DocumentRelation>, DocumentalError>;
}

/// Port de templates NDT — write-once, sem activação (templates chegam já Active).
///
/// `store` falha com `ActiveTemplateExists` se já existir template Active
/// para o mesmo `document_type`.
pub trait TemplateRepository {
    fn get(&self, id: &TemplateId) -> Result<Option<DocumentTemplate>, DocumentalError>;
    fn get_active_for_type(
        &self,
        document_type: &DocumentTypeCode,
    ) -> Result<Option<DocumentTemplate>, DocumentalError>;
    fn list_versions_for_type(
        &self,
        document_type: &DocumentTypeCode,
    ) -> Result<Vec<DocumentTemplate>, DocumentalError>;
    /// Write-once — não há update de conteúdo.
    fn store(&self, template: &DocumentTemplate) -> Result<(), DocumentalError>;
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
/// `filter` permite pesquisa por tipo, intervalo de tempo e paginação.
pub trait DocumentEventLog {
    fn append(&self, event: &DocumentEvent) -> Result<(), DocumentalError>;
    fn read_chain(&self, document_id: &DocumentId) -> Result<Vec<DocumentEvent>, DocumentalError>;
    fn last_event(
        &self,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentEvent>, DocumentalError>;
    /// Pesquisa filtrada com paginação.
    fn filter(
        &self,
        document_id: &DocumentId,
        filter: &EventFilter,
    ) -> Result<Vec<DocumentEvent>, DocumentalError>;
}

/// Guarda de documentos binários — anexos e documentos entrados.
///
/// O conteúdo é endereçado pelo hash SHA-256 (content_hash = nome do ficheiro).
/// Implementações DEVEM verificar `sha256(content) == attachment.content_hash`
/// antes de persistir, devolvendo `ContentHashMismatch` se não coincidir.
pub trait AttachmentStore {
    fn store(
        &self,
        attachment: &DocumentAttachment,
        content: &[u8],
    ) -> Result<(), DocumentalError>;
    fn retrieve_content(&self, id: &AttachmentId) -> Result<Option<Vec<u8>>, DocumentalError>;
    fn get_metadata(
        &self,
        id: &AttachmentId,
    ) -> Result<Option<DocumentAttachment>, DocumentalError>;
    fn list_for_document(
        &self,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAttachment>, DocumentalError>;
    fn delete_if_unreferenced(&self, id: &AttachmentId) -> Result<bool, DocumentalError>;
}

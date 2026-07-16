//! Serviço de aplicação de custódia documental.
//!
//! `DocumentCustodyService` orquestra as operações de custódia garantindo que
//! cada escrita é atómica: documento e evento de auditoria são persistidos pelo
//! adapter numa única transacção.
//!
//! # Responsabilidades
//! - Validar pré-condições de domínio antes de chamar os ports
//! - Construir os eventos de auditoria correctos para cada operação
//! - Garantir que a cadeia de custódia (event chain) é respeitada
//!
//! # Não é responsabilidade deste serviço
//! - Gerar IDs (passados pelo chamador)
//! - Gerar hashes criptográficos
//! - Computar `previous_hash` (o chamador conhece o estado da cadeia)

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    ports::{DocumentCustodyRepository, DocumentEventLog},
    AccessPurpose, AuthoritySnapshot, AuthoritySource, DocumentCustody, DocumentEvent,
    DocumentEventId, DocumentEventType, DocumentId, DocumentRelation, DocumentStatus,
    DocumentalError, EventActor, RelationType,
};

/// Serviço de aplicação para custódia documental.
///
/// Genérico sobre `R: DocumentCustodyRepository` e `L: DocumentEventLog`.
/// Os adapters concretos são injectados em runtime — o serviço não conhece SQLite.
pub struct DocumentCustodyService<R, L>
where
    R: DocumentCustodyRepository,
    L: DocumentEventLog,
{
    repository: R,
    event_log: L,
}

impl<R, L> DocumentCustodyService<R, L>
where
    R: DocumentCustodyRepository,
    L: DocumentEventLog,
{
    pub fn new(repository: R, event_log: L) -> Self {
        Self {
            repository,
            event_log,
        }
    }

    // ── Intake ────────────────────────────────────────────────────────────────

    /// Intake atómico: documento + evento `CustodyAccepted` numa transacção.
    ///
    /// `occurred_at` deve ser o instante de custódia formal (normalmente `Utc::now()`).
    /// `previous_hash` é sempre `None` para o primeiro evento de um documento.
    pub fn accept_into_custody(
        &self,
        doc: DocumentCustody,
        event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), DocumentalError> {
        doc.validate()?;
        let actor = match &doc.authority {
            AuthoritySource::User(snapshot) => EventActor::Authority(snapshot.clone()),
            AuthoritySource::System {
                system_id,
                justification,
            } => EventActor::System {
                system_id: system_id.clone(),
                justification: justification.clone(),
            },
        };
        let event = DocumentEvent {
            id: event_id,
            document_id: doc.id.clone(),
            event_type: DocumentEventType::CustodyAccepted,
            actor,
            occurred_at,
            previous_hash: None,
            data_json: Some(json!({
                "origin": doc.origin.as_str(),
                "document_type": doc.document_type.as_str(),
                "validation_code": doc.validation_code.as_str(),
            })),
        };
        self.repository.accept(&doc, &event)
    }

    // ── Transições custodiais ─────────────────────────────────────────────────

    /// Arquivo atómico: `Active → Archived` + evento `StatusChanged`.
    pub fn archive(
        &self,
        id: &DocumentId,
        actor: AuthoritySnapshot,
        event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
        previous_hash: Option<String>,
    ) -> Result<(), DocumentalError> {
        let doc = self
            .repository
            .get(id)?
            .ok_or_else(|| DocumentalError::DocumentNotFound(id.as_str().into()))?;
        let next = doc.transition_to(DocumentStatus::Archived)?;
        let event = DocumentEvent {
            id: event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::StatusChanged,
            actor: EventActor::Authority(actor),
            occurred_at,
            previous_hash,
            data_json: Some(json!({
                "from": doc.status.as_str(),
                "to": next.as_str(),
            })),
        };
        self.repository.transition_status(id, next, &event)
    }

    /// Revogação atómica: `Active|Archived → Revoked` + evento `StatusChanged`.
    pub fn revoke(
        &self,
        id: &DocumentId,
        actor: AuthoritySnapshot,
        event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
        previous_hash: Option<String>,
    ) -> Result<(), DocumentalError> {
        let doc = self
            .repository
            .get(id)?
            .ok_or_else(|| DocumentalError::DocumentNotFound(id.as_str().into()))?;
        let next = doc.transition_to(DocumentStatus::Revoked)?;
        let event = DocumentEvent {
            id: event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::StatusChanged,
            actor: EventActor::Authority(actor),
            occurred_at,
            previous_hash,
            data_json: Some(json!({
                "from": doc.status.as_str(),
                "to": next.as_str(),
            })),
        };
        self.repository.transition_status(id, next, &event)
    }

    /// Substituição atómica: `Active → Superseded` + relação `Supersedes`.
    ///
    /// Persiste atomicamente via `DocumentCustodyRepository::supersede`:
    /// alteração de estado, relação inter-documental, e os dois eventos de auditoria.
    #[allow(clippy::too_many_arguments)]
    pub fn supersede(
        &self,
        id: &DocumentId,
        successor_id: DocumentId,
        actor: AuthoritySnapshot,
        status_event_id: DocumentEventId,
        relation_event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
        previous_hash: Option<String>,
    ) -> Result<(), DocumentalError> {
        let doc = self
            .repository
            .get(id)?
            .ok_or_else(|| DocumentalError::DocumentNotFound(id.as_str().into()))?;
        let next = doc.transition_to(DocumentStatus::Superseded)?;
        let relation = DocumentRelation {
            relation_type: RelationType::Supersedes,
            from_id: id.clone(),
            to_id: successor_id,
            established_at: occurred_at,
        };
        let status_event = DocumentEvent {
            id: status_event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::StatusChanged,
            actor: EventActor::Authority(actor.clone()),
            occurred_at,
            previous_hash: previous_hash.clone(),
            data_json: Some(json!({
                "from": doc.status.as_str(),
                "to": next.as_str(),
            })),
        };
        let relation_event = DocumentEvent {
            id: relation_event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::RelationAdded,
            actor: EventActor::Authority(actor),
            occurred_at,
            previous_hash,
            data_json: None,
        };
        self.repository
            .supersede(id, &relation, &status_event, &relation_event)
    }

    // ── Numeração ─────────────────────────────────────────────────────────────

    /// Atribuição de número atómica + evento `NumberAssigned`.
    pub fn assign_number(
        &self,
        id: &DocumentId,
        number: &str,
        actor: AuthoritySnapshot,
        event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
        previous_hash: Option<String>,
    ) -> Result<(), DocumentalError> {
        let doc = self
            .repository
            .get(id)?
            .ok_or_else(|| DocumentalError::DocumentNotFound(id.as_str().into()))?;
        doc.assign_number(number)?;
        let event = DocumentEvent {
            id: event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::NumberAssigned,
            actor: EventActor::Authority(actor),
            occurred_at,
            previous_hash,
            data_json: Some(json!({ "number": number })),
        };
        self.repository.assign_number(id, number, &event)
    }

    // ── Acesso com registo ─────────────────────────────────────────────────────

    /// Leitura com registo de acesso — regista evento `Accessed` antes de devolver o documento.
    ///
    /// O evento é persistido ANTES da leitura (intent-first). Se o registo falhar,
    /// devolve erro sem retornar o documento — garante que todo o acesso efectivo
    /// tem evento correspondente na cadeia.
    pub fn get_with_access_log(
        &self,
        id: &DocumentId,
        actor: AuthoritySnapshot,
        purpose: AccessPurpose,
        event_id: DocumentEventId,
        occurred_at: DateTime<Utc>,
        previous_hash: Option<String>,
    ) -> Result<Option<DocumentCustody>, DocumentalError> {
        let event = DocumentEvent {
            id: event_id,
            document_id: id.clone(),
            event_type: DocumentEventType::Accessed,
            actor: EventActor::Authority(actor),
            occurred_at,
            previous_hash,
            data_json: Some(json!({ "purpose": purpose.as_str() })),
        };
        self.event_log.append(&event)?;
        self.repository.get(id)
    }
}

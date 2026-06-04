//! Log de eventos documentais — append-only, cadeia de hashes verificável.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use core_org::OrgPositionId;
use core_rh::UserId;

use crate::{AuthorityContext, DocumentId, DocumentalError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentEventId(pub String);

impl DocumentEventId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(DocumentalError::EmptyField("event_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentEventType {
    Created,
    StatusChanged,
    PayloadUpdated,
    NumberAssigned,
    NdfRendered,
    /// Assinatura registada (evento de auditoria dentro do estado `Approved`).
    /// A finalização formal é `StatusChanged` → `Finalized`.
    Signed,
    RelationAdded,
    AttachmentAdded,
    Archived,
    Annulled,
}

impl DocumentEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::StatusChanged => "status_changed",
            Self::PayloadUpdated => "payload_updated",
            Self::NumberAssigned => "number_assigned",
            Self::NdfRendered => "ndf_rendered",
            Self::Signed => "signed",
            Self::RelationAdded => "relation_added",
            Self::AttachmentAdded => "attachment_added",
            Self::Archived => "archived",
            Self::Annulled => "annulled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "created" => Some(Self::Created),
            "status_changed" => Some(Self::StatusChanged),
            "payload_updated" => Some(Self::PayloadUpdated),
            "number_assigned" => Some(Self::NumberAssigned),
            "ndf_rendered" => Some(Self::NdfRendered),
            "signed" => Some(Self::Signed),
            "relation_added" => Some(Self::RelationAdded),
            "attachment_added" => Some(Self::AttachmentAdded),
            "archived" => Some(Self::Archived),
            "annulled" => Some(Self::Annulled),
            _ => None,
        }
    }
}

impl TryFrom<&str> for DocumentEventType {
    type Error = crate::DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            crate::DocumentalError::OperationFailed(format!("tipo de evento desconhecido: {s}"))
        })
    }
}

/// Actor de um evento documental.
///
/// Eventos pré-finalização (criação, edição, revisão) têm apenas actor operacional
/// — utilizador + posição, sem contexto de autoridade jurídica formal.
/// Eventos que requerem autoridade jurídica (finalização, arquivo, anulação)
/// exigem `Authority` com o snapshot completo capturado no momento do acto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EventActor {
    /// Actor operacional — pré-finalização.
    Operator {
        user_id: UserId,
        position_id: OrgPositionId,
    },
    /// Actor com autoridade jurídica formal — pós-finalização.
    Authority(AuthorityContext),
}

/// Evento de auditoria documental — append-only.
///
/// `previous_hash` encadeia eventos em sequência verificável.
/// O primeiro evento de um documento tem `previous_hash = None`.
/// Cada evento subsequente inclui o hash do evento anterior,
/// formando uma cadeia imutável de auditoria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentEvent {
    pub id: DocumentEventId,
    pub document_id: DocumentId,
    pub event_type: DocumentEventType,
    pub actor: EventActor,
    pub occurred_at: DateTime<Utc>,
    pub previous_hash: Option<String>,
    pub data_json: Option<Value>,
}

impl DocumentEvent {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.document_id.validate()
    }
}

/// Verificação estrutural da cadeia de eventos.
///
/// Garante:
/// - O primeiro evento não tem `previous_hash`
/// - Todos os eventos subsequentes têm `previous_hash`
/// - A sequência está em ordem cronológica (occurred_at crescente)
///
/// Não recomputa hashes criptográficos — essa verificação é responsabilidade
/// da camada de infra que conhece o algoritmo de hashing utilizado.
pub fn verify_event_chain(events: &[DocumentEvent]) -> Result<(), DocumentalError> {
    if events.is_empty() {
        return Ok(());
    }
    if events[0].previous_hash.is_some() {
        return Err(DocumentalError::EventChainBroken(
            "primeiro evento não deve ter previous_hash".into(),
        ));
    }
    for window in events.windows(2) {
        let prev = &window[0];
        let curr = &window[1];
        if curr.previous_hash.is_none() {
            return Err(DocumentalError::EventChainBroken(format!(
                "evento '{}' não tem previous_hash mas não é o primeiro",
                curr.id.as_str()
            )));
        }
        if curr.occurred_at < prev.occurred_at {
            return Err(DocumentalError::EventChainBroken(format!(
                "evento '{}' tem occurred_at anterior ao evento precedente",
                curr.id.as_str()
            )));
        }
    }
    Ok(())
}

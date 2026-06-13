//! Log de eventos documentais — append-only, cadeia de hashes verificável.
//!
//! Todo o acesso a um documento gera um evento `Accessed`. Todo o acto de custódia
//! (intake, transição de estado, numeração) gera o evento correspondente.
//! A cadeia é imutável e cronologicamente verificável.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AuthoritySnapshot, DocumentId, DocumentalError};

// ── DocumentEventId ───────────────────────────────────────────────────────────

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

// ── AccessPurpose ─────────────────────────────────────────────────────────────

/// Propósito declarado de um acesso a um documento custodiado.
///
/// Obrigatório em eventos `Accessed` — o acesso só é válido se o propósito
/// estiver registado. Transportado em `data_json` do evento com a chave `"purpose"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessPurpose {
    /// Consulta normal por actor autorizado.
    Consultation,
    /// Acção de fiscalização ou controlo externo.
    Inspection,
    /// Exportação via @core-exports (renderização, transmissão).
    Export,
    /// Processo legal ou contencioso.
    LegalProceeding,
    /// Auditoria interna ou externa.
    Audit,
    /// Revisão ou verificação interna.
    InternalReview,
}

impl AccessPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Consultation => "consultation",
            Self::Inspection => "inspection",
            Self::Export => "export",
            Self::LegalProceeding => "legal_proceeding",
            Self::Audit => "audit",
            Self::InternalReview => "internal_review",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "consultation" => Some(Self::Consultation),
            "inspection" => Some(Self::Inspection),
            "export" => Some(Self::Export),
            "legal_proceeding" => Some(Self::LegalProceeding),
            "audit" => Some(Self::Audit),
            "internal_review" => Some(Self::InternalReview),
            _ => None,
        }
    }
}

impl TryFrom<&str> for AccessPurpose {
    type Error = DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            DocumentalError::OperationFailed(format!("propósito de acesso desconhecido: {s}"))
        })
    }
}

// ── DocumentEventType ─────────────────────────────────────────────────────────

/// Tipo de evento documental.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentEventType {
    /// Documento entrou em custódia formal — primeiro evento de qualquer documento.
    CustodyAccepted,
    /// Número administrativo atribuído após intake.
    NumberAssigned,
    /// Documento acedido — `data_json` contém `{ "purpose": "<AccessPurpose>" }`.
    Accessed,
    /// Relação inter-documental registada.
    RelationAdded,
    /// Anexo registado.
    AttachmentAdded,
    /// Transição de estado custodial — `data_json` contém `{ "from": "...", "to": "..." }`.
    StatusChanged,
}

impl DocumentEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CustodyAccepted => "custody_accepted",
            Self::NumberAssigned => "number_assigned",
            Self::Accessed => "accessed",
            Self::RelationAdded => "relation_added",
            Self::AttachmentAdded => "attachment_added",
            Self::StatusChanged => "status_changed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "custody_accepted" => Some(Self::CustodyAccepted),
            "number_assigned" => Some(Self::NumberAssigned),
            "accessed" => Some(Self::Accessed),
            "relation_added" => Some(Self::RelationAdded),
            "attachment_added" => Some(Self::AttachmentAdded),
            "status_changed" => Some(Self::StatusChanged),
            _ => None,
        }
    }
}

impl TryFrom<&str> for DocumentEventType {
    type Error = DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            DocumentalError::OperationFailed(format!("tipo de evento desconhecido: {s}"))
        })
    }
}

// ── EventActor ────────────────────────────────────────────────────────────────

/// Actor de um evento documental.
///
/// Todos os campos são primitivos — não há dependência de tipos de outros
/// bounded contexts. `Operator` é para processos de sistema; `Authority` é
/// obrigatório em actos com relevância jurídica formal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EventActor {
    /// Actor operacional — processo de sistema sem autoridade jurídica formal.
    /// `user_id` e `position_id` são identificadores em formato primitivo.
    Operator {
        user_id: String,
        position_id: String,
    },
    /// Actor com snapshot de autoridade jurídica — obrigatório para actos formais.
    Authority(AuthoritySnapshot),
}

// ── EventFilter ───────────────────────────────────────────────────────────────

/// Filtro de pesquisa sobre o log de eventos de um documento.
///
/// Todos os campos são opcionais — campos `None` não filtram.
/// `limit` e `offset` suportam paginação.
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_type: Option<DocumentEventType>,
    pub from: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: usize,
    pub offset: usize,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            event_type: None,
            from: None,
            until: None,
            limit: 100,
            offset: 0,
        }
    }
}

// ── DocumentEvent ─────────────────────────────────────────────────────────────

/// Evento de auditoria documental — append-only.
///
/// `previous_hash` encadeia eventos em sequência verificável.
/// O primeiro evento de um documento tem `previous_hash = None`.
/// Cada evento subsequente inclui o hash do evento anterior,
/// formando uma cadeia imutável de custódia.
///
/// `data_json` transporta metadados do evento (pequenos, para inspecção);
/// não deve conter conteúdo documental — esse pertence ao `DocumentCustody`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentEvent {
    pub id: DocumentEventId,
    pub document_id: DocumentId,
    pub event_type: DocumentEventType,
    pub actor: EventActor,
    pub occurred_at: DateTime<Utc>,
    /// Hash do evento anterior — `None` no primeiro evento.
    pub previous_hash: Option<String>,
    /// Metadados do evento em JSON (pequeno, para inspecção e auditoria).
    pub data_json: Option<Value>,
}

impl DocumentEvent {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.document_id.validate()
    }

    /// Bytes canónicos para hash — formato determinístico para cálculo de `previous_hash`.
    ///
    /// Layout: `{event_id}:{document_id}:{event_type}:{occurred_at_iso8601}:{prev_hash_or_GENESIS}`
    ///
    /// Adapters devem computar SHA-256 destes bytes e codificar em hex lowercase.
    /// Este método existe no domínio para que todos os adapters usem o mesmo formato.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let prev = self.previous_hash.as_deref().unwrap_or("GENESIS");
        format!(
            "{}:{}:{}:{}:{}",
            self.id.as_str(),
            self.document_id.as_str(),
            self.event_type.as_str(),
            self.occurred_at.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            prev,
        )
        .into_bytes()
    }
}

// ── verify_event_chain ────────────────────────────────────────────────────────

/// Verificação estrutural da cadeia de eventos.
///
/// Garante:
/// - O primeiro evento não tem `previous_hash`
/// - Todos os eventos subsequentes têm `previous_hash`
/// - A sequência está em ordem cronológica (`occurred_at` crescente)
///
/// Não recomputa hashes criptográficos — essa verificação é responsabilidade
/// da camada de infra que conhece o algoritmo utilizado.
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

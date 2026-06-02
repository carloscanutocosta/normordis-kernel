//! Porto de auditoria de `core-org` (driven/secondary port).
//!
//! `core-org` define o contrato e o evento; o adaptador infra (`org-sqlite`)
//! converte `OrgAuditEvent` para `core-audit` (evento + execução de controlo),
//! sem que o domínio conheça `core-audit`. Os eventos são serializáveis para
//! poderem viajar no outbox transaccional (ver `org-sqlite`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::OrgError;

// ── Acção ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OrgAuditAction {
    Created,
    Imported,
    Updated,
    Deactivated,
    StatusChanged { from: String, to: String },
}

impl OrgAuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Imported => "imported",
            Self::Updated => "updated",
            Self::Deactivated => "deactivated",
            Self::StatusChanged { .. } => "status_changed",
        }
    }
}

// ── Resultado ─────────────────────────────────────────────────────────────────

/// Resultado de uma operação controlada — base da evidência COSO.
/// `Success` mapeia para `AuditOutcome::Success` + `ControlExecution::Passed`;
/// `Failure` para `AuditOutcome::Failure` + `ControlExecution::Failed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgEventOutcome {
    Success,
    Failure,
}

impl OrgEventOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

// ── Evento ────────────────────────────────────────────────────────────────────

/// Evento de auditoria emitido pela camada de serviço.
///
/// Serializável para viajar no outbox transaccional. Transporta o resultado
/// (`outcome`) e o controlo COSO primário (`control_id`) para permitir evidência
/// tanto de sucessos como de falhas/negações, ligada a um controlo do registo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgAuditEvent {
    /// Identidade estável do evento. Gerada na construção e preservada através do
    /// outbox, garantindo que uma reentrega produz o mesmo `AuditEvent`
    /// (dedup idempotente no `AuditStore`).
    pub event_id: String,
    pub actor: String,
    pub action: OrgAuditAction,
    pub entity_kind: String,
    pub entity_id: String,
    pub occurred_at: DateTime<Utc>,
    pub outcome: OrgEventOutcome,
    /// Controlo COSO primário evidenciado por esta operação.
    pub control_id: Option<String>,
    /// Estado da entidade após a operação (snapshot) ou contexto da falha.
    pub payload: Option<Value>,
}

#[allow(clippy::too_many_arguments)]
impl OrgAuditEvent {
    /// Constrói um evento com `event_id` UUID v4 gerado.
    pub fn new(
        actor: impl Into<String>,
        action: OrgAuditAction,
        entity_kind: impl Into<String>,
        entity_id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        outcome: OrgEventOutcome,
        control_id: Option<String>,
        payload: Option<Value>,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            actor: actor.into(),
            action,
            entity_kind: entity_kind.into(),
            entity_id: entity_id.into(),
            occurred_at,
            outcome,
            control_id,
            payload,
        }
    }
}

// ── Porto ─────────────────────────────────────────────────────────────────────

/// Porto de entrega de evidência de auditoria. Implementado pelo adaptador infra,
/// que grava o `AuditEvent` na cadeia de hashes e a `ControlExecution` no registo
/// de controlos.
///
/// Deve ser **idempotente** por `entity_id`+`occurred_at`/conteúdo: o drainer do
/// outbox pode reentregar um evento; uma reentrega de evento já gravado deve
/// devolver `Ok(())` (ou `OrgError::AlreadyExists`, que o drainer trata como
/// entregue).
pub trait OrgAuditPort {
    fn record(&self, event: &OrgAuditEvent) -> Result<(), OrgError>;
}

// ── Implementação nula ────────────────────────────────────────────────────────

pub struct OrgNoopAudit;

impl OrgAuditPort for OrgNoopAudit {
    fn record(&self, _event: &OrgAuditEvent) -> Result<(), OrgError> {
        Ok(())
    }
}

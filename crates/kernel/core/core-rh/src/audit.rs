//! Bridge para `core-audit` e infraestrutura de evidência COSO de `core-rh`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{RhError, UserProfile};

// ── Bridge para core-audit ────────────────────────────────────────────────────

pub fn audit_actor_from_user(user: &UserProfile) -> core_audit::AuditActor {
    core_audit::AuditActor::with_metadata(
        user.user_id.as_str(),
        Some(user.display_name.clone()),
        Some("user".to_owned()),
    )
    .expect("valid core-rh user profiles must map to valid audit actors")
}

// ── RhAuditAction ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RhAuditAction {
    /// Afetação de uma pessoa a uma posição.
    Assign,
    /// Encerramento de uma afetação.
    CloseAssignment,
    /// Criação ou actualização de um utilizador.
    UpsertUser,
    /// Desactivação de um utilizador.
    DeactivateUser,
}

impl RhAuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "assign",
            Self::CloseAssignment => "close_assignment",
            Self::UpsertUser => "upsert_user",
            Self::DeactivateUser => "deactivate_user",
        }
    }
}

// ── RhEventOutcome ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RhEventOutcome {
    Success,
    Failure,
}

impl RhEventOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

// ── RhAuditEvent ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhAuditEvent {
    pub event_id: Uuid,
    pub actor: String,
    pub action: RhAuditAction,
    /// Tipo de entidade afectada (ex: "PersonAssignment", "User").
    pub entity_kind: String,
    pub entity_id: String,
    pub occurred_at: DateTime<Utc>,
    pub outcome: RhEventOutcome,
    pub control_id: Option<String>,
    pub payload: Option<Value>,
}

impl RhAuditEvent {
    pub fn new(
        actor: impl Into<String>,
        action: RhAuditAction,
        entity_kind: impl Into<String>,
        entity_id: impl Into<String>,
        outcome: RhEventOutcome,
        control_id: Option<String>,
        payload: Option<Value>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            actor: actor.into(),
            action,
            entity_kind: entity_kind.into(),
            entity_id: entity_id.into(),
            occurred_at: Utc::now(),
            outcome,
            control_id,
            payload,
        }
    }
}

// ── RhAuditPort ───────────────────────────────────────────────────────────────

/// Porto de destino da evidência — implementado pelo adapter de auditoria.
pub trait RhAuditPort {
    fn record(&self, event: &RhAuditEvent) -> Result<(), RhError>;
}

// ── RhNoopAudit ───────────────────────────────────────────────────────────────

/// Implementação nula — para testes e contextos sem auditoria real.
pub struct RhNoopAudit;

impl RhAuditPort for RhNoopAudit {
    fn record(&self, _event: &RhAuditEvent) -> Result<(), RhError> {
        Ok(())
    }
}

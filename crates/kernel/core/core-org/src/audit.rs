//! Porto de auditoria de `core-org` (driven/secondary port).
//!
//! `core-org` define o contrato; a implementação concreta vive no adaptador infra
//! (`org-sqlite`) que converte `OrgAuditEvent` para o formato de `core-audit`.
//! Desta forma a dependência org → audit não viola o hexagonal.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::OrgError;

// ── Acção ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum OrgAuditAction {
    Created,
    Imported,
    Updated,
    Deactivated,
    StatusChanged {
        from: &'static str,
        to: &'static str,
    },
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

// ── Evento ────────────────────────────────────────────────────────────────────

/// Evento de auditoria emitido pela camada de serviço.
///
/// `payload` contém o estado relevante após a operação (serializado como JSON).
/// O adaptador infra converte este evento para o formato de `core-audit`.
#[derive(Debug, Clone)]
pub struct OrgAuditEvent {
    pub actor: String,
    pub action: OrgAuditAction,
    pub entity_kind: &'static str,
    pub entity_id: String,
    pub occurred_at: DateTime<Utc>,
    /// Estado da entidade após a operação (delta / snapshot parcial).
    pub payload: Option<Value>,
}

impl OrgAuditEvent {
    pub fn new(
        actor: impl Into<String>,
        action: OrgAuditAction,
        entity_kind: &'static str,
        entity_id: impl Into<String>,
        occurred_at: DateTime<Utc>,
        payload: Option<Value>,
    ) -> Self {
        Self {
            actor: actor.into(),
            action,
            entity_kind,
            entity_id: entity_id.into(),
            occurred_at,
            payload,
        }
    }
}

// ── Porto ─────────────────────────────────────────────────────────────────────

pub trait OrgAuditPort {
    fn record(&self, event: OrgAuditEvent) -> Result<(), OrgError>;
}

// ── Implementação nula ────────────────────────────────────────────────────────

pub struct OrgNoopAudit;

impl OrgAuditPort for OrgNoopAudit {
    fn record(&self, _event: OrgAuditEvent) -> Result<(), OrgError> {
        Ok(())
    }
}

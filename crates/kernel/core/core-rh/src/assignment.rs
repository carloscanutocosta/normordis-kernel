//! Afetação temporal de uma pessoa a uma posição orgânica.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{RhError, UserId};

// ── PersonAssignmentId ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonAssignmentId(pub String);

impl PersonAssignmentId {
    pub fn new(id: impl Into<String>) -> Result<Self, RhError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(RhError::InvalidAssignment("assignment_id vazio".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── PersonAssignment ──────────────────────────────────────────────────────────

/// Afetação temporal de uma pessoa a uma posição orgânica.
///
/// `position_id` e `unit_id` são referências leves (String) para preservar a
/// independência de `core-org`. `basis` regista o despacho ou instrumento
/// jurídico que fundamentou a afetação — necessário para rastreabilidade COSO.
///
/// `version` é incrementado pelo repositório em cada `close` (OCC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonAssignment {
    pub id: PersonAssignmentId,
    pub person_id: UserId,
    /// ID da posição orgânica (referência leve, sem dep. directa de core-org).
    pub position_id: String,
    /// ID da unidade orgânica.
    pub unit_id: String,
    /// Despacho ou instrumento jurídico que criou a afetação.
    pub basis: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub version: u32,
}

impl PersonAssignment {
    pub fn validate(&self) -> Result<(), RhError> {
        if self.position_id.trim().is_empty() {
            return Err(RhError::InvalidAssignment("position_id vazio".into()));
        }
        if self.unit_id.trim().is_empty() {
            return Err(RhError::InvalidAssignment("unit_id vazio".into()));
        }
        if self.basis.trim().is_empty() {
            return Err(RhError::InvalidAssignment("basis vazio".into()));
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(RhError::InvalidAssignment(
                    "valid_until deve ser posterior a valid_from".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn is_effective_at(&self, date: NaiveDate) -> bool {
        date >= self.valid_from && self.valid_until.is_none_or(|u| date < u)
    }
}

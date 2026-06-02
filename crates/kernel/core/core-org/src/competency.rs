//! Competências: autoridade jurídica para praticar actos administrativos, com validade temporal.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{LegalInstrumentId, OrgError, OrgPositionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompetencyId(pub String);

impl CompetencyId {
    pub fn new(id: impl Into<String>) -> Result<Self, OrgError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OrgError::EmptyField("competency_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Competência — autoridade para praticar um acto jurídico ou administrativo.
/// Associada a uma posição orgânica por instrumento jurídico com validade temporal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Competency {
    pub id: CompetencyId,
    pub code: String,
    pub description: String,
    /// Âmbito da autoridade conferida (ex: "Assinar ofícios de nível 1 e 2")
    pub scope: String,
    pub assigned_to: OrgPositionId,
    pub granted_by: LegalInstrumentId,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    /// Versão para OCC — deve ser 0 em novas entidades.
    pub version: u32,
}

impl Competency {
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.code.trim().is_empty() {
            return Err(OrgError::EmptyField("code".into()));
        }
        if self.description.trim().is_empty() {
            return Err(OrgError::EmptyField("description".into()));
        }
        if self.scope.trim().is_empty() {
            return Err(OrgError::EmptyField("scope".into()));
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        Ok(())
    }

    pub fn is_effective_at(&self, date: NaiveDate) -> bool {
        date >= self.valid_from && self.valid_until.map_or(true, |u| date < u)
    }
}

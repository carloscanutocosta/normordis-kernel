//! Cargos e posições orgânicas — abstractos, independentes de quem os ocupa.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{LegalInstrumentId, OrgError, OrgUnitId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgPositionId(pub String);

impl OrgPositionId {
    pub fn new(id: impl Into<String>) -> Result<Self, OrgError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OrgError::EmptyField("position_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cargo ou posição orgânica — abstracto, independente de quem o ocupa.
/// A ocupação de uma posição por uma pessoa é responsabilidade de core-rh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgPosition {
    pub id: OrgPositionId,
    pub code: String,
    pub title: String,
    pub unit_id: OrgUnitId,
    pub created_by: LegalInstrumentId,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}

impl OrgPosition {
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.code.trim().is_empty() {
            return Err(OrgError::EmptyField("code".into()));
        }
        if self.title.trim().is_empty() {
            return Err(OrgError::EmptyField("title".into()));
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        Ok(())
    }

    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        date >= self.valid_from && self.valid_until.is_none_or(|u| date < u)
    }
}

//! Delegação de competência entre posições orgânicas com instrumento jurídico e validade temporal.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{CompetencyId, LegalInstrumentId, OrgError, OrgPositionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DelegationId(pub String);

impl DelegationId {
    pub fn new(id: impl Into<String>) -> Result<Self, OrgError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OrgError::EmptyField("delegation_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Delegação de competência entre posições orgânicas.
/// Requer instrumento jurídico próprio e tem validade temporal explícita.
/// Uma delegação não transfere a competência — o delegante mantém-na.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub id: DelegationId,
    pub competency_id: CompetencyId,
    pub from_position: OrgPositionId,
    pub to_position: OrgPositionId,
    pub instrument_id: LegalInstrumentId,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    /// Versão para OCC — deve ser 0 em novas entidades.
    pub version: u32,
}

impl Delegation {
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.from_position == self.to_position {
            return Err(OrgError::OperationFailed(
                "delegante e delegado não podem ser a mesma posição".into(),
            ));
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        Ok(())
    }

    pub fn is_effective_at(&self, date: NaiveDate) -> bool {
        date >= self.valid_from && self.valid_until.is_none_or(|u| date < u)
    }

    /// Verifica que `from_position` detém efectivamente a competência que está
    /// a delegar. O chamador deve fornecer as competências activas de `from_position`
    /// na data relevante (obtidas via `CompetencyRepository::list_for_position_at`).
    pub fn validate_can_delegate(
        &self,
        from_position_competencies: &[&CompetencyId],
    ) -> Result<(), OrgError> {
        let has_competency = from_position_competencies.contains(&&self.competency_id);
        if !has_competency {
            return Err(OrgError::OperationFailed(format!(
                "posição '{}' não detém a competência '{}' que pretende delegar",
                self.from_position.as_str(),
                self.competency_id.as_str()
            )));
        }
        Ok(())
    }
}

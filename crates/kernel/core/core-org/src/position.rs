//! Cargos e posições orgânicas — abstractos, independentes de quem os ocupa.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{LegalInstrumentId, OrgError, OrgUnitId};

// ── OrgPositionId ─────────────────────────────────────────────────────────────

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

// ── OrgPositionStatus ─────────────────────────────────────────────────────────

/// Estado de ciclo de vida de um cargo orgânico.
/// Active → Suspended|Extinct · Suspended → Active|Extinct · Extinct é terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgPositionStatus {
    Active,
    Suspended,
    Extinct,
}

impl OrgPositionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Extinct => "extinct",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "extinct" => Some(Self::Extinct),
            _ => None,
        }
    }

    pub fn can_transition_to(&self, next: &Self) -> bool {
        use OrgPositionStatus::*;
        matches!(
            (self, next),
            (Active, Suspended) | (Active, Extinct) | (Suspended, Active) | (Suspended, Extinct)
        )
    }
}

impl TryFrom<&str> for OrgPositionStatus {
    type Error = OrgError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
            .ok_or_else(|| OrgError::OperationFailed(format!("status de cargo desconhecido: {s}")))
    }
}

// ── PositionKind ──────────────────────────────────────────────────────────────

/// Classificação estrutural de uma posição orgânica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionKind {
    /// Cargo de direcção superior (director-geral, director de serviços…)
    Direcao,
    /// Cargo de coordenação
    Coordenacao,
    /// Cargo de chefia (chefe de serviço, chefe de divisão…)
    Chefia,
    /// Adjunto do chefe — pode ser substituto legal (ver `substitutes`)
    Adjunto,
    /// Cargo técnico sem liderança hierárquica formal
    Tecnico,
    /// Outros tipos, com descrição livre
    Outro(String),
}

impl PositionKind {
    pub fn as_str(&self) -> String {
        match self {
            Self::Direcao => "direcao".into(),
            Self::Coordenacao => "coordenacao".into(),
            Self::Chefia => "chefia".into(),
            Self::Adjunto => "adjunto".into(),
            Self::Tecnico => "tecnico".into(),
            Self::Outro(s) if s.is_empty() => "outro".into(),
            Self::Outro(s) => format!("outro:{s}"),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "direcao" => Some(Self::Direcao),
            "coordenacao" => Some(Self::Coordenacao),
            "chefia" => Some(Self::Chefia),
            "adjunto" => Some(Self::Adjunto),
            "tecnico" => Some(Self::Tecnico),
            "outro" => Some(Self::Outro(String::new())),
            other if other.starts_with("outro:") => Some(Self::Outro(other[6..].to_string())),
            _ => None,
        }
    }
}

impl TryFrom<&str> for PositionKind {
    type Error = OrgError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
            .ok_or_else(|| OrgError::OperationFailed(format!("tipo de cargo desconhecido: {s}")))
    }
}

// ── OrgPosition ───────────────────────────────────────────────────────────────

/// Cargo ou posição orgânica — abstracto, independente de quem o ocupa.
/// A ocupação de uma posição por uma pessoa é responsabilidade de core-rh.
///
/// `substitutes` — se preenchido, esta posição é o substituto legal da posição
/// referenciada nas ausências e impedimentos do respectivo titular.
///
/// `version` é incrementado pelo repositório em cada `update()` (OCC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgPosition {
    pub id: OrgPositionId,
    pub code: String,
    pub title: String,
    pub kind: PositionKind,
    pub substitutes: Option<OrgPositionId>,
    pub status: OrgPositionStatus,
    pub unit_id: OrgUnitId,
    pub created_by: LegalInstrumentId,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    /// Versão para OCC — deve ser 0 em novas entidades.
    pub version: u32,
}

impl OrgPosition {
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.code.trim().is_empty() {
            return Err(OrgError::EmptyField("code".into()));
        }
        if self.title.trim().is_empty() {
            return Err(OrgError::EmptyField("title".into()));
        }
        if let Some(ref sub) = self.substitutes {
            if sub == &self.id {
                return Err(OrgError::OperationFailed(
                    "uma posição não pode ser substituto de si própria".into(),
                ));
            }
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        Ok(())
    }

    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        matches!(self.status, OrgPositionStatus::Active)
            && date >= self.valid_from
            && self.valid_until.map_or(true, |u| date < u)
    }

    pub fn is_extinct(&self) -> bool {
        matches!(self.status, OrgPositionStatus::Extinct)
    }

    /// Valida a transição de status. Não muta o agregado.
    pub fn transition_status(
        &self,
        next: OrgPositionStatus,
    ) -> Result<OrgPositionStatus, OrgError> {
        if !self.status.can_transition_to(&next) {
            return Err(OrgError::OperationFailed(format!(
                "transição inválida de cargo: {} → {}",
                self.status.as_str(),
                next.as_str()
            )));
        }
        Ok(next)
    }

    /// Verifica que esta posição não aparece numa cadeia de substituição existente.
    /// O chamador obtém a cadeia via repositório antes de persistir.
    pub fn validate_no_substitute_cycle(&self, chain: &[&OrgPositionId]) -> Result<(), OrgError> {
        if chain.iter().any(|id| *id == &self.id) {
            return Err(OrgError::SubstitutionCycle);
        }
        Ok(())
    }
}

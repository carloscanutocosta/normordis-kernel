use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Tipologia de ciclo de avaliação formal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleType {
    /// Ciclo SIADAP anual (Lei n.º 66-B/2007).
    SiadapAnnual,
    /// Ciclo BSC trimestral.
    BscQuarterly,
    /// Ciclo BSC semestral.
    BscSemestral,
    /// Ciclo BSC anual.
    BscAnnual,
    /// Ciclo de programa ou plano estratégico ad-hoc.
    ProgramCycle,
    /// Ciclo personalizado (detalhado em `governance_context`).
    Custom,
}

impl CycleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SiadapAnnual => "siadap_annual",
            Self::BscQuarterly => "bsc_quarterly",
            Self::BscSemestral => "bsc_semestral",
            Self::BscAnnual => "bsc_annual",
            Self::ProgramCycle => "program_cycle",
            Self::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "siadap_annual" => Some(Self::SiadapAnnual),
            "bsc_quarterly" => Some(Self::BscQuarterly),
            "bsc_semestral" => Some(Self::BscSemestral),
            "bsc_annual" => Some(Self::BscAnnual),
            "program_cycle" => Some(Self::ProgramCycle),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Estado de um ciclo de avaliação.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleStatus {
    Planned,
    Open,
    Closed,
    Archived,
}

impl CycleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "planned" => Some(Self::Planned),
            "open" => Some(Self::Open),
            "closed" => Some(Self::Closed),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

/// Período formal de avaliação institucional.
///
/// Exemplos:
/// - Ciclo SIADAP 2026 (1 Jan 2026 – 31 Dez 2026)
/// - BSC Q1 2026 (1 Jan 2026 – 31 Mar 2026)
/// - Programa PRACE 2025-2027
///
/// O `code` é estável e único, usado como referência em `IndicatorInstance`
/// e em `MetricEvent.evaluation_cycle_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationCycle {
    pub id: String,
    pub code: String,
    pub name: String,
    pub cycle_type: CycleType,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Contexto normativo: referência legal, despacho de abertura, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_context: Option<String>,
    pub status: CycleStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl EvaluationCycle {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.code.trim().is_empty()
            || self.name.trim().is_empty()
            || self.created_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        if self.period_end <= self.period_start {
            return Err(MetricError::InvalidCriteria);
        }
        Ok(())
    }

    pub fn contains(&self, at: DateTime<Utc>) -> bool {
        at >= self.period_start && at < self.period_end
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Âmbito de aplicação de um target (unidade orgânica, serviço, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    OrgUnit,
    Service,
    Individual,
    Global,
}

impl ScopeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OrgUnit => "org_unit",
            Self::Service => "service",
            Self::Individual => "individual",
            Self::Global => "global",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "org_unit" => Some(Self::OrgUnit),
            "service" => Some(Self::Service),
            "individual" => Some(Self::Individual),
            "global" => Some(Self::Global),
            _ => None,
        }
    }
}

/// Threshold de semáforo associado a um target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    pub label: String,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    /// Cor semáforo: "green" | "yellow" | "red" (ou outro valor governado).
    pub color: String,
}

/// Target (objectivo/limiar) associado a uma versão de métrica.
///
/// Targets são artefactos governados — não podem ser alterados livremente
/// por apps. Podem variar por unidade orgânica, ciclo ou âmbito.
///
/// Invariante: targets não alteram a fórmula da métrica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetDefinition {
    pub id: String,
    pub metric_version_id: String,
    pub scope_type: ScopeType,
    /// Identificador do âmbito (org_unit_id, service_id, etc.).
    pub scope_id: String,
    pub target_value: f64,
    pub unit: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub thresholds: Vec<Threshold>,
    pub valid_from: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl TargetDefinition {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.metric_version_id.trim().is_empty()
            || self.scope_id.trim().is_empty()
            || self.unit.trim().is_empty()
            || self.created_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        if self.target_value.is_nan() || self.target_value.is_infinite() {
            return Err(MetricError::InvalidValue);
        }
        if let (Some(to), from) = (self.valid_to, self.valid_from) {
            if to <= from {
                return Err(MetricError::InvalidCriteria);
            }
        }
        Ok(())
    }
}

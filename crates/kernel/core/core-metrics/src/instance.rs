use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Estado de uma instância de indicador num ciclo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Pending,
    InProgress,
    Calculated,
    Validated,
    Closed,
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Calculated => "calculated",
            Self::Validated => "validated",
            Self::Closed => "closed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "calculated" => Some(Self::Calculated),
            "validated" => Some(Self::Validated),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

/// Instanciação de uma métrica para um ciclo, unidade orgânica e responsável.
///
/// Liga `MetricVersion` + `EvaluationCycle` + âmbito organizacional.
/// Não contém o resultado — define o alvo institucional da medição.
///
/// Permite comparar unidades orgânicas e períodos de forma governada, pois
/// todos os `MeasurementResult`s são referenciados a uma `IndicatorInstance`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorInstance {
    pub id: String,
    pub metric_version_id: String,
    pub evaluation_cycle_id: String,
    pub org_unit_id: String,
    /// Actor institucional responsável pela medição neste ciclo.
    pub responsible_actor_id: String,
    /// Âmbito adicional (ex: serviço específico dentro da UO).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub status: InstanceStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
}

impl IndicatorInstance {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.metric_version_id.trim().is_empty()
            || self.evaluation_cycle_id.trim().is_empty()
            || self.org_unit_id.trim().is_empty()
            || self.responsible_actor_id.trim().is_empty()
            || self.created_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        Ok(())
    }
}

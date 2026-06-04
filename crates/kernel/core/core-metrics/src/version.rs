use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Descreve como uma evidência deve ser recolhida para o cálculo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRequirement {
    pub source_type: String,
    pub description: String,
    pub mandatory: bool,
}

/// Binding que descreve como calcular a métrica a partir de fontes.
///
/// Em mini-apps, pode ser uma expressão textual ou referência a uma
/// função. O cálculo efectivo é da responsabilidade do layer de agregação,
/// não do `core-metrics`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalculationBinding {
    pub kind: String,
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Estado de publicação de uma versão métrica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricVersionStatus {
    Draft,
    Published,
    Retired,
}

impl MetricVersionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Retired => "retired",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "published" => Some(Self::Published),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

/// Versão versionada de uma definição métrica com fórmula e vigência.
///
/// Alterações materiais à fórmula, fontes ou severidade criam nova versão.
/// Resultados devem sempre referenciar a versão usada no cálculo.
///
/// Invariantes:
/// - A vigência (`valid_from`, `valid_to`) não pode sobrepor outra versão
///   publicada da mesma métrica.
/// - `metric_definition_id` + `version` devem ser únicos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricVersion {
    pub id: String,
    pub metric_definition_id: String,
    pub version: String,
    pub status: MetricVersionStatus,
    pub valid_from: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<DateTime<Utc>>,
    /// Referência textual à fórmula (ex: nome de função, expressão).
    pub formula_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculation_binding: Option<CalculationBinding>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_requirements: Vec<EvidenceRequirement>,
    /// Referência ao ato de aprovação (ex: id de despacho em `core-documental`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl MetricVersion {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.metric_definition_id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.formula_ref.trim().is_empty()
            || self.created_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        if let (Some(to), from) = (self.valid_to, self.valid_from) {
            if to <= from {
                return Err(MetricError::InvalidCriteria);
            }
        }
        Ok(())
    }

    pub fn is_active_at(&self, at: DateTime<Utc>) -> bool {
        if at < self.valid_from {
            return false;
        }
        if let Some(to) = self.valid_to {
            if at >= to {
                return false;
            }
        }
        matches!(self.status, MetricVersionStatus::Published)
    }
}

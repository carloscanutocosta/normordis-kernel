use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Estado de ciclo de vida de uma definição métrica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricDefinitionStatus {
    /// Rascunho — ainda não aprovada para uso.
    Draft,
    /// Activa — pode ser emitida por apps.
    Active,
    /// Suspensa — emissão temporariamente bloqueada.
    Suspended,
    /// Retirada — substituída ou descontinuada; não pode ser emitida.
    Retired,
}

impl MetricDefinitionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Retired => "retired",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

/// Artefacto governado que define um indicador institucional.
///
/// A definição não contém resultados nem fórmulas executáveis — estas ficam
/// em `MetricVersion`. A `MetricDefinition` representa a identidade estável
/// e o propósito do indicador enquanto decisão humana do órgão de gestão.
///
/// Invariante: `code` é estável e único no sistema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    /// Código canónico estável. Padrão: `^[a-z][a-z0-9_.-]*$`.
    /// É o mesmo valor que `MetricEvent.metric_code`.
    pub code: String,
    pub name: String,
    pub description: String,
    /// Propósito institucional: o que se quer medir e porquê.
    pub purpose: String,
    pub owner_org_unit_id: String,
    pub governance_owner: String,
    pub status: MetricDefinitionStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}

impl MetricDefinition {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.code.trim().is_empty()
            || self.name.trim().is_empty()
            || self.owner_org_unit_id.trim().is_empty()
            || self.governance_owner.trim().is_empty()
            || self.created_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        if !is_valid_metric_code(&self.code) {
            return Err(MetricError::InvalidName);
        }
        Ok(())
    }
}

fn is_valid_metric_code(code: &str) -> bool {
    let mut chars = code.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.' || c == '-')
}

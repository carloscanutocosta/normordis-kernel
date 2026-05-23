use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

/// Estado de um resultado de medição.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementStatus {
    /// Calculado — ainda não validado.
    Calculated,
    /// Validado — pode ser usado como leitura oficial.
    Validated,
    /// Retificado — substituído por outro resultado; mantido para auditoria.
    Rectified,
    /// Inválido — rejeitado; não pode ser promovido como leitura oficial.
    Invalid,
}

impl MeasurementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Calculated => "calculated",
            Self::Validated => "validated",
            Self::Rectified => "rectified",
            Self::Invalid => "invalid",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "calculated" => Some(Self::Calculated),
            "validated" => Some(Self::Validated),
            "rectified" => Some(Self::Rectified),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

/// Tipo de evidência ligada ao resultado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Evento auditável (`core-audit`).
    AuditEvent,
    /// Documento final (`core-documental`).
    Document,
    /// Evento métrico operacional (`MetricEvent`).
    MetricEvent,
    /// Estado validado de entidade.
    ValidatedState,
    /// Snapshot governado exportado.
    ExportSnapshot,
}

impl EvidenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuditEvent => "audit_event",
            Self::Document => "document",
            Self::MetricEvent => "metric_event",
            Self::ValidatedState => "validated_state",
            Self::ExportSnapshot => "export_snapshot",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "audit_event" => Some(Self::AuditEvent),
            "document" => Some(Self::Document),
            "metric_event" => Some(Self::MetricEvent),
            "validated_state" => Some(Self::ValidatedState),
            "export_snapshot" => Some(Self::ExportSnapshot),
            _ => None,
        }
    }
}

/// Liga um resultado de medição a uma fonte de evidência concreta.
///
/// Permite auditoria ex-post: reconstruir como e a partir de que fontes
/// um resultado foi calculado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceLink {
    pub id: String,
    pub measurement_result_id: String,
    pub evidence_type: EvidenceType,
    /// Referência ao `@core` de origem (ex: "core-audit", "core-documental").
    pub core_ref: String,
    pub resource_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Hash SHA-256 do recurso no momento do cálculo (reprodutibilidade).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    pub linked_at: DateTime<Utc>,
}

/// Resultado materializado de uma `IndicatorInstance`.
///
/// Invariantes:
/// - referencia sempre `MetricVersion` (saber com que fórmula foi calculado);
/// - não pode ser editado livremente — correcções criam novo resultado com
///   status `rectified` no anterior;
/// - um resultado `invalid` não pode ser promovido como leitura oficial;
/// - `calculation_snapshot_hash` permite verificar reprodutibilidade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementResult {
    pub id: String,
    pub indicator_instance_id: String,
    pub metric_version_id: String,
    pub value: f64,
    pub unit: String,
    pub status: MeasurementStatus,
    pub calculated_at: DateTime<Utc>,
    pub calculated_by: String,
    /// Hash do snapshot de dados usado no cálculo — para verificação posterior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculation_snapshot_hash: Option<String>,
    /// Flags de qualidade (ex: "insufficient_data", "estimated").
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quality_flags: Vec<String>,
    /// Instante institucional relevante (ex: fim do período avaliado).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    /// Id do resultado que este rectifica (se aplicável).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rectifies_result_id: Option<String>,
    /// Dados estruturados específicos do domínio (ex: rating SIADAP, justificação).
    /// O campo `value: f64` mantém-se para agregação; `payload` transporta
    /// dados qualitativos que não cabem num escalar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl MeasurementResult {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty()
            || self.indicator_instance_id.trim().is_empty()
            || self.metric_version_id.trim().is_empty()
            || self.unit.trim().is_empty()
            || self.calculated_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        if self.value.is_nan() || self.value.is_infinite() {
            return Err(MetricError::InvalidValue);
        }
        Ok(())
    }

    pub fn is_official(&self) -> bool {
        matches!(self.status, MeasurementStatus::Validated)
    }
}

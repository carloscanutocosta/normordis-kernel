use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::MetricError;

/// Observação quantitativa que alimenta uma métrica governada pelo órgão de gestão.
///
/// `metric_code` identifica qual `MetricDefinition` (definida em `metrics-studio`)
/// este evento alimenta. Apps não definem métricas — apenas emitem observações
/// com o código pré-aprovado.
///
/// Invariante: métricas não substituem auditoria institucional (`core-audit`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricEvent {
    pub id: String,
    /// Código canónico da `MetricDefinition` que este evento alimenta.
    /// Padrão: `^[a-z][a-z0-9_.-]*$` (e.g. `process.duration`, `document.count`).
    pub metric_code: String,
    /// Versão da definição métrica vigente no momento da observação.
    /// Permite ao layer de agregação saber com que fórmula calcular.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_version_id: Option<String>,
    /// Ciclo de avaliação formal (SIADAP anual, BSC trimestral, etc.) a que
    /// este evento pertence. Preenchido pela app quando o ciclo é conhecido
    /// no momento de emissão.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_cycle_id: Option<String>,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_unit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Instante institucional relevante, distinto do timestamp técnico de emissão.
    /// Ex: data da operação avaliada num ciclo SIADAP passado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub timestamp: DateTime<Utc>,
}

impl MetricEvent {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.id.trim().is_empty() {
            return Err(MetricError::MissingField);
        }
        if self.metric_code.trim().is_empty() {
            return Err(MetricError::MissingField);
        }
        if !is_valid_metric_code(&self.metric_code) {
            return Err(MetricError::InvalidName);
        }
        if self.value.is_nan() || self.value.is_infinite() {
            return Err(MetricError::InvalidValue);
        }
        Ok(())
    }
}

/// Cria um `MetricEvent` mínimo com `timestamp` UTC preenchido automaticamente.
pub fn new_event(
    id: impl Into<String>,
    metric_code: impl Into<String>,
    value: f64,
    unit: Option<impl Into<String>>,
    labels: Option<HashMap<String, String>>,
) -> MetricEvent {
    MetricEvent {
        id: id.into(),
        metric_code: metric_code.into(),
        metric_version_id: None,
        evaluation_cycle_id: None,
        value,
        unit: unit.map(Into::into),
        correlation_id: None,
        entity_type: None,
        entity_id: None,
        state: None,
        org_unit_id: None,
        source_app: None,
        version: None,
        valid_at: None,
        labels,
        payload: None,
        timestamp: Utc::now(),
    }
}

/// Valida o código canónico de uma métrica: `^[a-z][a-z0-9_.-]*$`.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_event() -> MetricEvent {
        new_event(
            "m-001",
            "process.duration",
            1.0,
            Some("ms"),
            None::<HashMap<String, String>>,
        )
    }

    #[test]
    fn valid_event_passes_validation() {
        assert!(valid_event().validate().is_ok());
    }

    #[test]
    fn empty_id_is_rejected() {
        let mut ev = valid_event();
        ev.id = String::new();
        assert_eq!(ev.validate(), Err(MetricError::MissingField));
    }

    #[test]
    fn empty_metric_code_is_rejected() {
        let mut ev = valid_event();
        ev.metric_code = String::new();
        assert_eq!(ev.validate(), Err(MetricError::MissingField));
    }

    #[test]
    fn invalid_metric_code_with_space_is_rejected() {
        let mut ev = valid_event();
        ev.metric_code = "invalid metric".to_string();
        assert_eq!(ev.validate(), Err(MetricError::InvalidName));
    }

    #[test]
    fn invalid_metric_code_starting_with_digit_is_rejected() {
        let mut ev = valid_event();
        ev.metric_code = "1metric".to_string();
        assert_eq!(ev.validate(), Err(MetricError::InvalidName));
    }

    #[test]
    fn valid_codes_with_separators_are_accepted() {
        for code in ["a", "a.b", "a-b", "a_b", "process.duration.p99"] {
            let mut ev = valid_event();
            ev.metric_code = code.to_string();
            assert!(ev.validate().is_ok(), "expected valid: {code}");
        }
    }

    #[test]
    fn nan_value_is_rejected() {
        let mut ev = valid_event();
        ev.value = f64::NAN;
        assert_eq!(ev.validate(), Err(MetricError::InvalidValue));
    }

    #[test]
    fn infinite_value_is_rejected() {
        let mut ev = valid_event();
        ev.value = f64::INFINITY;
        assert_eq!(ev.validate(), Err(MetricError::InvalidValue));
    }

    #[test]
    fn new_event_has_non_zero_timestamp() {
        let ev = valid_event();
        assert!(ev.timestamp.timestamp() > 0);
    }

    #[test]
    fn governance_relation_fields_are_accepted() {
        let mut ev = valid_event();
        ev.metric_version_id = Some("mv-001".to_string());
        ev.evaluation_cycle_id = Some("cycle-siadap-2026".to_string());
        ev.org_unit_id = Some("uo:porto".to_string());
        ev.source_app = Some("ops-console".to_string());
        assert!(ev.validate().is_ok());
    }
}

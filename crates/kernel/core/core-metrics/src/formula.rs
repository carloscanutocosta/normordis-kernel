use crate::error::MetricError;
use crate::event::MetricEvent;
use crate::target::TargetDefinition;
use crate::version::CalculationBinding;

/// Motor de cálculo de métricas.
///
/// Recebe o binding declarado na versão da métrica e os eventos operacionais
/// recolhidos no período, e devolve o valor calculado da métrica.
///
/// A separação entre `FormulaEngine` e `MeasurementResultStore` permite que
/// o cálculo seja testado de forma isolada sem persistência.
pub trait FormulaEngine: Send + Sync {
    fn calculate(
        &self,
        binding: &CalculationBinding,
        events: &[MetricEvent],
        target: Option<&TargetDefinition>,
    ) -> Result<f64, MetricError>;
}

// ── Agregações suportadas pelo BasicFormulaEngine ─────────────────────────────

/// Tipo de agregação para `CalculationBinding { kind: "aggregate" }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregationKind {
    /// Soma de `MetricEvent.value`.
    Sum,
    /// Média aritmética de `MetricEvent.value`.
    Average,
    /// Contagem de eventos (ignora `value`).
    Count,
    /// Soma ponderada: `Σ(value × weight_value)` / `Σ(weight_value)`.
    /// Requer `parameters.weight_field = "value"` (único peso suportado).
    WeightedAverage,
    /// Razão: contagem de eventos que satisfazem `filter_state` / total.
    /// Resultado em [0, 1].
    Ratio,
    /// Último valor recebido (por `timestamp` DESC).
    Last,
    /// Primeiro valor recebido (por `timestamp` ASC).
    First,
    /// Valor mínimo.
    Min,
    /// Valor máximo.
    Max,
}

impl AggregationKind {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sum" => Some(Self::Sum),
            "average" | "avg" => Some(Self::Average),
            "count" => Some(Self::Count),
            "weighted_average" | "weighted_avg" => Some(Self::WeightedAverage),
            "ratio" => Some(Self::Ratio),
            "last" => Some(Self::Last),
            "first" => Some(Self::First),
            "min" => Some(Self::Min),
            "max" => Some(Self::Max),
            _ => None,
        }
    }
}

// ── BasicFormulaEngine ────────────────────────────────────────────────────────

/// Implementação built-in do motor de fórmulas.
///
/// Interpreta `CalculationBinding` com os seguintes `kind`:
///
/// - `"aggregate"`: agrega `MetricEvent.value` com a função em `expression`
///   (ver `AggregationKind`). Parâmetros opcionais em `parameters`:
///   - `filter_state`: só considera eventos com `state == valor`
///   - `filter_entity_type`: só considera eventos com `entity_type == valor`
///   - `weight_field`: para `weighted_average` (actualmente só `"value"`)
///
/// - `"threshold"`: agrega com `expression`, depois compara ao `target_value`.
///   Devolve `1.0` se satisfaz, `0.0` se não satisfaz.
///   Parâmetro `operator`: `">="` (padrão), `"<="`, `">"`, `"<"`, `"=="`.
pub struct BasicFormulaEngine;

impl FormulaEngine for BasicFormulaEngine {
    fn calculate(
        &self,
        binding: &CalculationBinding,
        events: &[MetricEvent],
        target: Option<&TargetDefinition>,
    ) -> Result<f64, MetricError> {
        match binding.kind.as_str() {
            "aggregate" => aggregate(binding, events),
            "threshold" => threshold(binding, events, target),
            other => {
                tracing_warn(other);
                Err(MetricError::InvalidCriteria)
            }
        }
    }
}

// ── helpers internos ──────────────────────────────────────────────────────────

fn aggregate(binding: &CalculationBinding, events: &[MetricEvent]) -> Result<f64, MetricError> {
    let kind =
        AggregationKind::from_str(&binding.expression).ok_or(MetricError::InvalidCriteria)?;

    let filtered: Vec<&MetricEvent> = filter_events(events, binding);

    if filtered.is_empty() {
        return match kind {
            AggregationKind::Count | AggregationKind::Sum => Ok(0.0),
            AggregationKind::Ratio => Ok(0.0),
            _ => Ok(0.0),
        };
    }

    let result = match kind {
        AggregationKind::Sum => filtered.iter().map(|e| e.value).sum(),
        AggregationKind::Average => {
            let s: f64 = filtered.iter().map(|e| e.value).sum();
            s / filtered.len() as f64
        }
        AggregationKind::Count => filtered.len() as f64,
        AggregationKind::WeightedAverage => {
            // weight_field is always "value" for now; the weight of each event
            // is stored in the `value` field itself (self-weighted)
            let sum: f64 = filtered.iter().map(|e| e.value * e.value).sum();
            let total: f64 = filtered.iter().map(|e| e.value).sum();
            if total == 0.0 {
                0.0
            } else {
                sum / total
            }
        }
        AggregationKind::Ratio => {
            let filter_state = binding
                .parameters
                .as_ref()
                .and_then(|p| p.get("filter_state"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let matching = events
                .iter()
                .filter(|e| e.state.as_deref() == Some(filter_state))
                .count();
            if events.is_empty() {
                0.0
            } else {
                matching as f64 / events.len() as f64
            }
        }
        AggregationKind::Last => {
            let mut sorted = filtered.clone();
               sorted.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
            sorted[0].value
        }
        AggregationKind::First => {
            let mut sorted = filtered.clone();
               sorted.sort_by_key(|a| a.timestamp);
            sorted[0].value
        }
        AggregationKind::Min => filtered
            .iter()
            .map(|e| e.value)
            .fold(f64::INFINITY, f64::min),
        AggregationKind::Max => filtered
            .iter()
            .map(|e| e.value)
            .fold(f64::NEG_INFINITY, f64::max),
    };

    Ok(result)
}

fn threshold(
    binding: &CalculationBinding,
    events: &[MetricEvent],
    target: Option<&TargetDefinition>,
) -> Result<f64, MetricError> {
    let agg_binding = CalculationBinding {
        kind: "aggregate".to_string(),
        expression: binding.expression.clone(),
        parameters: binding.parameters.clone(),
    };
    let value = aggregate(&agg_binding, events)?;

    let target_value = target.map(|t| t.target_value).unwrap_or(0.0);

    let operator = binding
        .parameters
        .as_ref()
        .and_then(|p| p.get("operator"))
        .and_then(|v| v.as_str())
        .unwrap_or(">=");

    let met = match operator {
        ">=" => value >= target_value,
        "<=" => value <= target_value,
        ">" => value > target_value,
        "<" => value < target_value,
        "==" | "=" => (value - target_value).abs() < f64::EPSILON,
        _ => return Err(MetricError::InvalidCriteria),
    };

    Ok(if met { 1.0 } else { 0.0 })
}

fn filter_events<'a>(
    events: &'a [MetricEvent],
    binding: &CalculationBinding,
) -> Vec<&'a MetricEvent> {
    let filter_state = binding
        .parameters
        .as_ref()
        .and_then(|p| p.get("filter_state"))
        .and_then(|v| v.as_str());
    let filter_entity_type = binding
        .parameters
        .as_ref()
        .and_then(|p| p.get("filter_entity_type"))
        .and_then(|v| v.as_str());

    events
        .iter()
        .filter(|e| {
            if let Some(s) = filter_state {
                if e.state.as_deref() != Some(s) {
                    return false;
                }
            }
            if let Some(et) = filter_entity_type {
                if e.entity_type.as_deref() != Some(et) {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn tracing_warn(kind: &str) {
    // Sem dependência de tracing — apenas para depuração em debug builds.
    let _ = kind;
    #[cfg(debug_assertions)]
    eprintln!("[core-metrics] FormulaEngine: kind desconhecido '{kind}'");
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::new_event;

    fn binding(kind: &str, expression: &str) -> CalculationBinding {
        CalculationBinding {
            kind: kind.to_string(),
            expression: expression.to_string(),
            parameters: None,
        }
    }

    fn binding_with(kind: &str, expression: &str, params: serde_json::Value) -> CalculationBinding {
        CalculationBinding {
            kind: kind.to_string(),
            expression: expression.to_string(),
            parameters: Some(params),
        }
    }

    fn ev(id: &str, value: f64) -> MetricEvent {
        new_event(id, "m.test", value, Some("unit"), None)
    }

    fn ev_state(id: &str, value: f64, state: &str) -> MetricEvent {
        let mut e = ev(id, value);
        e.state = Some(state.to_string());
        e
    }

    #[test]
    fn sum() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 10.0), ev("2", 20.0), ev("3", 30.0)];
        let result = engine
            .calculate(&binding("aggregate", "sum"), &events, None)
            .unwrap();
        assert_eq!(result, 60.0);
    }

    #[test]
    fn average() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 10.0), ev("2", 30.0)];
        let result = engine
            .calculate(&binding("aggregate", "avg"), &events, None)
            .unwrap();
        assert_eq!(result, 20.0);
    }

    #[test]
    fn count() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 0.0), ev("2", 0.0), ev("3", 0.0)];
        let result = engine
            .calculate(&binding("aggregate", "count"), &events, None)
            .unwrap();
        assert_eq!(result, 3.0);
    }

    #[test]
    fn ratio_with_filter_state() {
        let engine = BasicFormulaEngine;
        let events = vec![
            ev_state("1", 1.0, "concluido"),
            ev_state("2", 1.0, "concluido"),
            ev_state("3", 1.0, "pendente"),
            ev_state("4", 1.0, "pendente"),
        ];
        let b = binding_with(
            "aggregate",
            "ratio",
            serde_json::json!({"filter_state": "concluido"}),
        );
        let result = engine.calculate(&b, &events, None).unwrap();
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn threshold_met() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 80.0), ev("2", 90.0)];
        let target = TargetDefinition {
            id: "t".to_string(),
            metric_version_id: "v".to_string(),
            scope_type: crate::target::ScopeType::Global,
            scope_id: "global".to_string(),
            target_value: 80.0,
            unit: "percent".to_string(),
            thresholds: vec![],
            valid_from: chrono::Utc::now(),
            valid_to: None,
            created_at: chrono::Utc::now(),
            created_by: "test".to_string(),
        };
        let b = binding("threshold", "avg");
        let result = engine.calculate(&b, &events, Some(&target)).unwrap();
        assert_eq!(result, 1.0); // 85 >= 80
    }

    #[test]
    fn threshold_not_met() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 60.0), ev("2", 70.0)];
        let target = TargetDefinition {
            id: "t".to_string(),
            metric_version_id: "v".to_string(),
            scope_type: crate::target::ScopeType::Global,
            scope_id: "global".to_string(),
            target_value: 80.0,
            unit: "percent".to_string(),
            thresholds: vec![],
            valid_from: chrono::Utc::now(),
            valid_to: None,
            created_at: chrono::Utc::now(),
            created_by: "test".to_string(),
        };
        let b = binding("threshold", "avg");
        let result = engine.calculate(&b, &events, Some(&target)).unwrap();
        assert_eq!(result, 0.0); // 65 < 80
    }

    #[test]
    fn min_max() {
        let engine = BasicFormulaEngine;
        let events = vec![ev("1", 5.0), ev("2", 15.0), ev("3", 10.0)];
        assert_eq!(
            engine
                .calculate(&binding("aggregate", "min"), &events, None)
                .unwrap(),
            5.0
        );
        assert_eq!(
            engine
                .calculate(&binding("aggregate", "max"), &events, None)
                .unwrap(),
            15.0
        );
    }

    #[test]
    fn empty_events_returns_zero() {
        let engine = BasicFormulaEngine;
        let result = engine
            .calculate(&binding("aggregate", "sum"), &[], None)
            .unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn unknown_kind_returns_error() {
        let engine = BasicFormulaEngine;
        assert!(engine
            .calculate(&binding("unknown", "sum"), &[], None)
            .is_err());
    }
}

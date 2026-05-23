use std::sync::{Arc, Mutex};

use crate::error::MetricError;
use crate::event::MetricEvent;

/// Interface de emissão de eventos métricos consumida por outros módulos.
pub trait MetricEmitter: Send + Sync {
    fn emit(&self, event: MetricEvent) -> Result<(), MetricError>;
}

/// Registo em memória para observabilidade técnica em processo (lab/testes).
///
/// Não é durável — dados são perdidos ao reiniciar o processo.
#[derive(Debug, Default)]
pub struct InMemoryMetricRegistry {
    events: Mutex<Vec<MetricEvent>>,
}

impl InMemoryMetricRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Devolve uma cópia de todos os eventos registados.
    pub fn snapshot(&self) -> Vec<MetricEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl MetricEmitter for InMemoryMetricRegistry {
    fn emit(&self, event: MetricEvent) -> Result<(), MetricError> {
        event.validate()?;
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

impl<T: MetricEmitter> MetricEmitter for Arc<T> {
    fn emit(&self, event: MetricEvent) -> Result<(), MetricError> {
        (**self).emit(event)
    }
}

/// Emitter composto que propaga o mesmo evento para vários destinos sequencialmente.
///
/// Útil quando o runtime precisa de manter observabilidade em memória e, em
/// paralelo, persistência canónica durável.
pub struct FanoutEmitter {
    emitters: Vec<Box<dyn MetricEmitter>>,
}

impl FanoutEmitter {
    pub fn new(emitters: Vec<Box<dyn MetricEmitter>>) -> Self {
        Self { emitters }
    }
}

impl MetricEmitter for FanoutEmitter {
    fn emit(&self, event: MetricEvent) -> Result<(), MetricError> {
        for emitter in &self.emitters {
            emitter.emit(event.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::new_event;
    use std::collections::HashMap;

    fn ev(id: &str) -> MetricEvent {
        new_event(id, "process.duration", 1.0, Some("ms"), None::<HashMap<String, String>>)
    }

    #[test]
    fn in_memory_registry_emits_and_snapshots() {
        let reg = InMemoryMetricRegistry::new();
        reg.emit(ev("m-001")).unwrap();
        reg.emit(ev("m-002")).unwrap();

        let snap = reg.snapshot();
        assert_eq!(snap.len(), 2);
    }

    #[test]
    fn snapshot_is_isolated_from_internal_state() {
        let reg = InMemoryMetricRegistry::new();
        let mut e = ev("m-001");
        e.labels = Some([("k".to_string(), "v".to_string())].into());
        reg.emit(e).unwrap();

        let mut snap = reg.snapshot();
        snap[0].labels.as_mut().unwrap().insert("k".to_string(), "mutated".to_string());

        let snap2 = reg.snapshot();
        assert_eq!(snap2[0].labels.as_ref().unwrap()["k"], "v");
    }

    #[test]
    fn emit_invalid_event_is_rejected() {
        let reg = InMemoryMetricRegistry::new();
        let mut bad = ev("m-001");
        bad.metric_code = "invalid name".to_string();
        assert!(reg.emit(bad).is_err());
        assert_eq!(reg.snapshot().len(), 0);
    }

    #[test]
    fn fanout_emitter_propagates_to_all_targets() {
        let ra = Arc::new(InMemoryMetricRegistry::new());
        let rb = Arc::new(InMemoryMetricRegistry::new());

        let fanout = FanoutEmitter::new(vec![
            Box::new(Arc::clone(&ra)),
            Box::new(Arc::clone(&rb)),
        ]);

        fanout.emit(ev("m-001")).unwrap();

        assert_eq!(ra.snapshot().len(), 1);
        assert_eq!(rb.snapshot().len(), 1);
    }

    #[test]
    fn fanout_stops_on_first_error() {
        struct AlwaysFail;
        impl MetricEmitter for AlwaysFail {
            fn emit(&self, _: MetricEvent) -> Result<(), MetricError> {
                Err(MetricError::RepoUnavailable)
            }
        }

        let reg = Arc::new(InMemoryMetricRegistry::new());
        let fanout = FanoutEmitter::new(vec![
            Box::new(AlwaysFail),
            Box::new(Arc::clone(&reg)),
        ]);

        assert_eq!(fanout.emit(ev("m-001")), Err(MetricError::RepoUnavailable));
        assert_eq!(reg.snapshot().len(), 0);
    }
}

use crate::control_category::ControlCategory;
use crate::control_definition::ControlDefinition;
use crate::control_execution::{ControlExecution, ControlExecutionResult};
use crate::control_registry::ControlRegistryStore;
use crate::error::AuditError;

/// Serviço do Registo de Controlos NORMORDIS.
///
/// `ControlRegistryService` é a fachada sobre [`ControlRegistryStore`] para
/// gestão do catálogo de controlos e registo das suas execuções.
///
/// # Enquadramento COSO
///
/// Este serviço centraliza as operações que permitem responder às perguntas
/// COSO sobre controlos:
///
/// | Pergunta COSO                                | Operação                              |
/// |----------------------------------------------|---------------------------------------|
/// | Que controlos estão definidos?               | [`list_controls`]                     |
/// | Que controlos cobre esta categoria?          | [`list_controls_by_category`]         |
/// | Este controlo existe e está activo?          | [`get_control`]                       |
/// | O controlo foi executado neste evento?       | [`list_executions_by_event`]          |
/// | Quantas vezes este controlo foi verificado?  | [`list_executions_by_control`]        |
/// | Qual a conformidade desta execução?          | [`record_execution`]                  |
///
/// [`list_controls`]: ControlRegistryService::list_controls
/// [`list_controls_by_category`]: ControlRegistryService::list_controls_by_category
/// [`get_control`]: ControlRegistryService::get_control
/// [`list_executions_by_event`]: ControlRegistryService::list_executions_by_event
/// [`list_executions_by_control`]: ControlRegistryService::list_executions_by_control
/// [`record_execution`]: ControlRegistryService::record_execution
#[derive(Debug)]
pub struct ControlRegistryService<S>
where
    S: ControlRegistryStore,
{
    store: S,
}

impl<S> ControlRegistryService<S>
where
    S: ControlRegistryStore,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    // ── Definições ────────────────────────────────────────────────────────────

    /// Cria ou actualiza a definição de um controlo no catálogo.
    ///
    /// Idempotente: pode ser chamado múltiplas vezes para o mesmo `control_id`,
    /// substituindo a definição anterior. Use `version` e `valid_from`/`valid_to`
    /// para gerir o histórico de versões.
    pub fn define_control(&self, definition: &ControlDefinition) -> Result<(), AuditError> {
        self.store.define_control(definition)
    }

    /// Devolve a definição de um controlo pelo seu identificador.
    ///
    /// Devolve `None` se o controlo não existir no catálogo.
    pub fn get_control(&self, control_id: &str) -> Result<Option<ControlDefinition>, AuditError> {
        self.store.get_control(control_id)
    }

    /// Lista todos os controlos do catálogo, com paginação.
    pub fn list_controls(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError> {
        self.store.list_controls(limit, offset)
    }

    /// Lista os controlos de uma categoria, com paginação.
    pub fn list_controls_by_category(
        &self,
        category: ControlCategory,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError> {
        self.store
            .list_controls_by_category(category, limit, offset)
    }

    // ── Execuções ─────────────────────────────────────────────────────────────

    /// Grava um registo de execução de controlo.
    ///
    /// A execução é append-only — não pode ser alterada após gravação.
    pub fn record_execution(&self, execution: &ControlExecution) -> Result<(), AuditError> {
        self.store.record_execution(execution)
    }

    /// Grava uma execução criada a partir dos parâmetros individuais.
    ///
    /// Gera o `execution_id` como UUID v4 e usa `Utc::now()` como timestamp.
    pub fn record_control_execution(
        &self,
        control_id: impl Into<String>,
        event_id: impl Into<String>,
        result: ControlExecutionResult,
        evidence_ref: Option<String>,
        notes: Option<String>,
    ) -> Result<ControlExecution, AuditError> {
        let execution = ControlExecution::new(control_id, event_id, result, evidence_ref, notes)?;
        self.store.record_execution(&execution)?;
        Ok(execution)
    }

    /// Lista as execuções de um controlo, ordenadas cronologicamente, com paginação.
    pub fn list_executions_by_control(
        &self,
        control_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlExecution>, AuditError> {
        self.store
            .list_executions_by_control(control_id, limit, offset)
    }

    /// Devolve todos os controlos verificados sobre um evento de auditoria.
    ///
    /// É a operação central para construir a vista completa de conformidade
    /// de um evento — *"que controlos foram verificados e com que resultado?"*
    pub fn list_executions_by_event(
        &self,
        event_id: &str,
    ) -> Result<Vec<ControlExecution>, AuditError> {
        self.store.list_executions_by_event(event_id)
    }

    // ── Métricas de conformidade ──────────────────────────────────────────────

    /// Calcula métricas de conformidade para um controlo.
    ///
    /// Percorre todas as execuções do controlo (sem paginação) e devolve
    /// contagens de Passed / Failed / Dispensed. Adequado para alimentar
    /// dashboards e o Balanced Scorecard.
    pub fn conformance_summary(&self, control_id: &str) -> Result<ConformanceSummary, AuditError> {
        let executions = self
            .store
            .list_executions_by_control(control_id, usize::MAX, 0)?;
        let mut summary = ConformanceSummary {
            control_id: control_id.to_string(),
            total: executions.len(),
            passed: 0,
            failed: 0,
            dispensed: 0,
        };
        for exec in &executions {
            match exec.result {
                ControlExecutionResult::Passed => summary.passed += 1,
                ControlExecutionResult::Failed => summary.failed += 1,
                ControlExecutionResult::Dispensed => summary.dispensed += 1,
            }
        }
        Ok(summary)
    }
}

/// Métricas de conformidade de um controlo.
///
/// Alimenta directamente o Balanced Scorecard NORMORDIS e os dashboards de
/// auditoria interna. A taxa de conformidade é calculada como
/// `passed / (passed + failed)` — execuções dispensadas não são incluídas
/// no denominador, pois representam decisões formais, não falhas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceSummary {
    /// Identificador do controlo.
    pub control_id: String,
    /// Total de execuções registadas.
    pub total: usize,
    /// Execuções com resultado [`Passed`].
    ///
    /// [`Passed`]: crate::control_execution::ControlExecutionResult::Passed
    pub passed: usize,
    /// Execuções com resultado [`Failed`].
    ///
    /// [`Failed`]: crate::control_execution::ControlExecutionResult::Failed
    pub failed: usize,
    /// Execuções com resultado [`Dispensed`].
    ///
    /// [`Dispensed`]: crate::control_execution::ControlExecutionResult::Dispensed
    pub dispensed: usize,
}

impl ConformanceSummary {
    /// Taxa de conformidade: `passed / (passed + failed)`.
    ///
    /// Devolve `None` se não houver execuções Passed ou Failed (divisão por zero).
    /// Execuções `Dispensed` não são incluídas no denominador.
    pub fn conformance_rate(&self) -> Option<f64> {
        let denominator = self.passed + self.failed;
        if denominator == 0 {
            return None;
        }
        Some(self.passed as f64 / denominator as f64)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use chrono::TimeZone;
    use serde_json::Value;
    use support_storage::{StorageError, StorageKey, StorageNamespace, StorageValue};

    use super::*;
    use crate::control_category::ControlSeverity;
    use crate::control_registry::{ControlRegistryConfig, StorageControlRegistryStore};

    fn service() -> ControlRegistryService<StorageControlRegistryStore<MemStorage>> {
        ControlRegistryService::new(StorageControlRegistryStore::new(
            MemStorage::default(),
            ControlRegistryConfig::default(),
        ))
    }

    fn ctrl(id: &str) -> ControlDefinition {
        ControlDefinition {
            control_id: id.to_string(),
            name: format!("Controlo {id}"),
            description: None,
            category: ControlCategory::Traceability,
            severity: ControlSeverity::High,
            owner: None,
            implemented_by: vec!["@core-audit".to_string()],
            references: vec!["COSO".to_string()],
            version: "1.0.0".to_string(),
            valid_from: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            valid_to: None,
            active: true,
        }
    }

    #[test]
    fn record_control_execution_generates_uuid() {
        let svc = service();
        let exec = svc
            .record_control_execution(
                "CTRL-TRACE-001",
                "event-1",
                ControlExecutionResult::Passed,
                None,
                None,
            )
            .unwrap();

        uuid::Uuid::parse_str(&exec.execution_id).unwrap();
    }

    #[test]
    fn conformance_summary_counts_correctly() {
        let svc = service();
        for i in 1..=3u32 {
            svc.record_control_execution(
                "CTRL-AUTH-001",
                &format!("event-{i}"),
                ControlExecutionResult::Passed,
                None,
                None,
            )
            .unwrap();
        }
        svc.record_control_execution(
            "CTRL-AUTH-001",
            "event-4",
            ControlExecutionResult::Failed,
            None,
            None,
        )
        .unwrap();
        svc.record_control_execution(
            "CTRL-AUTH-001",
            "event-5",
            ControlExecutionResult::Dispensed,
            None,
            Some("Dispensa aprovada.".to_string()),
        )
        .unwrap();

        let summary = svc.conformance_summary("CTRL-AUTH-001").unwrap();
        assert_eq!(summary.total, 5);
        assert_eq!(summary.passed, 3);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.dispensed, 1);
        // taxa: 3 / (3 + 1) = 0.75
        assert!((summary.conformance_rate().unwrap() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn conformance_rate_is_none_when_no_decisive_executions() {
        let svc = service();
        svc.record_control_execution(
            "CTRL-AUTH-001",
            "event-1",
            ControlExecutionResult::Dispensed,
            None,
            Some("Justificação formal.".to_string()),
        )
        .unwrap();

        let summary = svc.conformance_summary("CTRL-AUTH-001").unwrap();
        assert_eq!(summary.conformance_rate(), None);
    }

    #[test]
    fn list_executions_by_event_returns_all_controls() {
        let svc = service();
        svc.record_control_execution(
            "CTRL-AUTH-001",
            "event-1",
            ControlExecutionResult::Passed,
            None,
            None,
        )
        .unwrap();
        svc.record_control_execution(
            "CTRL-TRACE-001",
            "event-1",
            ControlExecutionResult::Passed,
            None,
            None,
        )
        .unwrap();

        let executions = svc.list_executions_by_event("event-1").unwrap();
        assert_eq!(executions.len(), 2);
    }

    #[test]
    fn define_and_get_control_roundtrip() {
        let svc = service();
        let def = ctrl("CTRL-TRACE-001");
        svc.define_control(&def).unwrap();

        let retrieved = svc.get_control("CTRL-TRACE-001").unwrap().unwrap();
        assert_eq!(retrieved.control_id, "CTRL-TRACE-001");
    }

    // ── Test storage ──────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Default)]
    struct MemStorage {
        inner: Arc<std::sync::Mutex<HashMap<(String, String), Value>>>,
    }

    impl support_storage::Storage for MemStorage {
        fn put_json(
            &self,
            ns: &StorageNamespace,
            key: &StorageKey,
            value: &StorageValue,
        ) -> Result<(), StorageError> {
            self.inner.lock().unwrap().insert(
                (ns.as_str().to_string(), key.as_str().to_string()),
                value.clone(),
            );
            Ok(())
        }

        fn put_json_if_absent(
            &self,
            ns: &StorageNamespace,
            key: &StorageKey,
            value: &StorageValue,
        ) -> Result<bool, StorageError> {
            let mut inner = self.inner.lock().unwrap();
            let k = (ns.as_str().to_string(), key.as_str().to_string());
            if inner.contains_key(&k) {
                return Ok(false);
            }
            inner.insert(k, value.clone());
            Ok(true)
        }

        fn get_json(
            &self,
            ns: &StorageNamespace,
            key: &StorageKey,
        ) -> Result<Option<StorageValue>, StorageError> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&(ns.as_str().to_string(), key.as_str().to_string()))
                .cloned())
        }

        fn delete(&self, ns: &StorageNamespace, key: &StorageKey) -> Result<(), StorageError> {
            self.inner
                .lock()
                .unwrap()
                .remove(&(ns.as_str().to_string(), key.as_str().to_string()));
            Ok(())
        }
    }
}

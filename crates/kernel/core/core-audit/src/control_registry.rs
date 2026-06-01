use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use support_storage::Storage;

use crate::control_category::ControlCategory;
use crate::control_definition::ControlDefinition;
use crate::control_execution::ControlExecution;
use crate::error::AuditError;

// ── Chaves internas de storage ────────────────────────────────────────────────

const DEF_PREFIX: &str = "def.";
const EXEC_PREFIX: &str = "exec.";
const BY_CTRL_PREFIX: &str = "by-ctrl.";
const BY_EVENT_PREFIX: &str = "by-event.";
const ALL_DEFS_KEY: &str = "defs.index";

// ── Configuração ──────────────────────────────────────────────────────────────

/// Configuração do [`StorageControlRegistryStore`].
///
/// Por omissão, usa o namespace `audit.controls` para ambos os tipos de registo
/// (definições e execuções), diferenciados por prefixo de chave.
#[derive(Debug, Clone)]
pub struct ControlRegistryConfig {
    pub namespace: support_storage::StorageNamespace,
}

impl ControlRegistryConfig {
    pub fn new(namespace: support_storage::StorageNamespace) -> Self {
        Self { namespace }
    }
}

impl Default for ControlRegistryConfig {
    fn default() -> Self {
        Self {
            namespace: support_storage::StorageNamespace::new(DEFAULT_CONTROL_REGISTRY_NAMESPACE)
                .expect("default control registry namespace must be valid"),
        }
    }
}

/// Namespace por omissão para o Registo de Controlos.
pub const DEFAULT_CONTROL_REGISTRY_NAMESPACE: &str = "audit.controls";

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Contrato de persistência do Registo de Controlos NORMORDIS.
///
/// # Semântica
///
/// O `ControlRegistryStore` gere dois tipos de registo:
///
/// 1. **Definições** ([`ControlDefinition`]) — o catálogo institucional dos controlos.
///    `define_control` é idempotente: cria ou substitui uma definição existente.
///    O versionamento é responsabilidade do chamador (campo `version` e `valid_from`/`valid_to`).
///
/// 2. **Execuções** ([`ControlExecution`]) — o registo imutável de que um determinado
///    controlo foi verificado sobre um determinado evento. Append-only: não é possível
///    alterar ou eliminar uma execução registada.
///
/// # Implementações disponíveis
///
/// - [`StorageControlRegistryStore`] — sobre `support_storage::Storage` genérico;
///   adequado para testes e volumes pequenos.
/// - `ControlRegistrySqliteStore` (`adapter-audit-sqlite`) — recomendado para produção.
pub trait ControlRegistryStore: Send + Sync {
    // ── Definições de controlos ───────────────────────────────────────────────

    /// Cria ou substitui a definição de um controlo no catálogo.
    ///
    /// Idempotente: se já existir um controlo com o mesmo `control_id`, é substituído.
    /// Use `valid_from`/`valid_to` e `version` para gerir o histórico de versões.
    fn define_control(&self, definition: &ControlDefinition) -> Result<(), AuditError>;

    /// Devolve a definição de um controlo pelo seu identificador.
    fn get_control(&self, control_id: &str) -> Result<Option<ControlDefinition>, AuditError>;

    /// Lista todos os controlos do catálogo, com paginação.
    fn list_controls(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError>;

    /// Lista os controlos de uma categoria, com paginação.
    fn list_controls_by_category(
        &self,
        category: ControlCategory,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError>;

    // ── Execuções de controlos ────────────────────────────────────────────────

    /// Grava um registo de execução de controlo.
    ///
    /// Append-only: falha com [`DuplicateControlExecution`] se o `execution_id` já existir.
    ///
    /// [`DuplicateControlExecution`]: AuditError::DuplicateControlExecution
    fn record_execution(&self, execution: &ControlExecution) -> Result<(), AuditError>;

    /// Lista as execuções associadas a um controlo, ordenadas por `executed_at_utc`,
    /// com paginação.
    fn list_executions_by_control(
        &self,
        control_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlExecution>, AuditError>;

    /// Devolve todas as execuções associadas a um evento de auditoria.
    ///
    /// Permite obter todos os controlos verificados num determinado evento,
    /// realizando a query inversa à de [`list_executions_by_control`].
    ///
    /// [`list_executions_by_control`]: ControlRegistryStore::list_executions_by_control
    fn list_executions_by_event(
        &self,
        event_id: &str,
    ) -> Result<Vec<ControlExecution>, AuditError>;
}

// ── Implementação genérica ────────────────────────────────────────────────────

/// Implementação sobre `support_storage::Storage` genérico.
///
/// Adequada para testes e volumes pequenos. Para produção, use
/// `ControlRegistrySqliteStore` (`adapter-audit-sqlite`).
#[derive(Debug)]
pub struct StorageControlRegistryStore<S>
where
    S: Storage,
{
    storage: S,
    config: ControlRegistryConfig,
    write_lock: Mutex<()>,
}

impl<S> StorageControlRegistryStore<S>
where
    S: Storage,
{
    pub fn new(storage: S, config: ControlRegistryConfig) -> Self {
        Self {
            storage,
            config,
            write_lock: Mutex::new(()),
        }
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn config(&self) -> &ControlRegistryConfig {
        &self.config
    }

    fn def_key(control_id: &str) -> Result<support_storage::StorageKey, AuditError> {
        let key = format!("{DEF_PREFIX}{control_id}");
        support_storage::StorageKey::new(key).map_err(|_| AuditError::OperationFailed)
    }

    fn exec_key(execution_id: &str) -> Result<support_storage::StorageKey, AuditError> {
        let key = format!("{EXEC_PREFIX}{execution_id}");
        support_storage::StorageKey::new(key).map_err(|_| AuditError::OperationFailed)
    }

    fn by_ctrl_key(control_id: &str) -> Result<support_storage::StorageKey, AuditError> {
        let hash = hex::encode(Sha256::digest(control_id.as_bytes()));
        let key = format!("{BY_CTRL_PREFIX}{hash}");
        support_storage::StorageKey::new(key).map_err(|_| AuditError::OperationFailed)
    }

    fn by_event_key(event_id: &str) -> Result<support_storage::StorageKey, AuditError> {
        let hash = hex::encode(Sha256::digest(event_id.as_bytes()));
        let key = format!("{BY_EVENT_PREFIX}{hash}");
        support_storage::StorageKey::new(key).map_err(|_| AuditError::OperationFailed)
    }

    fn all_defs_index_key() -> Result<support_storage::StorageKey, AuditError> {
        support_storage::StorageKey::new(ALL_DEFS_KEY).map_err(|_| AuditError::OperationFailed)
    }

    fn read_execution_index(
        &self,
        key: &support_storage::StorageKey,
    ) -> Result<ControlExecutionIndex, AuditError> {
        let ns = &self.config.namespace;
        let Some(value) = self.storage.get_json(ns, key)? else {
            return Ok(ControlExecutionIndex::default());
        };
        serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)
    }

    fn append_execution_index(
        &self,
        key: &support_storage::StorageKey,
        entry: ControlExecutionIndexEntry,
    ) -> Result<(), AuditError> {
        let ns = &self.config.namespace;
        let mut index = self.read_execution_index(key)?;
        index.entries.push(entry);
        index
            .entries
            .sort_by_key(|e| (e.executed_at_epoch_ms, e.execution_id.clone()));
        let value = serde_json::to_value(index).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(ns, key, &value)?;
        Ok(())
    }

    fn read_defs_index(&self) -> Result<ControlDefinitionsIndex, AuditError> {
        let ns = &self.config.namespace;
        let key = Self::all_defs_index_key()?;
        let Some(value) = self.storage.get_json(ns, &key)? else {
            return Ok(ControlDefinitionsIndex::default());
        };
        serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)
    }

    fn upsert_defs_index(&self, control_id: &str) -> Result<(), AuditError> {
        let ns = &self.config.namespace;
        let key = Self::all_defs_index_key()?;
        let mut index = self.read_defs_index()?;
        if !index.control_ids.contains(&control_id.to_string()) {
            index.control_ids.push(control_id.to_string());
            index.control_ids.sort();
        }
        let value = serde_json::to_value(index).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(ns, &key, &value)?;
        Ok(())
    }
}

impl<S> ControlRegistryStore for StorageControlRegistryStore<S>
where
    S: Storage,
{
    fn define_control(&self, definition: &ControlDefinition) -> Result<(), AuditError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| AuditError::StoreFailed)?;
        definition.validate()?;
        let ns = &self.config.namespace;
        let key = Self::def_key(&definition.control_id)?;
        let value =
            serde_json::to_value(definition).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(ns, &key, &value)?;
        self.upsert_defs_index(&definition.control_id)?;
        Ok(())
    }

    fn get_control(&self, control_id: &str) -> Result<Option<ControlDefinition>, AuditError> {
        let ns = &self.config.namespace;
        let key = Self::def_key(control_id)?;
        let Some(value) = self.storage.get_json(ns, &key)? else {
            return Ok(None);
        };
        let def: ControlDefinition =
            serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)?;
        Ok(Some(def))
    }

    fn list_controls(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError> {
        let index = self.read_defs_index()?;
        let mut defs = Vec::new();
        for control_id in index.control_ids.iter().skip(offset).take(limit) {
            if let Some(def) = self.get_control(control_id)? {
                defs.push(def);
            }
        }
        Ok(defs)
    }

    fn list_controls_by_category(
        &self,
        category: ControlCategory,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlDefinition>, AuditError> {
        let all = self.list_controls(usize::MAX, 0)?;
        Ok(all
            .into_iter()
            .filter(|d| d.category == category)
            .skip(offset)
            .take(limit)
            .collect())
    }

    fn record_execution(&self, execution: &ControlExecution) -> Result<(), AuditError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| AuditError::StoreFailed)?;
        execution.validate()?;
        let ns = &self.config.namespace;
        let exec_key = Self::exec_key(&execution.execution_id)?;
        let value =
            serde_json::to_value(execution).map_err(|_| AuditError::SerializationFailed)?;

        if !self.storage.put_json_if_absent(ns, &exec_key, &value)? {
            return Err(AuditError::DuplicateControlExecution);
        }

        let entry = ControlExecutionIndexEntry {
            execution_id: execution.execution_id.clone(),
            executed_at_epoch_ms: execution.executed_at_utc.timestamp_millis(),
        };

        let by_ctrl_key = Self::by_ctrl_key(&execution.control_id)?;
        self.append_execution_index(&by_ctrl_key, entry.clone())?;

        let by_event_key = Self::by_event_key(&execution.event_id)?;
        self.append_execution_index(&by_event_key, entry)?;

        Ok(())
    }

    fn list_executions_by_control(
        &self,
        control_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ControlExecution>, AuditError> {
        let ns = &self.config.namespace;
        let key = Self::by_ctrl_key(control_id)?;
        let index = self.read_execution_index(&key)?;
        let mut executions = Vec::new();
        for entry in index.entries.into_iter().skip(offset).take(limit) {
            let exec_key = Self::exec_key(&entry.execution_id)?;
            if let Some(value) = self.storage.get_json(ns, &exec_key)? {
                let exec: ControlExecution =
                    serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)?;
                executions.push(exec);
            }
        }
        Ok(executions)
    }

    fn list_executions_by_event(
        &self,
        event_id: &str,
    ) -> Result<Vec<ControlExecution>, AuditError> {
        let ns = &self.config.namespace;
        let key = Self::by_event_key(event_id)?;
        let index = self.read_execution_index(&key)?;
        let mut executions = Vec::new();
        for entry in index.entries {
            let exec_key = Self::exec_key(&entry.execution_id)?;
            if let Some(value) = self.storage.get_json(ns, &exec_key)? {
                let exec: ControlExecution =
                    serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)?;
                executions.push(exec);
            }
        }
        Ok(executions)
    }
}

// ── Estruturas internas de índice ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ControlDefinitionsIndex {
    control_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ControlExecutionIndex {
    entries: Vec<ControlExecutionIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ControlExecutionIndexEntry {
    execution_id: String,
    executed_at_epoch_ms: i64,
}

use sha2::{Digest, Sha256};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use chrono::TimeZone;
    use serde_json::Value;
    use support_storage::{StorageError, StorageKey, StorageNamespace, StorageValue};

    use chrono::Utc;

    use super::*;
    use crate::control_category::ControlSeverity;
    use crate::control_execution::ControlExecutionResult;

    fn store() -> StorageControlRegistryStore<MemoryStorage> {
        StorageControlRegistryStore::new(MemoryStorage::default(), ControlRegistryConfig::default())
    }

    fn auth_control(id: &str) -> ControlDefinition {
        ControlDefinition {
            control_id: id.to_string(),
            name: format!("Controlo {id}"),
            description: None,
            category: ControlCategory::Auth,
            severity: ControlSeverity::High,
            owner: None,
            implemented_by: vec!["@core-security".to_string()],
            references: vec!["COSO".to_string()],
            version: "1.0.0".to_string(),
            valid_from: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            valid_to: None,
            active: true,
        }
    }

    fn passed_exec(execution_id: &str, control_id: &str, event_id: &str) -> ControlExecution {
        ControlExecution::with_id_and_time(
            execution_id,
            control_id,
            event_id,
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            ControlExecutionResult::Passed,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn define_and_retrieve_control() {
        let store = store();
        let def = auth_control("CTRL-AUTH-001");
        store.define_control(&def).unwrap();

        let retrieved = store.get_control("CTRL-AUTH-001").unwrap().unwrap();
        assert_eq!(retrieved.name, "Controlo CTRL-AUTH-001");
    }

    #[test]
    fn define_control_is_idempotent() {
        let store = store();
        let mut def = auth_control("CTRL-AUTH-001");
        store.define_control(&def).unwrap();
        def.name = "Nome actualizado".to_string();
        store.define_control(&def).unwrap();

        let retrieved = store.get_control("CTRL-AUTH-001").unwrap().unwrap();
        assert_eq!(retrieved.name, "Nome actualizado");
    }

    #[test]
    fn get_nonexistent_control_returns_none() {
        let store = store();
        assert_eq!(store.get_control("CTRL-AUTH-999").unwrap(), None);
    }

    #[test]
    fn list_controls_paginates() {
        let store = store();
        for i in 1..=5u32 {
            store
                .define_control(&auth_control(&format!("CTRL-AUTH-{i:03}")))
                .unwrap();
        }
        let page1 = store.list_controls(2, 0).unwrap();
        let page2 = store.list_controls(2, 2).unwrap();
        let page3 = store.list_controls(2, 4).unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
    }

    #[test]
    fn list_controls_by_category_filters() {
        let store = store();
        store
            .define_control(&auth_control("CTRL-AUTH-001"))
            .unwrap();
        let mut trace = auth_control("CTRL-TRACE-001");
        trace.category = ControlCategory::Traceability;
        store.define_control(&trace).unwrap();

        let auth = store
            .list_controls_by_category(ControlCategory::Auth, 10, 0)
            .unwrap();
        let trace = store
            .list_controls_by_category(ControlCategory::Traceability, 10, 0)
            .unwrap();

        assert_eq!(auth.len(), 1);
        assert_eq!(trace.len(), 1);
    }

    #[test]
    fn record_and_retrieve_execution_by_control() {
        let store = store();
        let exec = passed_exec("exec-1", "CTRL-AUTH-001", "event-1");
        store.record_execution(&exec).unwrap();

        let executions = store
            .list_executions_by_control("CTRL-AUTH-001", 10, 0)
            .unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].execution_id, "exec-1");
    }

    #[test]
    fn duplicate_execution_is_rejected() {
        let store = store();
        let exec = passed_exec("exec-1", "CTRL-AUTH-001", "event-1");
        store.record_execution(&exec).unwrap();
        assert_eq!(
            store.record_execution(&exec).unwrap_err(),
            AuditError::DuplicateControlExecution
        );
    }

    #[test]
    fn list_executions_by_event_returns_all_controls_for_event() {
        let store = store();
        store
            .record_execution(&passed_exec("exec-1", "CTRL-AUTH-001", "event-1"))
            .unwrap();
        store
            .record_execution(&passed_exec("exec-2", "CTRL-TRACE-001", "event-1"))
            .unwrap();
        store
            .record_execution(&passed_exec("exec-3", "CTRL-AUTH-001", "event-2"))
            .unwrap();

        let for_event1 = store.list_executions_by_event("event-1").unwrap();
        let for_event2 = store.list_executions_by_event("event-2").unwrap();

        assert_eq!(for_event1.len(), 2);
        assert_eq!(for_event2.len(), 1);
    }

    #[test]
    fn list_executions_by_control_paginates() {
        let store = store();
        for i in 1..=4u32 {
            store
                .record_execution(&passed_exec(
                    &format!("exec-{i}"),
                    "CTRL-AUTH-001",
                    &format!("event-{i}"),
                ))
                .unwrap();
        }
        let page1 = store
            .list_executions_by_control("CTRL-AUTH-001", 2, 0)
            .unwrap();
        let page2 = store
            .list_executions_by_control("CTRL-AUTH-001", 2, 2)
            .unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
    }

    // ── Test storage ──────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Default)]
    struct MemoryStorage {
        inner: Arc<std::sync::Mutex<HashMap<(String, String), Value>>>,
    }

    impl Storage for MemoryStorage {
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

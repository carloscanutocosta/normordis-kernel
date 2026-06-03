#![allow(deprecated)]

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use support_storage::Storage;

use crate::emitter::MetricEmitter;
use crate::error::MetricError;
use crate::event::MetricEvent;

pub const DEFAULT_NAMESPACE: &str = "metrics.events";
pub const DEFAULT_LIST_LIMIT: usize = 100;
pub const MAX_LIST_LIMIT: usize = 1000;

/// Contrato de persistência canónica de eventos métricos.
///
/// Semântica garantida:
/// - `id` é único no repositório
/// - ordenação de listagem por `timestamp` descendente
/// - `MetricEvent` é imutável após persistência
pub trait MetricStore: Send + Sync {
    fn save(&self, event: MetricEvent) -> Result<(), MetricError>;
    fn get_by_id(&self, id: &str) -> Result<MetricEvent, MetricError>;
    fn list(&self, criteria: &MetricListCriteria) -> Result<Vec<MetricEvent>, MetricError>;
}

/// Filtros de consulta para eventos métricos persistidos.
#[derive(Debug, Clone, Default)]
pub struct MetricListCriteria {
    pub metric_code: Option<String>,
    pub metric_version_id: Option<String>,
    pub evaluation_cycle_id: Option<String>,
    pub correlation_id: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub state: Option<String>,
    pub org_unit_id: Option<String>,
    pub source_app: Option<String>,
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,
    pub limit: usize,
    pub offset: usize,
}

impl MetricListCriteria {
    pub fn validate(&self) -> Result<(), MetricError> {
        if self.limit > MAX_LIST_LIMIT {
            return Err(MetricError::InvalidCriteria);
        }
        if let (Some(from), Some(to)) = (self.time_from, self.time_to) {
            if to < from {
                return Err(MetricError::InvalidCriteria);
            }
        }
        Ok(())
    }

    fn effective_limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_LIST_LIMIT
        } else {
            self.limit
        }
    }
}

/// Implementação de `MetricStore` sobre `support-storage`.
///
/// Eventos individuais são armazenados em `events/{id}`.
/// Um índice compacto em `events.index` suporta filtragem e paginação
/// sem carregar o payload completo de cada evento.
///
/// **Depreciado**: use `metrics-sqlite` (`MetricsSqliteStore`) em vez disto.
/// `StorageMetricStore` não suporta os stores de governação e será removido
/// numa versão futura.
#[deprecated(
    since = "0.4.0",
    note = "Use metrics-sqlite::MetricsSqliteStore instead"
)]
pub struct StorageMetricStore<S: Storage> {
    storage: S,
    namespace: support_storage::StorageNamespace,
    write_lock: Mutex<()>,
}

impl<S: Storage> StorageMetricStore<S> {
    pub fn new(storage: S, namespace: support_storage::StorageNamespace) -> Self {
        Self {
            storage,
            namespace,
            write_lock: Mutex::new(()),
        }
    }

    pub fn with_default_namespace(storage: S) -> Result<Self, MetricError> {
        let namespace = support_storage::StorageNamespace::new(DEFAULT_NAMESPACE)
            .map_err(|_| MetricError::RepoUnavailable)?;
        Ok(Self::new(storage, namespace))
    }
}

/// Entrada no índice compacto — contém os campos filtráveis sem o payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    metric_code: String,
    timestamp_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    metric_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation_cycle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    org_unit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_app: Option<String>,
    event_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Index {
    entries: Vec<IndexEntry>,
}

const INDEX_KEY: &str = "events.index";

fn event_storage_key(id: &str) -> Result<support_storage::StorageKey, MetricError> {
    support_storage::StorageKey::new(format!("event.{id}")).map_err(|_| MetricError::MarshalFailed)
}

fn index_storage_key() -> Result<support_storage::StorageKey, MetricError> {
    support_storage::StorageKey::new(INDEX_KEY).map_err(|_| MetricError::RepoUnavailable)
}

impl<S: Storage> StorageMetricStore<S> {
    fn read_index(&self) -> Result<Index, MetricError> {
        let key = index_storage_key()?;
        let Some(value) = self.storage.get_json(&self.namespace, &key)? else {
            return Ok(Index::default());
        };
        serde_json::from_value(value).map_err(|_| MetricError::DataCorruption)
    }

    fn write_index(&self, index: &Index) -> Result<(), MetricError> {
        let key = index_storage_key()?;
        let value = serde_json::to_value(index).map_err(|_| MetricError::MarshalFailed)?;
        self.storage.put_json(&self.namespace, &key, &value)?;
        Ok(())
    }
}

impl<S: Storage> MetricStore for StorageMetricStore<S> {
    fn save(&self, event: MetricEvent) -> Result<(), MetricError> {
        event.validate()?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| MetricError::RepoUnavailable)?;
        let key = event_storage_key(&event.id)?;
        let value = serde_json::to_value(&event).map_err(|_| MetricError::MarshalFailed)?;
        if !self
            .storage
            .put_json_if_absent(&self.namespace, &key, &value)?
        {
            return Err(MetricError::Conflict);
        }
        let mut index = self.read_index()?;
        index.entries.push(IndexEntry {
            id: event.id.clone(),
            metric_code: event.metric_code.clone(),
            timestamp_ms: event.timestamp.timestamp_millis(),
            metric_version_id: event.metric_version_id.clone(),
            evaluation_cycle_id: event.evaluation_cycle_id.clone(),
            correlation_id: event.correlation_id.clone(),
            entity_type: event.entity_type.clone(),
            entity_id: event.entity_id.clone(),
            state: event.state.clone(),
            org_unit_id: event.org_unit_id.clone(),
            source_app: event.source_app.clone(),
            event_key: key.as_str().to_string(),
        });
        index
            .entries
               .sort_by_key(|b| std::cmp::Reverse(b.timestamp_ms));
        self.write_index(&index)?;
        Ok(())
    }

    fn get_by_id(&self, id: &str) -> Result<MetricEvent, MetricError> {
        let key = event_storage_key(id)?;
        let Some(value) = self.storage.get_json(&self.namespace, &key)? else {
            return Err(MetricError::NotFound);
        };
        serde_json::from_value(value).map_err(|_| MetricError::DataCorruption)
    }

    fn list(&self, criteria: &MetricListCriteria) -> Result<Vec<MetricEvent>, MetricError> {
        criteria.validate()?;
        let index = self.read_index()?;
        let limit = criteria.effective_limit();
        let matching: Vec<&IndexEntry> = index
            .entries
            .iter()
            .filter(|e| matches_criteria(e, criteria))
            .collect();
        let page = matching.into_iter().skip(criteria.offset).take(limit);
        let mut events = Vec::new();
        for entry in page {
            let key = support_storage::StorageKey::new(&entry.event_key)
                .map_err(|_| MetricError::DataCorruption)?;
            let Some(value) = self.storage.get_json(&self.namespace, &key)? else {
                return Err(MetricError::DataCorruption);
            };
            let event: MetricEvent =
                serde_json::from_value(value).map_err(|_| MetricError::DataCorruption)?;
            events.push(event);
        }
        Ok(events)
    }
}

fn matches_criteria(entry: &IndexEntry, c: &MetricListCriteria) -> bool {
    if let Some(code) = &c.metric_code {
        if &entry.metric_code != code {
            return false;
        }
    }
    if let Some(vid) = &c.metric_version_id {
        if entry.metric_version_id.as_deref() != Some(vid.as_str()) {
            return false;
        }
    }
    if let Some(cid_cycle) = &c.evaluation_cycle_id {
        if entry.evaluation_cycle_id.as_deref() != Some(cid_cycle.as_str()) {
            return false;
        }
    }
    if let Some(cid) = &c.correlation_id {
        if entry.correlation_id.as_deref() != Some(cid.as_str()) {
            return false;
        }
    }
    if let Some(et) = &c.entity_type {
        if entry.entity_type.as_deref() != Some(et.as_str()) {
            return false;
        }
    }
    if let Some(eid) = &c.entity_id {
        if entry.entity_id.as_deref() != Some(eid.as_str()) {
            return false;
        }
    }
    if let Some(st) = &c.state {
        if entry.state.as_deref() != Some(st.as_str()) {
            return false;
        }
    }
    if let Some(ou) = &c.org_unit_id {
        if entry.org_unit_id.as_deref() != Some(ou.as_str()) {
            return false;
        }
    }
    if let Some(sa) = &c.source_app {
        if entry.source_app.as_deref() != Some(sa.as_str()) {
            return false;
        }
    }
    if let Some(from) = c.time_from {
        let ts =
            DateTime::from_timestamp_millis(entry.timestamp_ms).unwrap_or(DateTime::<Utc>::MIN_UTC);
        if ts < from {
            return false;
        }
    }
    if let Some(to) = c.time_to {
        let ts =
            DateTime::from_timestamp_millis(entry.timestamp_ms).unwrap_or(DateTime::<Utc>::MIN_UTC);
        if ts > to {
            return false;
        }
    }
    true
}

/// Adapta um `MetricStore` à interface `MetricEmitter`.
///
/// Permite que boundaries que só conhecem `MetricEmitter` persistam
/// eventos métricos de forma canónica.
pub struct StoreEmitter {
    store: Box<dyn MetricStore>,
}

impl StoreEmitter {
    pub fn new(store: Box<dyn MetricStore>) -> Self {
        Self { store }
    }
}

impl MetricEmitter for StoreEmitter {
    fn emit(&self, event: MetricEvent) -> Result<(), MetricError> {
        self.store.save(event)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use support_storage::{StorageError, StorageKey, StorageNamespace, StorageValue};

    use super::*;
    use crate::event::new_event;

    fn store() -> StorageMetricStore<MemStorage> {
        StorageMetricStore::with_default_namespace(MemStorage::default()).unwrap()
    }

    fn ev(id: &str) -> MetricEvent {
        let mut e = new_event(
            id,
            "process.duration",
            1.0,
            Some("ms"),
            None::<HashMap<String, String>>,
        );
        e.timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap();
        e
    }

    #[test]
    fn save_and_get_by_id() {
        let s = store();
        s.save(ev("m-001")).unwrap();
        let got = s.get_by_id("m-001").unwrap();
        assert_eq!(got.id, "m-001");
    }

    #[test]
    fn duplicate_save_returns_conflict() {
        let s = store();
        s.save(ev("m-001")).unwrap();
        assert_eq!(s.save(ev("m-001")), Err(MetricError::Conflict));
    }

    #[test]
    fn get_by_id_missing_returns_not_found() {
        let s = store();
        assert_eq!(s.get_by_id("unknown"), Err(MetricError::NotFound));
    }

    #[test]
    fn list_returns_all_by_default() {
        let s = store();
        for i in 0..3u32 {
            let mut e = ev(&format!("m-{i:03}"));
            e.timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 10, i, 0).unwrap();
            s.save(e).unwrap();
        }
        let results = s.list(&MetricListCriteria::default()).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn list_is_ordered_by_timestamp_descending() {
        let s = store();
        let mut e1 = ev("m-001");
        e1.timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap();
        let mut e2 = ev("m-002");
        e2.timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 11, 0, 0).unwrap();
        s.save(e1).unwrap();
        s.save(e2).unwrap();

        let results = s.list(&MetricListCriteria::default()).unwrap();
        assert_eq!(results[0].id, "m-002");
        assert_eq!(results[1].id, "m-001");
    }

    #[test]
    fn list_filters_by_metric_code() {
        let s = store();
        let mut e1 = ev("m-001");
        e1.metric_code = "process.duration".to_string();
        let mut e2 = ev("m-002");
        e2.metric_code = "document.count".to_string();
        s.save(e1).unwrap();
        s.save(e2).unwrap();

        let results = s
            .list(&MetricListCriteria {
                metric_code: Some("document.count".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m-002");
    }

    #[test]
    fn list_filters_by_metric_version_and_cycle() {
        let s = store();
        let mut e1 = ev("m-001");
        e1.metric_version_id = Some("mv-001".to_string());
        e1.evaluation_cycle_id = Some("cycle-siadap-2026".to_string());
        let mut e2 = ev("m-002");
        e2.metric_version_id = Some("mv-002".to_string());
        s.save(e1).unwrap();
        s.save(e2).unwrap();

        let results = s
            .list(&MetricListCriteria {
                evaluation_cycle_id: Some("cycle-siadap-2026".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m-001");
    }

    #[test]
    fn list_filters_by_org_unit_id() {
        let s = store();
        let mut e1 = ev("m-001");
        e1.org_unit_id = Some("uo:porto".to_string());
        let mut e2 = ev("m-002");
        e2.org_unit_id = Some("uo:lisboa".to_string());
        s.save(e1).unwrap();
        s.save(e2).unwrap();

        let results = s
            .list(&MetricListCriteria {
                org_unit_id: Some("uo:porto".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m-001");
    }

    #[test]
    fn list_respects_limit_and_offset() {
        let s = store();
        for i in 0..5u32 {
            let mut e = ev(&format!("m-{i:03}"));
            e.timestamp = Utc.with_ymd_and_hms(2026, 5, 17, 10, i, 0).unwrap();
            s.save(e).unwrap();
        }
        let results = s
            .list(&MetricListCriteria {
                limit: 2,
                offset: 1,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn invalid_criteria_limit_is_rejected() {
        let s = store();
        let err = s
            .list(&MetricListCriteria {
                limit: MAX_LIST_LIMIT + 1,
                ..Default::default()
            })
            .unwrap_err();
        assert_eq!(err, MetricError::InvalidCriteria);
    }

    #[test]
    fn time_range_inverted_is_rejected() {
        let criteria = MetricListCriteria {
            time_from: Some(Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap()),
            time_to: Some(Utc.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap()),
            ..Default::default()
        };
        assert_eq!(criteria.validate(), Err(MetricError::InvalidCriteria));
    }

    #[test]
    fn store_emitter_saves_via_store() {
        let inner = StorageMetricStore::with_default_namespace(MemStorage::default()).unwrap();
        let emitter = StoreEmitter::new(Box::new(inner));
        emitter.emit(ev("m-001")).unwrap();
    }

    // ── in-process memory Storage para testes ────────────────────────────────
    #[derive(Debug, Clone, Default)]
    struct MemStorage {
        inner: Arc<std::sync::Mutex<HashMap<(String, String), StorageValue>>>,
    }

    impl Storage for MemStorage {
        fn put_json(
            &self,
            namespace: &StorageNamespace,
            key: &StorageKey,
            value: &StorageValue,
        ) -> Result<(), StorageError> {
            self.inner.lock().unwrap().insert(
                (namespace.as_str().to_string(), key.as_str().to_string()),
                value.clone(),
            );
            Ok(())
        }

        fn put_json_if_absent(
            &self,
            namespace: &StorageNamespace,
            key: &StorageKey,
            value: &StorageValue,
        ) -> Result<bool, StorageError> {
            let mut inner = self.inner.lock().unwrap();
            let k = (namespace.as_str().to_string(), key.as_str().to_string());
            if inner.contains_key(&k) {
                return Ok(false);
            }
            inner.insert(k, value.clone());
            Ok(true)
        }

        fn get_json(
            &self,
            namespace: &StorageNamespace,
            key: &StorageKey,
        ) -> Result<Option<StorageValue>, StorageError> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&(namespace.as_str().to_string(), key.as_str().to_string()))
                .cloned())
        }

        fn delete(
            &self,
            namespace: &StorageNamespace,
            key: &StorageKey,
        ) -> Result<(), StorageError> {
            self.inner
                .lock()
                .unwrap()
                .remove(&(namespace.as_str().to_string(), key.as_str().to_string()));
            Ok(())
        }
    }
}

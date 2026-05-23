use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use support_storage::Storage;

use crate::chain::{
    compute_manifest_hash, compute_record_hash, AuditChainIndex, AuditChainIndexEntry,
    AuditChainLink, AuditChainReport, AuditChainState, AuditExportManifest, AUDIT_CHAIN_HEAD_KEY,
    AUDIT_CHAIN_INDEX_KEY,
};
use crate::config::AuditStoreConfig;
use crate::error::AuditError;
use crate::event::AuditEvent;
use crate::query::{
    audit_actor_index_key, audit_event_key, audit_event_lookup_key, audit_target_index_key,
};
use crate::target::AuditTarget;

pub trait AuditStore: Send + Sync {
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;

    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError>;

    fn list_by_actor(&self, actor_id: &str) -> Result<Vec<AuditEvent>, AuditError>;

    fn list_by_target(&self, target: &AuditTarget) -> Result<Vec<AuditEvent>, AuditError>;

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError>;

    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError>;
}

#[derive(Debug)]
pub struct StorageAuditStore<S>
where
    S: Storage,
{
    storage: S,
    config: AuditStoreConfig,
    write_lock: Mutex<()>,
}

impl<S> StorageAuditStore<S>
where
    S: Storage,
{
    pub fn new(storage: S, config: AuditStoreConfig) -> Self {
        Self {
            storage,
            config,
            write_lock: Mutex::new(()),
        }
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn config(&self) -> &AuditStoreConfig {
        &self.config
    }
}

impl<S> AuditStore for StorageAuditStore<S>
where
    S: Storage,
{
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| AuditError::StoreFailed)?;
        event.validate()?;
        let namespace = &self.config.namespace;
        let event_key = audit_event_key(event)?;
        let lookup_key = audit_event_lookup_key(&event.event_id)?;
        let chain_head_key = support_storage::StorageKey::new(AUDIT_CHAIN_HEAD_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_state = self.read_chain_state(namespace, &chain_head_key)?;
        let sequence = chain_state.sequence + 1;
        let previous_record_hash = chain_state.head_record_hash.clone();
        let record_hash = compute_record_hash(event, sequence, previous_record_hash.as_deref())?;

        let record = AuditEventRecord {
            schema_version: 1,
            event: event.clone(),
            chain_link: AuditChainLink {
                sequence,
                previous_record_hash: previous_record_hash.clone(),
                record_hash: record_hash.clone(),
            },
        };
        let value = serde_json::to_value(record).map_err(|_| AuditError::SerializationFailed)?;
        let lookup = serde_json::to_value(AuditEventLookup {
            event_key: event_key.as_str().to_string(),
        })
        .map_err(|_| AuditError::SerializationFailed)?;

        if !self
            .storage
            .put_json_if_absent(namespace, &event_key, &value)?
        {
            return Err(AuditError::DuplicateEvent);
        }
        if !self
            .storage
            .put_json_if_absent(namespace, &lookup_key, &lookup)?
        {
            return Err(AuditError::DuplicateEvent);
        }
        self.append_index(
            namespace,
            audit_actor_index_key(&event.actor.actor_id)?,
            event,
        )?;
        self.append_index(
            namespace,
            audit_target_index_key(&event.target.target_type, &event.target.target_id)?,
            event,
        )?;
        self.append_chain_index(
            namespace,
            &chain_index_key,
            AuditChainIndexEntry {
                event_id: event.event_id.clone(),
                event_key: event_key.as_str().to_string(),
                sequence,
                record_hash: record_hash.clone(),
            },
        )?;
        let next_state = AuditChainState {
            schema_version: 1,
            sequence,
            head_event_id: Some(event.event_id.clone()),
            head_record_hash: Some(record_hash),
        };
        let value =
            serde_json::to_value(next_state).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(namespace, &chain_head_key, &value)?;
        Ok(())
    }

    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError> {
        let namespace = &self.config.namespace;
        let lookup_key = audit_event_lookup_key(event_id)?;
        let Some(lookup_value) = self.storage.get_json(namespace, &lookup_key)? else {
            return Ok(None);
        };
        let lookup: AuditEventLookup =
            serde_json::from_value(lookup_value).map_err(|_| AuditError::DeserializationFailed)?;
        let event_key = support_storage::StorageKey::new(lookup.event_key)
            .map_err(|_| AuditError::StoreFailed)?;
        let Some(value) = self.storage.get_json(namespace, &event_key)? else {
            return Ok(None);
        };
        read_record_value(value).map(Some)
    }

    fn list_by_actor(&self, actor_id: &str) -> Result<Vec<AuditEvent>, AuditError> {
        if actor_id.trim().is_empty() || actor_id != actor_id.trim() {
            return Err(AuditError::InvalidActor);
        }
        let namespace = &self.config.namespace;
        let key = audit_actor_index_key(actor_id)?;
        let entries = self.read_index(namespace, &key)?;
        self.read_index_events(namespace, entries)
    }

    fn list_by_target(&self, target: &AuditTarget) -> Result<Vec<AuditEvent>, AuditError> {
        target.validate()?;
        let namespace = &self.config.namespace;
        let key = audit_target_index_key(&target.target_type, &target.target_id)?;
        let entries = self.read_index(namespace, &key)?;
        self.read_index_events(namespace, entries)
    }

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError> {
        let namespace = &self.config.namespace;
        let chain_head_key = support_storage::StorageKey::new(AUDIT_CHAIN_HEAD_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_state = self.read_chain_state(namespace, &chain_head_key)?;
        let chain_index = self.read_chain_index(namespace, &chain_index_key)?;
        let mut previous_hash: Option<String> = None;

        for (expected_index, entry) in chain_index.entries.iter().enumerate() {
            let key = support_storage::StorageKey::new(&entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            let Some(value) = self.storage.get_json(namespace, &key)? else {
                return Err(AuditError::ChainVerificationFailed);
            };
            let record = read_record(value)?;
            let expected_sequence = expected_index as u64 + 1;
            if record.chain_link.sequence != expected_sequence
                || record.chain_link.previous_record_hash != previous_hash
                || record.chain_link.record_hash != entry.record_hash
            {
                return Err(AuditError::ChainVerificationFailed);
            }
            previous_hash = Some(record.chain_link.record_hash);
        }

        if chain_state.sequence as usize != chain_index.entries.len()
            || chain_state.head_record_hash != previous_hash
        {
            return Err(AuditError::ChainVerificationFailed);
        }

        Ok(AuditChainReport {
            checked_events: chain_index.entries.len(),
            head_record_hash: chain_state.head_record_hash,
        })
    }

    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError> {
        let report = self.verify_chain()?;
        let manifest_hash =
            compute_manifest_hash(report.checked_events, report.head_record_hash.as_deref())?;
        Ok(AuditExportManifest {
            schema_version: 1,
            generated_at_utc: chrono::Utc::now(),
            events_count: report.checked_events,
            head_record_hash: report.head_record_hash,
            manifest_hash,
        })
    }
}

impl<S> StorageAuditStore<S>
where
    S: Storage,
{
    fn append_index(
        &self,
        namespace: &support_storage::StorageNamespace,
        key: support_storage::StorageKey,
        event: &AuditEvent,
    ) -> Result<(), AuditError> {
        let mut index = self.read_index(namespace, &key)?;
        index.entries.push(AuditIndexEntry {
            event_id: event.event_id.clone(),
            event_key: audit_event_key(event)?.as_str().to_string(),
            occurred_at_epoch_ms: event.occurred_at_utc.timestamp_millis(),
        });
        index
            .entries
            .sort_by_key(|entry| (entry.occurred_at_epoch_ms, entry.event_id.clone()));
        let value = serde_json::to_value(index).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(namespace, &key, &value)?;
        Ok(())
    }

    fn append_chain_index(
        &self,
        namespace: &support_storage::StorageNamespace,
        key: &support_storage::StorageKey,
        entry: AuditChainIndexEntry,
    ) -> Result<(), AuditError> {
        let mut index = self.read_chain_index(namespace, key)?;
        index.entries.push(entry);
        index.entries.sort_by_key(|entry| entry.sequence);
        let value = serde_json::to_value(index).map_err(|_| AuditError::SerializationFailed)?;
        self.storage.put_json(namespace, key, &value)?;
        Ok(())
    }

    fn read_chain_state(
        &self,
        namespace: &support_storage::StorageNamespace,
        key: &support_storage::StorageKey,
    ) -> Result<AuditChainState, AuditError> {
        let Some(value) = self.storage.get_json(namespace, key)? else {
            return Ok(AuditChainState {
                schema_version: 1,
                ..AuditChainState::default()
            });
        };
        let state: AuditChainState =
            serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)?;
        if state.schema_version != 1 {
            return Err(AuditError::ChainVerificationFailed);
        }
        Ok(state)
    }

    fn read_chain_index(
        &self,
        namespace: &support_storage::StorageNamespace,
        key: &support_storage::StorageKey,
    ) -> Result<AuditChainIndex, AuditError> {
        let Some(value) = self.storage.get_json(namespace, key)? else {
            return Ok(AuditChainIndex::default());
        };
        serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)
    }

    fn read_index(
        &self,
        namespace: &support_storage::StorageNamespace,
        key: &support_storage::StorageKey,
    ) -> Result<AuditIndex, AuditError> {
        let Some(value) = self.storage.get_json(namespace, key)? else {
            return Ok(AuditIndex::default());
        };
        serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)
    }

    fn read_index_events(
        &self,
        namespace: &support_storage::StorageNamespace,
        index: AuditIndex,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        let mut events = Vec::with_capacity(index.entries.len());
        for entry in index.entries {
            let key = support_storage::StorageKey::new(entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            if let Some(value) = self.storage.get_json(namespace, &key)? {
                events.push(read_record_value(value)?);
            }
        }
        Ok(events)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AuditEventRecord {
    schema_version: u32,
    event: AuditEvent,
    chain_link: AuditChainLink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuditEventLookup {
    event_key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct AuditIndex {
    entries: Vec<AuditIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuditIndexEntry {
    event_id: String,
    event_key: String,
    occurred_at_epoch_ms: i64,
}

fn read_record_value(value: Value) -> Result<AuditEvent, AuditError> {
    Ok(read_record(value)?.event)
}

fn read_record(value: Value) -> Result<AuditEventRecord, AuditError> {
    let record: AuditEventRecord =
        serde_json::from_value(value).map_err(|_| AuditError::DeserializationFailed)?;
    let expected_hash = compute_record_hash(
        &record.event,
        record.chain_link.sequence,
        record.chain_link.previous_record_hash.as_deref(),
    )?;
    if record.schema_version != 1 || record.chain_link.record_hash != expected_hash {
        return Err(AuditError::IntegrityFailed);
    }
    record.event.validate()?;
    Ok(record)
}

#[allow(dead_code)]
fn _assert_storage_value_is_json(value: Value) -> Value {
    value
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use support_storage::{StorageError, StorageKey, StorageNamespace, StorageValue};

    use super::*;
    use crate::{AuditActor, AuditTarget, DEFAULT_AUDIT_EVENTS_NAMESPACE, STORE_FAILED};

    fn storage_store() -> StorageAuditStore<JsonMemoryStorage> {
        StorageAuditStore::new(JsonMemoryStorage::default(), AuditStoreConfig::default())
    }

    fn configured_store(namespace: &str) -> StorageAuditStore<JsonMemoryStorage> {
        StorageAuditStore::new(
            JsonMemoryStorage::default(),
            AuditStoreConfig::new(StorageNamespace::new(namespace).unwrap()),
        )
    }

    fn event() -> AuditEvent {
        AuditEvent::with_id_and_time(
            "event-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            Some(json!({"ip":"127.0.0.1","ok":true})),
        )
        .unwrap()
    }

    #[test]
    fn storage_audit_store_records_via_support_storage() {
        let store = storage_store();
        store.record(&event()).unwrap();

        assert_eq!(store.storage().len(), 6);
    }

    #[test]
    fn configurable_namespace_is_used() {
        let store = configured_store("audit.custom");
        let event = event();
        let event_key = audit_event_key(&event).unwrap();

        store.record(&event).unwrap();

        assert!(store
            .storage()
            .contains(&StorageNamespace::new("audit.custom").unwrap(), &event_key));
        assert_eq!(store.get("event-1").unwrap(), Some(event));
    }

    #[test]
    fn default_namespace_preserves_compatibility() {
        let store = storage_store();
        let event = event();
        let event_key = audit_event_key(&event).unwrap();
        let namespace = StorageNamespace::new(DEFAULT_AUDIT_EVENTS_NAMESPACE).unwrap();

        store.record(&event).unwrap();

        assert_eq!(
            store.config().namespace.as_str(),
            DEFAULT_AUDIT_EVENTS_NAMESPACE
        );
        assert!(store.storage().contains(&namespace, &event_key));
    }

    #[test]
    fn stores_with_different_namespaces_do_not_collide() {
        let storage = JsonMemoryStorage::default();
        let first = StorageAuditStore::new(
            storage.clone(),
            AuditStoreConfig::new(StorageNamespace::new("audit.first").unwrap()),
        );
        let second = StorageAuditStore::new(
            storage,
            AuditStoreConfig::new(StorageNamespace::new("audit.second").unwrap()),
        );
        let event = event();

        first.record(&event).unwrap();
        second.record(&event).unwrap();

        assert_eq!(first.get("event-1").unwrap(), Some(event.clone()));
        assert_eq!(second.get("event-1").unwrap(), Some(event));
        assert_eq!(first.storage().len(), 12);
    }

    #[test]
    fn invalid_namespace_is_rejected_before_config_construction() {
        assert_eq!(
            StorageNamespace::new("audit events").unwrap_err(),
            StorageError::InvalidNamespace
        );
        assert_eq!(
            StorageNamespace::new("audit:events").unwrap_err(),
            StorageError::InvalidNamespace
        );
    }

    #[test]
    fn storage_audit_store_reads_recorded_event() {
        let store = storage_store();
        let event = event();

        store.record(&event).unwrap();

        assert_eq!(store.get("event-1").unwrap(), Some(event));
    }

    #[test]
    fn details_json_is_preserved() {
        let store = storage_store();
        let event = event();

        store.record(&event).unwrap();
        let stored = store.get("event-1").unwrap().unwrap();

        assert_eq!(
            stored.details_json,
            Some(json!({"ip":"127.0.0.1","ok":true}))
        );
    }

    #[test]
    fn duplicate_event_is_rejected() {
        let store = storage_store();
        let event = event();

        store.record(&event).unwrap();
        let err = store.record(&event).unwrap_err();

        assert_eq!(err, AuditError::DuplicateEvent);
    }

    #[test]
    fn list_by_actor_returns_events_in_time_order() {
        let store = storage_store();
        let first = event();
        let second = AuditEvent::with_id_and_time(
            "event-2",
            "document.updated",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-2").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 1, 0).unwrap(),
            None,
        )
        .unwrap();

        store.record(&second).unwrap();
        store.record(&first).unwrap();

        let events = store.list_by_actor("user-1").unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            vec!["event-1", "event-2"]
        );
    }

    #[test]
    fn list_by_target_returns_matching_events() {
        let store = storage_store();
        let target = AuditTarget::new("document", "doc-1").unwrap();
        let first = event();
        let other = AuditEvent::with_id_and_time(
            "event-2",
            "document.updated",
            AuditActor::new("user-2").unwrap(),
            AuditTarget::new("document", "doc-2").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 1, 0).unwrap(),
            None,
        )
        .unwrap();

        store.record(&first).unwrap();
        store.record(&other).unwrap();

        let events = store.list_by_target(&target).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "event-1");
    }

    #[test]
    fn tampered_record_fails_integrity_check() {
        let store = storage_store();
        let event = event();
        let namespace = &store.config().namespace;
        let event_key = audit_event_key(&event).unwrap();

        store.record(&event).unwrap();
        store.storage().mutate(namespace, &event_key, |value| {
            value["event"]["event_type"] = json!("document.deleted");
        });

        assert_eq!(
            store.get("event-1").unwrap_err(),
            AuditError::IntegrityFailed
        );
    }

    #[test]
    fn verifies_hash_chain() {
        let store = storage_store();
        let first = event();
        let second = AuditEvent::with_id_and_time(
            "event-2",
            "document.updated",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 1, 0).unwrap(),
            None,
        )
        .unwrap();

        store.record(&first).unwrap();
        store.record(&second).unwrap();

        let report = store.verify_chain().unwrap();
        assert_eq!(report.checked_events, 2);
        assert!(report.head_record_hash.is_some());
    }

    #[test]
    fn export_manifest_reports_verified_chain_head() {
        let store = storage_store();
        store.record(&event()).unwrap();

        let manifest = store.export_manifest().unwrap();

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.events_count, 1);
        assert!(manifest.head_record_hash.is_some());
        assert!(!manifest.manifest_hash.is_empty());
    }

    #[test]
    fn event_record_serialization_shape_is_preserved() {
        let store = storage_store();
        let event = event();
        let event_key = audit_event_key(&event).unwrap();

        store.record(&event).unwrap();
        let value = store
            .storage()
            .get_raw(&store.config().namespace, &event_key)
            .unwrap();

        assert_eq!(value["schema_version"], json!(1));
        assert_eq!(value["event"]["event_id"], json!("event-1"));
        assert_eq!(value["event"]["event_type"], json!("document.created"));
        assert_eq!(value["chain_link"]["sequence"], json!(1));
    }

    #[test]
    fn concurrent_recording_preserves_all_events() {
        let store = Arc::new(storage_store());
        let mut handles = Vec::new();

        for index in 0..32 {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let event = AuditEvent::with_id_and_time(
                    format!("event-{index}"),
                    "document.updated",
                    AuditActor::new("user-concurrent").unwrap(),
                    AuditTarget::new("document", format!("doc-{index}")).unwrap(),
                    Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
                    Some(json!({"index": index})),
                )
                .unwrap();
                store.record(&event).unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(store.list_by_actor("user-concurrent").unwrap().len(), 32);
    }

    #[derive(Debug)]
    struct FailingStorage {
        calls: Mutex<usize>,
    }

    impl FailingStorage {
        fn new() -> Self {
            Self {
                calls: Mutex::new(0),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    struct JsonMemoryStorage {
        inner: Arc<Mutex<HashMap<(String, String), StorageValue>>>,
    }

    impl JsonMemoryStorage {
        fn len(&self) -> usize {
            self.inner.lock().unwrap().len()
        }

        fn contains(&self, namespace: &StorageNamespace, key: &StorageKey) -> bool {
            self.inner
                .lock()
                .unwrap()
                .contains_key(&(namespace.as_str().to_string(), key.as_str().to_string()))
        }

        fn get_raw(&self, namespace: &StorageNamespace, key: &StorageKey) -> Option<StorageValue> {
            self.inner
                .lock()
                .unwrap()
                .get(&(namespace.as_str().to_string(), key.as_str().to_string()))
                .cloned()
        }

        fn mutate(
            &self,
            namespace: &StorageNamespace,
            key: &StorageKey,
            mutate: impl FnOnce(&mut StorageValue),
        ) {
            let mut inner = self.inner.lock().unwrap();
            mutate(
                inner
                    .get_mut(&(namespace.as_str().to_string(), key.as_str().to_string()))
                    .unwrap(),
            );
        }
    }

    impl Storage for JsonMemoryStorage {
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
            let storage_key = (namespace.as_str().to_string(), key.as_str().to_string());
            if inner.contains_key(&storage_key) {
                return Ok(false);
            }
            inner.insert(storage_key, value.clone());
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

    impl Storage for FailingStorage {
        fn put_json(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
            _value: &StorageValue,
        ) -> Result<(), StorageError> {
            *self.calls.lock().unwrap() += 1;
            Err(StorageError::BackendFailed)
        }

        fn put_json_if_absent(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
            _value: &StorageValue,
        ) -> Result<bool, StorageError> {
            Err(StorageError::BackendFailed)
        }

        fn get_json(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
        ) -> Result<Option<StorageValue>, StorageError> {
            Err(StorageError::BackendFailed)
        }

        fn delete(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
        ) -> Result<(), StorageError> {
            Err(StorageError::BackendFailed)
        }

        fn exists(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
        ) -> Result<bool, StorageError> {
            Err(StorageError::BackendFailed)
        }
    }

    #[test]
    fn storage_error_converts_to_audit_error() {
        let store = StorageAuditStore::new(FailingStorage::new(), AuditStoreConfig::default());
        let err = store.record(&event()).unwrap_err();

        assert_eq!(err, AuditError::StoreFailed);
        assert_eq!(err.to_mini_error().to_public().code, STORE_FAILED);
    }
}

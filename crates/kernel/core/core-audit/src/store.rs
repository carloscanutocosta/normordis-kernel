use std::sync::Mutex;

use chrono::{DateTime, Utc};
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

/// Contrato de persistência de auditoria — append-only, cadeia de hashes verificável.
///
/// # Implementações disponíveis
/// - [`StorageAuditStore`] — sobre `support_storage::Storage` genérico; adequado para
///   testes e volumes pequenos. **Não é atómica:** falhas a meio de `record` podem
///   deixar estado parcial. Use `AuditSqliteStore` em produção.
/// - `AuditSqliteStore` (`adapter-audit-sqlite`) — relacional SQLite com `BEGIN IMMEDIATE`,
///   triggers append-only, `details_json` encriptado e verificação incremental eficiente.
pub trait AuditStore: Send + Sync {
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;

    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError>;

    /// Devolve eventos do actor, ordenados por tempo, com paginação.
    fn list_by_actor(
        &self,
        actor_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError>;

    /// Devolve eventos sobre o alvo, ordenados por tempo, com paginação.
    fn list_by_target(
        &self,
        target: &AuditTarget,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError>;

    fn list_all(&self, limit: usize, offset: usize) -> Result<Vec<AuditEvent>, AuditError>;

    /// Devolve eventos cujo `occurred_at_utc` pertence ao intervalo `[from, to[`,
    /// ordenados cronologicamente, com paginação `limit`/`offset`.
    fn list_by_date_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError>;

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError>;

    /// Verifica incrementalmente a partir de `from_sequence`. Reduz o custo de
    /// verificações periódicas para O(novos_eventos). A âncora é lida da própria
    /// base — útil para detectar corrupção acidental. Para provar que o prefixo
    /// não foi adulterado, use [`verify_chain_from_checkpoint`].
    ///
    /// Valores `from_sequence <= 1` equivalem a `verify_chain()`.
    fn verify_chain_since(&self, from_sequence: u64) -> Result<AuditChainReport, AuditError>;

    /// Verificação incremental com checkpoint externo de confiança.
    ///
    /// `checkpoint_sequence` é a última sequência já verificada e considerada segura.
    /// `checkpoint_hash` é o `record_hash` desse evento, guardado externamente
    /// (ficheiro assinado, HSM, etc.) — fora da base auditada.
    ///
    /// O método valida primeiro que o evento na base tem exactamente esse hash;
    /// se não bater, retorna `ChainVerificationFailed` mesmo antes de verificar
    /// o sufixo. Só assim se prova que o prefixo não foi adulterado.
    fn verify_chain_from_checkpoint(
        &self,
        checkpoint_sequence: u64,
        checkpoint_hash: &str,
    ) -> Result<AuditChainReport, AuditError>;

    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError>;
}

/// Implementação sobre `support_storage::Storage` genérico.
///
/// **Adequada para testes e volumes pequenos.** `record()` executa múltiplas
/// operações KV independentes sem transacção — uma falha a meio pode deixar
/// estado parcial. Para produção, use `AuditSqliteStore` (`adapter-audit-sqlite`).
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

    fn list_by_actor(
        &self,
        actor_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        if actor_id.trim().is_empty() || actor_id != actor_id.trim() {
            return Err(AuditError::InvalidActor);
        }
        let namespace = &self.config.namespace;
        let key = audit_actor_index_key(actor_id)?;
        let index = self.read_index(namespace, &key)?;
        let paged = AuditIndex {
            entries: index.entries.into_iter().skip(offset).take(limit).collect(),
        };
        self.read_index_events(namespace, paged)
    }

    fn list_by_target(
        &self,
        target: &AuditTarget,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        target.validate()?;
        let namespace = &self.config.namespace;
        let key = audit_target_index_key(&target.target_type, &target.target_id)?;
        let index = self.read_index(namespace, &key)?;
        let paged = AuditIndex {
            entries: index.entries.into_iter().skip(offset).take(limit).collect(),
        };
        self.read_index_events(namespace, paged)
    }

    fn list_all(&self, limit: usize, offset: usize) -> Result<Vec<AuditEvent>, AuditError> {
        let namespace = &self.config.namespace;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index = self.read_chain_index(namespace, &chain_index_key)?;
        let mut events = Vec::new();
        for entry in chain_index.entries.into_iter().skip(offset).take(limit) {
            let key = support_storage::StorageKey::new(entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            if let Some(value) = self.storage.get_json(namespace, &key)? {
                events.push(read_record_value(value)?);
            }
        }
        Ok(events)
    }

    fn list_by_date_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        let namespace = &self.config.namespace;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index = self.read_chain_index(namespace, &chain_index_key)?;

        let from_ms = from.timestamp_millis();
        let to_ms = to.timestamp_millis();

        // event_key = "{epoch_ms}.{event_id}" — o prefixo codifica occurred_at_utc
        let mut matching: Vec<(i64, AuditChainIndexEntry)> = chain_index
            .entries
            .into_iter()
            .filter_map(|entry| {
                let epoch_ms = entry
                    .event_key
                    .split_once('.')
                    .and_then(|(ms, _)| ms.parse::<i64>().ok())?;
                if epoch_ms >= from_ms && epoch_ms < to_ms {
                    Some((epoch_ms, entry))
                } else {
                    None
                }
            })
            .collect();

        matching.sort_by_key(|(ms, _)| *ms);

        let mut events = Vec::new();
        for (_, entry) in matching.into_iter().skip(offset).take(limit) {
            let key = support_storage::StorageKey::new(entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            if let Some(value) = self.storage.get_json(namespace, &key)? {
                events.push(read_record_value(value)?);
            }
        }
        Ok(events)
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

    fn verify_chain_since(&self, from_sequence: u64) -> Result<AuditChainReport, AuditError> {
        if from_sequence <= 1 {
            return self.verify_chain();
        }
        let namespace = &self.config.namespace;
        let chain_head_key = support_storage::StorageKey::new(AUDIT_CHAIN_HEAD_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_state = self.read_chain_state(namespace, &chain_head_key)?;
        let chain_index = self.read_chain_index(namespace, &chain_index_key)?;

        // A cadeia tem de ter pelo menos from_sequence entradas
        if chain_index.entries.len() < from_sequence as usize {
            return Err(AuditError::ChainVerificationFailed);
        }

        // Âncora: hash do evento na posição from_sequence - 1 (0-indexed: from_sequence - 2)
        let anchor_hash = chain_index
            .entries
            .get((from_sequence - 2) as usize)
            .map(|e| e.record_hash.clone())
            .ok_or(AuditError::ChainVerificationFailed)?;

        let skip = (from_sequence - 1) as usize;
        let mut previous_hash: Option<String> = Some(anchor_hash);

        for (absolute_index, entry) in chain_index.entries.iter().enumerate().skip(skip) {
            let key = support_storage::StorageKey::new(&entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            let Some(value) = self.storage.get_json(namespace, &key)? else {
                return Err(AuditError::ChainVerificationFailed);
            };
            let record = read_record(value)?;
            let expected_sequence = absolute_index as u64 + 1;
            if record.chain_link.sequence != expected_sequence
                || record.chain_link.previous_record_hash != previous_hash
                || record.chain_link.record_hash != entry.record_hash
            {
                return Err(AuditError::ChainVerificationFailed);
            }
            previous_hash = Some(record.chain_link.record_hash);
        }

        let checked = chain_index.entries.len() - skip;

        if chain_state.sequence as usize != chain_index.entries.len()
            || chain_state.head_record_hash != previous_hash
        {
            return Err(AuditError::ChainVerificationFailed);
        }

        Ok(AuditChainReport {
            checked_events: checked,
            head_record_hash: chain_state.head_record_hash,
        })
    }

    fn verify_chain_from_checkpoint(
        &self,
        checkpoint_sequence: u64,
        checkpoint_hash: &str,
    ) -> Result<AuditChainReport, AuditError> {
        if checkpoint_sequence == 0 {
            return Err(AuditError::ChainVerificationFailed);
        }
        let namespace = &self.config.namespace;
        let chain_head_key = support_storage::StorageKey::new(AUDIT_CHAIN_HEAD_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_index_key = support_storage::StorageKey::new(AUDIT_CHAIN_INDEX_KEY)
            .map_err(|_| AuditError::OperationFailed)?;
        let chain_state = self.read_chain_state(namespace, &chain_head_key)?;
        let chain_index = self.read_chain_index(namespace, &chain_index_key)?;

        // Valida o checkpoint contra a BD: se divergir, o prefixo foi adulterado
        let checkpoint_entry = chain_index
            .entries
            .get((checkpoint_sequence - 1) as usize)
            .ok_or(AuditError::ChainVerificationFailed)?;
        if checkpoint_entry.record_hash != checkpoint_hash {
            return Err(AuditError::ChainVerificationFailed);
        }

        // Verifica o sufixo a partir do checkpoint
        let skip = checkpoint_sequence as usize;
        let mut previous_hash: Option<String> = Some(checkpoint_hash.to_string());

        for (absolute_index, entry) in chain_index.entries.iter().enumerate().skip(skip) {
            let key = support_storage::StorageKey::new(&entry.event_key)
                .map_err(|_| AuditError::StoreFailed)?;
            let Some(value) = self.storage.get_json(namespace, &key)? else {
                return Err(AuditError::ChainVerificationFailed);
            };
            let record = read_record(value)?;
            let expected_sequence = absolute_index as u64 + 1;
            if record.chain_link.sequence != expected_sequence
                || record.chain_link.previous_record_hash != previous_hash
                || record.chain_link.record_hash != entry.record_hash
            {
                return Err(AuditError::ChainVerificationFailed);
            }
            previous_hash = Some(record.chain_link.record_hash);
        }

        let checked = chain_index.entries.len() - skip;

        if chain_state.sequence as usize != chain_index.entries.len()
            || chain_state.head_record_hash != previous_hash
        {
            return Err(AuditError::ChainVerificationFailed);
        }

        Ok(AuditChainReport {
            checked_events: checked,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use support_storage::{StorageError, StorageKey, StorageNamespace, StorageValue};

    use super::*;
    use crate::{
        AuditActor, AuditOutcome, AuditTarget, DEFAULT_AUDIT_EVENTS_NAMESPACE, STORE_FAILED,
    };

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
            AuditOutcome::NotApplicable,
            None,
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
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();

        store.record(&second).unwrap();
        store.record(&first).unwrap();

        let events = store.list_by_actor("user-1", 10, 0).unwrap();
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
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();

        store.record(&first).unwrap();
        store.record(&other).unwrap();

        let events = store.list_by_target(&target, 10, 0).unwrap();
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
            AuditOutcome::NotApplicable,
            None,
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
                    AuditOutcome::NotApplicable,
                    None,
                    Some(json!({"index": index})),
                )
                .unwrap();
                store.record(&event).unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            store.list_by_actor("user-concurrent", 64, 0).unwrap().len(),
            32
        );
    }

    #[derive(Debug)]
    struct FailingStorage;

    impl FailingStorage {
        fn new() -> Self {
            Self
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

    #[test]
    fn list_all_returns_events_in_sequence_order() {
        let store = storage_store();
        let first = event();
        let second = AuditEvent::with_id_and_time(
            "event-2",
            "document.updated",
            AuditActor::new("user-2").unwrap(),
            AuditTarget::new("document", "doc-2").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 2, 0).unwrap(),
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();
        let third = AuditEvent::with_id_and_time(
            "event-3",
            "document.deleted",
            AuditActor::new("user-3").unwrap(),
            AuditTarget::new("document", "doc-3").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 3, 0).unwrap(),
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();

        store.record(&third).unwrap();
        store.record(&first).unwrap();
        store.record(&second).unwrap();

        let all = store.list_all(10, 0).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].event_id, "event-3");
        assert_eq!(all[1].event_id, "event-1");
        assert_eq!(all[2].event_id, "event-2");
    }

    #[test]
    fn list_all_supports_pagination() {
        let store = storage_store();
        for i in 1..=5u32 {
            let ev = AuditEvent::with_id_and_time(
                format!("event-{i}"),
                "document.created",
                AuditActor::new(format!("user-{i}")).unwrap(),
                AuditTarget::new("document", format!("doc-{i}")).unwrap(),
                Utc.with_ymd_and_hms(2026, 5, 11, 10, i, 0).unwrap(),
                AuditOutcome::NotApplicable,
                None,
                None,
            )
            .unwrap();
            store.record(&ev).unwrap();
        }

        let page1 = store.list_all(2, 0).unwrap();
        let page2 = store.list_all(2, 2).unwrap();
        let page3 = store.list_all(2, 4).unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
        let all_ids: Vec<_> = [page1, page2, page3]
            .concat()
            .into_iter()
            .map(|e| e.event_id)
            .collect();
        assert_eq!(all_ids.len(), 5);
        let unique: std::collections::HashSet<_> = all_ids.iter().collect();
        assert_eq!(unique.len(), 5);
    }

    #[test]
    fn list_all_empty_store_returns_empty() {
        let store = storage_store();
        assert_eq!(store.list_all(10, 0).unwrap(), vec![]);
    }

    // ── list_by_date_range ────────────────────────────────────────────────────

    fn events_at_hours(count: u32) -> Vec<AuditEvent> {
        (1..=count)
            .map(|h| {
                AuditEvent::with_id_and_time(
                    format!("event-{h}"),
                    "document.created",
                    AuditActor::new(format!("user-{h}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{h}")).unwrap(),
                    Utc.with_ymd_and_hms(2026, 5, 11, h, 0, 0).unwrap(),
                    AuditOutcome::NotApplicable,
                    None,
                    None,
                )
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn list_by_date_range_returns_events_in_window() {
        let store = storage_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }

        let from = Utc.with_ymd_and_hms(2026, 5, 11, 2, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 11, 4, 0, 0).unwrap();
        let events = store.list_by_date_range(from, to, 10, 0).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "event-2");
        assert_eq!(events[1].event_id, "event-3");
    }

    #[test]
    fn list_by_date_range_respects_pagination() {
        let store = storage_store();
        for ev in events_at_hours(6) {
            store.record(&ev).unwrap();
        }

        let from = Utc.with_ymd_and_hms(2026, 5, 11, 1, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 11, 7, 0, 0).unwrap();

        let page1 = store.list_by_date_range(from, to, 2, 0).unwrap();
        let page2 = store.list_by_date_range(from, to, 2, 2).unwrap();
        let page3 = store.list_by_date_range(from, to, 2, 4).unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 2);
        let ids: Vec<_> = [page1, page2, page3]
            .concat()
            .into_iter()
            .map(|e| e.event_id)
            .collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn list_by_date_range_empty_window_returns_empty() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        let from = Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 13, 0, 0, 0).unwrap();
        assert_eq!(store.list_by_date_range(from, to, 10, 0).unwrap(), vec![]);
    }

    #[test]
    fn list_by_date_range_to_is_exclusive() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        // to = exactly event-2's timestamp → deve excluir event-2
        let from = Utc.with_ymd_and_hms(2026, 5, 11, 1, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 11, 2, 0, 0).unwrap();
        let events = store.list_by_date_range(from, to, 10, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "event-1");
    }

    // ── verify_chain_since ────────────────────────────────────────────────────

    #[test]
    fn verify_chain_since_verifies_suffix_of_chain() {
        let store = storage_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }

        let report = store.verify_chain_since(3).unwrap();
        assert_eq!(report.checked_events, 3); // eventos 3, 4, 5
        assert!(report.head_record_hash.is_some());
        // hash igual ao da verificação completa
        assert_eq!(
            report.head_record_hash,
            store.verify_chain().unwrap().head_record_hash
        );
    }

    #[test]
    fn verify_chain_since_1_equals_verify_chain() {
        let store = storage_store();
        for ev in events_at_hours(4) {
            store.record(&ev).unwrap();
        }
        let full = store.verify_chain().unwrap();
        let since1 = store.verify_chain_since(1).unwrap();
        assert_eq!(full.checked_events, since1.checked_events);
        assert_eq!(full.head_record_hash, since1.head_record_hash);
    }

    #[test]
    fn verify_chain_since_beyond_chain_length_fails() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store.verify_chain_since(10).unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    // ── list_by_actor / list_by_target — paginação ────────────────────────────

    #[test]
    fn list_by_actor_supports_pagination() {
        let store = storage_store();
        for ev in events_at_hours(4) {
            store.record(&ev).unwrap();
        }
        let page1 = store.list_by_actor("user-1", 2, 0).unwrap();
        let page2 = store.list_by_actor("user-1", 2, 2).unwrap();
        assert_eq!(page1.len(), 1); // apenas event-1 é de user-1
        assert_eq!(page2.len(), 0);
    }

    #[test]
    fn list_by_target_supports_pagination() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        let t1 = AuditTarget::new("document", "doc-1").unwrap();
        let all = store.list_by_target(&t1, 10, 0).unwrap();
        assert_eq!(all.len(), 1);
        let none = store.list_by_target(&t1, 10, 1).unwrap();
        assert_eq!(none.len(), 0);
    }

    // ── verify_chain_from_checkpoint ─────────────────────────────────────────

    #[test]
    fn verify_chain_from_checkpoint_verifies_suffix_with_external_anchor() {
        let store = storage_store();
        for ev in events_at_hours(5) {
            store.record(&ev).unwrap();
        }
        // Obtém o hash do evento 2 como checkpoint externo
        let full = store.verify_chain().unwrap();
        let checkpoint_2 = {
            let ns = &store.config().namespace;
            let ev2 = AuditEvent::with_id_and_time(
                "event-2",
                "document.created",
                AuditActor::new("user-2").unwrap(),
                AuditTarget::new("document", "doc-2").unwrap(),
                Utc.with_ymd_and_hms(2026, 5, 11, 2, 0, 0).unwrap(),
                AuditOutcome::NotApplicable,
                None,
                None,
            )
            .unwrap();
            let key = audit_event_key(&ev2).unwrap();
            let value = store.storage().get_raw(ns, &key).unwrap();
            value["chain_link"]["record_hash"]
                .as_str()
                .unwrap()
                .to_string()
        };

        let report = store
            .verify_chain_from_checkpoint(2, &checkpoint_2)
            .unwrap();
        assert_eq!(report.checked_events, 3); // eventos 3, 4, 5
        assert_eq!(report.head_record_hash, full.head_record_hash);
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_wrong_external_hash() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store
                .verify_chain_from_checkpoint(2, "hash-errado-que-nao-existe")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_out_of_range_sequence() {
        let store = storage_store();
        for ev in events_at_hours(3) {
            store.record(&ev).unwrap();
        }
        assert_eq!(
            store
                .verify_chain_from_checkpoint(10, "qualquer-hash")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }

    #[test]
    fn verify_chain_from_checkpoint_rejects_zero_sequence() {
        let store = storage_store();
        store.record(&event()).unwrap();
        assert_eq!(
            store
                .verify_chain_from_checkpoint(0, "qualquer-hash")
                .unwrap_err(),
            AuditError::ChainVerificationFailed
        );
    }
}

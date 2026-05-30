use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::actor::AuditActor;
use crate::error::AuditError;
use crate::event::AuditEvent;
use crate::signature::{sign_manifest, AuditSigningKey, SignedAuditExportManifest};
use crate::store::AuditStore;
use crate::target::AuditTarget;

#[derive(Debug)]
pub struct AuditService<S>
where
    S: AuditStore,
{
    store: S,
}

impl<S> AuditService<S>
where
    S: AuditStore,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn record_event(
        &self,
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        details_json: Option<Value>,
    ) -> Result<AuditEvent, AuditError> {
        let event = AuditEvent::new(event_type, actor, target, details_json)?;
        self.store.record(&event)?;
        Ok(event)
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError> {
        self.store.get(event_id)
    }

    pub fn list_by_actor(
        &self,
        actor_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_by_actor(actor_id, limit, offset)
    }

    pub fn list_by_target(
        &self,
        target: &AuditTarget,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_by_target(target, limit, offset)
    }

    pub fn list_all(&self, limit: usize, offset: usize) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_all(limit, offset)
    }

    pub fn list_by_date_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_by_date_range(from, to, limit, offset)
    }

    pub fn verify_chain(&self) -> Result<crate::AuditChainReport, AuditError> {
        self.store.verify_chain()
    }

    pub fn export_manifest(&self) -> Result<crate::AuditExportManifest, AuditError> {
        self.store.export_manifest()
    }

    pub fn verify_chain_since(&self, from_sequence: u64) -> Result<crate::AuditChainReport, AuditError> {
        self.store.verify_chain_since(from_sequence)
    }

    pub fn verify_chain_from_checkpoint(
        &self,
        checkpoint_sequence: u64,
        checkpoint_hash: &str,
    ) -> Result<crate::AuditChainReport, AuditError> {
        self.store.verify_chain_from_checkpoint(checkpoint_sequence, checkpoint_hash)
    }

    pub fn sign_and_export(
        &self,
        signing_key: &AuditSigningKey,
        key_id: Option<String>,
    ) -> Result<SignedAuditExportManifest, AuditError> {
        let manifest = self.export_manifest()?;
        sign_manifest(manifest, signing_key, key_id)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::Utc;

    use super::*;
    use crate::{AuditStore, AuditTarget};

    #[derive(Debug, Default)]
    struct RecordingStore {
        events: Mutex<Vec<AuditEvent>>,
    }

    impl AuditStore for RecordingStore {
        fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }

        fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .find(|event| event.event_id == event_id)
                .cloned())
        }

        fn list_by_actor(
            &self,
            actor_id: &str,
            limit: usize,
            offset: usize,
        ) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.actor.actor_id == actor_id)
                .skip(offset)
                .take(limit)
                .cloned()
                .collect())
        }

        fn list_by_target(
            &self,
            target: &AuditTarget,
            limit: usize,
            offset: usize,
        ) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.target == *target)
                .skip(offset)
                .take(limit)
                .cloned()
                .collect())
        }

        fn list_all(&self, limit: usize, offset: usize) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect())
        }

        fn list_by_date_range(
            &self,
            from: DateTime<Utc>,
            to: DateTime<Utc>,
            limit: usize,
            offset: usize,
        ) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.occurred_at_utc >= from && e.occurred_at_utc < to)
                .skip(offset)
                .take(limit)
                .cloned()
                .collect())
        }

        fn verify_chain(&self) -> Result<crate::AuditChainReport, AuditError> {
            Ok(crate::AuditChainReport {
                checked_events: self.events.lock().unwrap().len(),
                head_record_hash: None,
            })
        }

        fn verify_chain_since(&self, from_sequence: u64) -> Result<crate::AuditChainReport, AuditError> {
            let total = self.events.lock().unwrap().len();
            let start = (from_sequence as usize).saturating_sub(1).min(total);
            Ok(crate::AuditChainReport {
                checked_events: total - start,
                head_record_hash: None,
            })
        }

        fn verify_chain_from_checkpoint(
            &self,
            checkpoint_sequence: u64,
            _checkpoint_hash: &str,
        ) -> Result<crate::AuditChainReport, AuditError> {
            let total = self.events.lock().unwrap().len();
            let start = (checkpoint_sequence as usize).min(total);
            Ok(crate::AuditChainReport {
                checked_events: total - start,
                head_record_hash: None,
            })
        }

        fn export_manifest(&self) -> Result<crate::AuditExportManifest, AuditError> {
            Ok(crate::AuditExportManifest {
                schema_version: 1,
                generated_at_utc: chrono::Utc::now(),
                events_count: self.events.lock().unwrap().len(),
                head_record_hash: None,
                manifest_hash: "test".to_string(),
            })
        }
    }

    #[test]
    fn record_event_generates_uuid() {
        let service = AuditService::new(RecordingStore::default());
        let event = service
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                None,
            )
            .unwrap();

        uuid::Uuid::parse_str(&event.event_id).unwrap();
    }

    #[test]
    fn record_event_writes_to_store() {
        let service = AuditService::new(RecordingStore::default());
        let event = service
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                None,
            )
            .unwrap();

        assert_eq!(service.store().get(&event.event_id).unwrap(), Some(event));
    }

    #[test]
    fn list_all_delegates_to_store() {
        let service = AuditService::new(RecordingStore::default());
        service
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                None,
            )
            .unwrap();
        service
            .record_event(
                "document.updated",
                AuditActor::new("user-2").unwrap(),
                AuditTarget::new("document", "doc-2").unwrap(),
                None,
            )
            .unwrap();

        assert_eq!(service.list_all(10, 0).unwrap().len(), 2);
        assert_eq!(service.list_all(1, 0).unwrap().len(), 1);
        assert_eq!(service.list_all(1, 1).unwrap().len(), 1);
        assert_eq!(service.list_all(10, 2).unwrap().len(), 0);
    }

    #[test]
    fn sign_and_export_produces_verifiable_manifest() {
        use crate::{verify_signed_manifest, AuditSigningKey};

        let service = AuditService::new(RecordingStore::default());
        service
            .record_event(
                "document.created",
                AuditActor::new("user-1").unwrap(),
                AuditTarget::new("document", "doc-1").unwrap(),
                None,
            )
            .unwrap();

        let key = AuditSigningKey::from_bytes([42; 32]);
        let signed = service.sign_and_export(&key, Some("key-1".to_string())).unwrap();

        assert_eq!(signed.manifest.events_count, 1);
        verify_signed_manifest(&signed).unwrap();
    }

    #[test]
    fn list_by_date_range_delegates_with_filter() {
        let service = AuditService::new(RecordingStore::default());
        for h in 1u32..=4 {
            service
                .record_event(
                    "document.created",
                    AuditActor::new(format!("user-{h}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{h}")).unwrap(),
                    None,
                )
                .unwrap();
        }
        // RecordingStore usa Utc::now() para occurred_at — verificamos apenas que
        // o método delega sem pânico e devolve um Vec.
        let from = Utc::now() - chrono::Duration::hours(1);
        let to = Utc::now() + chrono::Duration::hours(1);
        let events = service.list_by_date_range(from, to, 10, 0).unwrap();
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn verify_chain_since_delegates_to_store() {
        let service = AuditService::new(RecordingStore::default());
        for i in 0..5 {
            service
                .record_event(
                    "document.created",
                    AuditActor::new(format!("user-{i}")).unwrap(),
                    AuditTarget::new("document", format!("doc-{i}")).unwrap(),
                    None,
                )
                .unwrap();
        }
        let report = service.verify_chain_since(3).unwrap();
        assert_eq!(report.checked_events, 3); // eventos 3, 4, 5
    }
}

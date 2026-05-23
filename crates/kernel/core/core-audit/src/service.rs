use serde_json::Value;

use crate::actor::AuditActor;
use crate::error::AuditError;
use crate::event::AuditEvent;
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

    pub fn list_by_actor(&self, actor_id: &str) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_by_actor(actor_id)
    }

    pub fn list_by_target(&self, target: &AuditTarget) -> Result<Vec<AuditEvent>, AuditError> {
        self.store.list_by_target(target)
    }

    pub fn verify_chain(&self) -> Result<crate::AuditChainReport, AuditError> {
        self.store.verify_chain()
    }

    pub fn export_manifest(&self) -> Result<crate::AuditExportManifest, AuditError> {
        self.store.export_manifest()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

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

        fn list_by_actor(&self, actor_id: &str) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.actor.actor_id == actor_id)
                .cloned()
                .collect())
        }

        fn list_by_target(&self, target: &AuditTarget) -> Result<Vec<AuditEvent>, AuditError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.target == *target)
                .cloned()
                .collect())
        }

        fn verify_chain(&self) -> Result<crate::AuditChainReport, AuditError> {
            Ok(crate::AuditChainReport {
                checked_events: self.events.lock().unwrap().len(),
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
}

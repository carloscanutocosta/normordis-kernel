use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::actor::AuditActor;
use crate::error::AuditError;
use crate::event::AuditEvent;
use crate::outcome::AuditOutcome;
use crate::signature::{sign_manifest, AuditSigningKey, SignedAuditExportManifest};
use crate::store::AuditStore;
use crate::target::AuditTarget;

/// Pedido de gravação de um evento de auditoria.
///
/// # Enquadramento COSO
///
/// O `RecordAuditEventRequest` formaliza todos os elementos necessários para que
/// um evento constitua evidência COSO válida:
///
/// - **Quem actuou** → `actor`
/// - **Sobre quê** → `target`
/// - **Que acção** → `event_type`
/// - **Com que resultado** → `outcome`
/// - **Que controlo exerceu** → `control_id` (opcional)
/// - **Com que contexto** → `details_json` (opcional)
///
/// O timestamp (`occurred_at_utc`) é gerado automaticamente pelo serviço no
/// momento da gravação.
///
/// # Exemplo
///
/// ```rust
/// use core_audit::{AuditActor, AuditOutcome, AuditTarget, RecordAuditEventRequest};
///
/// // Evento simples sem controlo associado
/// let req = RecordAuditEventRequest::new(
///     "user.session.started",
///     AuditActor::new("user-42").unwrap(),
///     AuditTarget::new("session", "sess-001").unwrap(),
///     AuditOutcome::Success,
/// );
///
/// // Evento ligado a um controlo COSO e com contexto adicional
/// let req = RecordAuditEventRequest::new(
///     "document.classification.changed",
///     AuditActor::new("inspector-7").unwrap(),
///     AuditTarget::new("document", "doc-123").unwrap(),
///     AuditOutcome::Success,
/// )
/// .with_control_id("CTRL-014")
/// .with_details(serde_json::json!({
///     "from": "restricted",
///     "to": "confidential"
/// }));
/// ```
#[derive(Debug)]
pub struct RecordAuditEventRequest {
    /// Classificação semântica do acontecimento (ex.: `document.classification.changed`).
    pub event_type: String,
    /// Actor que realizou a acção.
    pub actor: AuditActor,
    /// Entidade sobre a qual a acção incidiu.
    pub target: AuditTarget,
    /// Resultado observável da operação.
    pub outcome: AuditOutcome,
    /// Referência ao controlo COSO exercido, se aplicável.
    pub control_id: Option<String>,
    /// Contexto adicional em JSON livre, sem dados sensíveis.
    pub details_json: Option<Value>,
}

impl RecordAuditEventRequest {
    /// Constrói um pedido com os campos obrigatórios.
    ///
    /// `control_id` e `details_json` são `None` por omissão; use os métodos
    /// de builder [`with_control_id`] e [`with_details`] para os definir.
    ///
    /// [`with_control_id`]: RecordAuditEventRequest::with_control_id
    /// [`with_details`]: RecordAuditEventRequest::with_details
    pub fn new(
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            actor,
            target,
            outcome,
            control_id: None,
            details_json: None,
        }
    }

    /// Associa este evento a um controlo do Registo de Controlos.
    ///
    /// O `control_id` é uma referência externa; o `core-audit` não valida a
    /// existência do controlo no registo, apenas o formato da referência.
    pub fn with_control_id(mut self, control_id: impl Into<String>) -> Self {
        self.control_id = Some(control_id.into());
        self
    }

    /// Adiciona contexto adicional ao evento em formato JSON livre.
    ///
    /// O valor não deve conter chaves sensíveis (passwords, tokens, etc.).
    /// A política de validação rejeita o evento se as encontrar.
    pub fn with_details(mut self, details: Value) -> Self {
        self.details_json = Some(details);
        self
    }
}

/// Serviço de auditoria — ponto de entrada para gravação e consulta de eventos.
///
/// `AuditService` é uma fachada sobre [`AuditStore`] que centraliza a criação
/// de eventos (geração de UUID e timestamp) e delega toda a persistência ao
/// store configurado.
///
/// # Uso típico
///
/// ```rust,ignore
/// use core_audit::{AuditService, AuditOutcome, RecordAuditEventRequest};
///
/// let service = AuditService::new(store);
///
/// service.record_event(
///     RecordAuditEventRequest::new(
///         "document.created",
///         actor,
///         target,
///         AuditOutcome::Success,
///     )
///     .with_control_id("CTRL-001"),
/// )?;
/// ```
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

    /// Cria e grava um evento de auditoria a partir de um [`RecordAuditEventRequest`].
    ///
    /// O `event_id` e o `occurred_at_utc` são gerados automaticamente.
    ///
    /// # Erros
    ///
    /// Devolve [`AuditError`] se a validação do evento falhar ou se a gravação
    /// no store falhar.
    pub fn record_event(
        &self,
        request: RecordAuditEventRequest,
    ) -> Result<AuditEvent, AuditError> {
        let event = AuditEvent::new(
            request.event_type,
            request.actor,
            request.target,
            request.outcome,
            request.control_id,
            request.details_json,
        )?;
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

    pub fn verify_chain_since(
        &self,
        from_sequence: u64,
    ) -> Result<crate::AuditChainReport, AuditError> {
        self.store.verify_chain_since(from_sequence)
    }

    pub fn verify_chain_from_checkpoint(
        &self,
        checkpoint_sequence: u64,
        checkpoint_hash: &str,
    ) -> Result<crate::AuditChainReport, AuditError> {
        self.store
            .verify_chain_from_checkpoint(checkpoint_sequence, checkpoint_hash)
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

        fn verify_chain_since(
            &self,
            from_sequence: u64,
        ) -> Result<crate::AuditChainReport, AuditError> {
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

    fn simple_request(event_type: &str, actor_id: &str, target_id: &str) -> RecordAuditEventRequest {
        RecordAuditEventRequest::new(
            event_type,
            AuditActor::new(actor_id).unwrap(),
            AuditTarget::new("document", target_id).unwrap(),
            AuditOutcome::Success,
        )
    }

    #[test]
    fn record_event_generates_uuid() {
        let service = AuditService::new(RecordingStore::default());
        let event = service
            .record_event(simple_request("document.created", "user-1", "doc-1"))
            .unwrap();

        uuid::Uuid::parse_str(&event.event_id).unwrap();
    }

    #[test]
    fn record_event_writes_to_store() {
        let service = AuditService::new(RecordingStore::default());
        let event = service
            .record_event(simple_request("document.created", "user-1", "doc-1"))
            .unwrap();

        assert_eq!(service.store().get(&event.event_id).unwrap(), Some(event));
    }

    #[test]
    fn record_event_preserves_outcome() {
        let service = AuditService::new(RecordingStore::default());
        let req = RecordAuditEventRequest::new(
            "document.deleted",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::Failure,
        );
        let event = service.record_event(req).unwrap();

        assert_eq!(event.outcome, AuditOutcome::Failure);
    }

    #[test]
    fn record_event_preserves_control_id() {
        let service = AuditService::new(RecordingStore::default());
        let req = simple_request("document.approved", "user-1", "doc-1")
            .with_control_id("CTRL-014");
        let event = service.record_event(req).unwrap();

        assert_eq!(event.control_id, Some("CTRL-014".to_string()));
    }

    #[test]
    fn record_event_preserves_details() {
        let service = AuditService::new(RecordingStore::default());
        let req = simple_request("document.created", "user-1", "doc-1")
            .with_details(serde_json::json!({"ip": "127.0.0.1"}));
        let event = service.record_event(req).unwrap();

        assert_eq!(
            event.details_json,
            Some(serde_json::json!({"ip": "127.0.0.1"}))
        );
    }

    #[test]
    fn list_all_delegates_to_store() {
        let service = AuditService::new(RecordingStore::default());
        service
            .record_event(simple_request("document.created", "user-1", "doc-1"))
            .unwrap();
        service
            .record_event(simple_request("document.updated", "user-2", "doc-2"))
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
            .record_event(simple_request("document.created", "user-1", "doc-1"))
            .unwrap();

        let key = AuditSigningKey::from_bytes([42; 32]);
        let signed = service
            .sign_and_export(&key, Some("key-1".to_string()))
            .unwrap();

        assert_eq!(signed.manifest.events_count, 1);
        verify_signed_manifest(&signed).unwrap();
    }

    #[test]
    fn list_by_date_range_delegates_with_filter() {
        let service = AuditService::new(RecordingStore::default());
        for h in 1u32..=4 {
            service
                .record_event(simple_request(
                    "document.created",
                    &format!("user-{h}"),
                    &format!("doc-{h}"),
                ))
                .unwrap();
        }
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
                .record_event(simple_request(
                    "document.created",
                    &format!("user-{i}"),
                    &format!("doc-{i}"),
                ))
                .unwrap();
        }
        let report = service.verify_chain_since(3).unwrap();
        assert_eq!(report.checked_events, 3);
    }
}

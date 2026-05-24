use chrono::{NaiveDate, Utc};
use core_audit::{AuditChainReport, AuditError, AuditEvent, AuditExportManifest, AuditStore};
use core_org::{OrgAddress, OrgContacts, OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus};
use core_rh::{resolve_current_user, OrgPositionRef, UserIdentity, UserRole};
use miniapp_runtime::{
    create_document_created_event, create_document_instance, record_document_created,
    CreateDocumentRequest, MiniAppContext,
};
use serde_json::json;

// ── Fake AuditStore ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct RecordingAuditStore {
    events: std::sync::Mutex<Vec<AuditEvent>>,
}

impl RecordingAuditStore {
    fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

impl AuditStore for RecordingAuditStore {
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
            .find(|e| e.event_id == event_id)
            .cloned())
    }

    fn list_by_actor(&self, actor_id: &str) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.actor.actor_id == actor_id)
            .cloned()
            .collect())
    }

    fn list_by_target(
        &self,
        target: &core_audit::AuditTarget,
    ) -> Result<Vec<AuditEvent>, AuditError> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.target == *target)
            .cloned()
            .collect())
    }

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError> {
        Ok(AuditChainReport {
            checked_events: self.len(),
            head_record_hash: None,
        })
    }

    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError> {
        Ok(AuditExportManifest {
            schema_version: 1,
            generated_at_utc: Utc::now(),
            events_count: self.len(),
            head_record_hash: None,
            manifest_hash: "test".to_string(),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sample_org() -> OrgUnit {
    OrgUnit {
        id: OrgUnitId("org1".into()),
        short_name: "SF VFX".into(),
        full_name: "Serviço de Finanças VFX".into(),
        service_code: Some("SFVFX".into()),
        level: OrgLevel::new(3).unwrap(),
        parent_id: Some(OrgUnitId("df-vfx".into())),
        contacts: OrgContacts {
            email: Some("sf@example.test".into()),
            phone: None,
            fax: None,
            address: OrgAddress::default(),
        },
        created_by: None,
        legal_reference: None,
        valid_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        valid_until: None,
        status: OrgUnitStatus::Active,
    }
}

fn context_sem_posicao() -> MiniAppContext {
    let user = resolve_current_user(UserIdentity {
        user_id: "u1".into(),
        username: "ccosta".into(),
        display_name: "Carlos Costa".into(),
        email: Some("carlos@example.test".into()),
        role: UserRole::Utilizador,
    })
    .unwrap();
    MiniAppContext {
        app_name: "requerimentos".into(),
        user_context: user,
        org_config: sample_org(),
    }
}

fn context_com_posicao() -> MiniAppContext {
    let pos = OrgPositionRef::new("pos-001", "org1", "comp-001", None).unwrap();
    let user = resolve_current_user(UserIdentity {
        user_id: "u1".into(),
        username: "ccosta".into(),
        display_name: "Carlos Costa".into(),
        email: Some("carlos@example.test".into()),
        role: UserRole::Utilizador,
    })
    .unwrap()
    .with_position(pos);
    MiniAppContext {
        app_name: "requerimentos".into(),
        user_context: user,
        org_config: sample_org(),
    }
}

fn sample_request() -> CreateDocumentRequest {
    CreateDocumentRequest {
        document_id: "doc-001".into(),
        document_type: "requerimento".into(),
        template_id: "tpl-req-v1".into(),
        template_version: "v1".into(),
        payload_json: json!({ "assunto": "certidão" }),
    }
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[test]
fn create_document_instance_starts_in_draft() {
    let ctx = context_sem_posicao();
    let doc = create_document_instance(&ctx, sample_request(), Utc::now()).unwrap();
    use core_documental::DocumentStatus;
    assert_eq!(doc.status, DocumentStatus::Draft);
    assert_eq!(doc.template_id.as_str(), "tpl-req-v1");
    assert_eq!(doc.document_type, "requerimento");
    assert!(doc.authority_context.is_none());
    assert!(doc.document_number.is_none());
}

#[test]
fn create_document_created_event_ok() {
    let ctx = context_com_posicao();
    let doc = create_document_instance(&ctx, sample_request(), Utc::now()).unwrap();
    let evt = create_document_created_event(&doc, &ctx, Utc::now()).unwrap();
    use core_documental::DocumentEventType;
    assert_eq!(evt.event_type, DocumentEventType::Created);
    assert!(evt.previous_hash.is_none());
    assert_eq!(evt.document_id.as_str(), "doc-001");
}

#[test]
fn create_document_created_event_sem_posicao_rejeita() {
    let ctx = context_sem_posicao();
    let doc = create_document_instance(&ctx, sample_request(), Utc::now()).unwrap();
    use miniapp_runtime::RuntimeError;
    assert!(matches!(
        create_document_created_event(&doc, &ctx, Utc::now()),
        Err(RuntimeError::MissingPosition)
    ));
}

#[test]
fn records_audit_event() {
    let ctx = context_sem_posicao();
    let doc = create_document_instance(&ctx, sample_request(), Utc::now()).unwrap();
    let recorder = RecordingAuditStore::default();
    record_document_created(&recorder, &ctx, &doc, Utc::now()).unwrap();
    assert_eq!(recorder.len(), 1);
}

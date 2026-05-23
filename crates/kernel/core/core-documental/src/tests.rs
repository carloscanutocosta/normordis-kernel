//! Testes unitários de `core-documental`.
//!
//! Cobrem invariantes de domínio: máquina de estados, finalização, numeração,
//! cadeia de eventos, validação de pacote e templates.

use chrono::Utc;
use serde_json::json;

use crate::{
    archive::{NdfRecord, NdfRecordId},
    attachment::{AttachmentId, AttachmentKind, DocumentAttachment},
    authority::AuthorityContext,
    custody::{DocumentCustody, DocumentId, DocumentStatus},
    error::DocumentalError,
    events::{DocumentEvent, DocumentEventId, DocumentEventType, EventActor},
    package::{
        validate_document_package, Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef,
    },
    template::{DocumentTemplate, TemplateId, TemplateStatus},
};

use core_org::{CompetencyId, OrgPositionId, OrgUnitId};
use core_rh::{
    identity::{UserContext, UserIdentity},
    org::OrgPositionRef,
    UserId, UserRole,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sample_authority() -> AuthorityContext {
    AuthorityContext {
        user_id: UserId::new("user-1").unwrap(),
        position_id: OrgPositionId::new("pos-1").unwrap(),
        unit_id: OrgUnitId::new("unit-1").unwrap(),
        competency_id: CompetencyId::new("comp-1").unwrap(),
        delegation_id: None,
        captured_at: Utc::now(),
    }
}

fn sample_custody() -> DocumentCustody {
    DocumentCustody {
        id: DocumentId::new("doc-001").unwrap(),
        document_type: "oficio".into(),
        template_id: TemplateId::new("tpl-001").unwrap(),
        template_version: "v1".into(),
        status: DocumentStatus::Draft,
        payload_json: json!({ "assunto": "Teste" }),
        authority_context: None,
        document_number: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn sample_package() -> DocumentPackage {
    let ts = Utc::now();
    DocumentPackage {
        document_id: "doc:oficio:001:1.0.0".into(),
        created_at: ts,
        template: TemplateRef {
            template_id: "tpl-oficio".into(),
            template_version: "v1".into(),
            valid_at: None,
        },
        engine: EngineRef {
            engine_id: "engine-typst".into(),
            engine_version: "v1".into(),
        },
        artefacts: vec![Artefact {
            kind: "pdf".into(),
            artefact_ref: "doc:oficio:001:1.0.0:pdf".into(),
            hash_result: HashResult {
                algorithm: "SHA-256".into(),
                hash: "sha256:abc123".into(),
                timestamp: ts,
                input_kind: None,
                input_ref: None,
                meta: None,
            },
            mime: Some("application/pdf".into()),
            size_bytes: Some(1024),
        }],
        subject: None,
        meta: None,
    }
}

// ── DocumentId ────────────────────────────────────────────────────────────────

#[test]
fn document_id_aceita_chave_segura() {
    let id = DocumentId::new("doc-oficio_001.2026").unwrap();
    assert_eq!(id.as_str(), "doc-oficio_001.2026");
}

#[test]
fn document_id_rejeita_traversal_e_separadores_de_caminho() {
    for unsafe_id in [
        "../etc/passwd",
        "..\\..\\config.json",
        "docs/001",
        "docs\\001",
        ".",
        "..",
        "doc..001",
    ] {
        assert!(matches!(
            DocumentId::new(unsafe_id),
            Err(DocumentalError::InvalidIdentifier { .. })
        ));
    }
}

#[test]
fn document_id_rejeita_caracteres_fora_da_chave_canonica() {
    for unsafe_id in ["C:temp", "doc 001", "doc#001", "doc/001"] {
        assert!(matches!(
            DocumentId::new(unsafe_id),
            Err(DocumentalError::InvalidIdentifier { .. })
        ));
    }
}

#[test]
fn custody_validate_rejeita_document_id_construido_diretamente() {
    let mut doc = sample_custody();
    doc.id = DocumentId("../escape".into());
    assert!(matches!(
        doc.validate(),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
}

#[test]
fn document_id_deserialize_rejeita_valor_inseguro() {
    let result = serde_json::from_str::<DocumentId>(r#""..\config.json""#);
    assert!(result.is_err());
}

// ── DocumentStatus ────────────────────────────────────────────────────────────

#[test]
fn transicoes_validas_de_draft() {
    assert!(DocumentStatus::Draft.can_transition_to(&DocumentStatus::PendingApproval));
    assert!(DocumentStatus::Draft.can_transition_to(&DocumentStatus::Archived));
    assert!(!DocumentStatus::Draft.can_transition_to(&DocumentStatus::Finalized));
    assert!(!DocumentStatus::Draft.can_transition_to(&DocumentStatus::Annulled));
}

#[test]
fn transicoes_validas_de_pending_approval() {
    assert!(DocumentStatus::PendingApproval.can_transition_to(&DocumentStatus::Approved));
    assert!(DocumentStatus::PendingApproval.can_transition_to(&DocumentStatus::Draft));
    assert!(!DocumentStatus::PendingApproval.can_transition_to(&DocumentStatus::Finalized));
}

#[test]
fn estados_terminais_nao_transitam() {
    assert!(!DocumentStatus::Archived.can_transition_to(&DocumentStatus::Draft));
    assert!(!DocumentStatus::Annulled.can_transition_to(&DocumentStatus::Draft));
    assert!(!DocumentStatus::Archived.can_transition_to(&DocumentStatus::Annulled));
}

#[test]
fn from_str_round_trip() {
    for s in [
        "draft",
        "pending_approval",
        "approved",
        "finalized",
        "archived",
        "annulled",
    ] {
        let status = DocumentStatus::from_str(s).unwrap();
        assert_eq!(status.as_str(), s);
    }
}

#[test]
fn from_str_desconhecido_devolve_none() {
    assert!(DocumentStatus::from_str("invalid").is_none());
}

#[test]
fn try_from_str_desconhecido_devolve_err() {
    let result = DocumentStatus::try_from("nao_existe");
    assert!(result.is_err());
}

// ── DocumentCustody ───────────────────────────────────────────────────────────

#[test]
fn assign_number_uma_vez() {
    let doc = sample_custody();
    assert!(doc.assign_number("2026/001").is_ok());
}

#[test]
fn assign_number_vazio_rejeita() {
    let doc = sample_custody();
    assert!(matches!(
        doc.assign_number("   "),
        Err(DocumentalError::EmptyDocumentNumber)
    ));
}

#[test]
fn assign_number_duas_vezes_rejeita() {
    let mut doc = sample_custody();
    doc.document_number = Some("2026/001".into());
    assert!(matches!(
        doc.assign_number("2026/002"),
        Err(DocumentalError::NumberAlreadyAssigned)
    ));
}

#[test]
fn finalize_sem_autoridade_rejeita() {
    let doc = sample_custody();
    assert!(matches!(
        doc.finalize(),
        Err(DocumentalError::MissingAuthorityContext)
    ));
}

#[test]
fn finalize_sem_numero_rejeita() {
    let mut doc = sample_custody();
    doc.authority_context = Some(sample_authority());
    assert!(matches!(
        doc.finalize(),
        Err(DocumentalError::MissingDocumentNumber)
    ));
}

#[test]
fn finalize_com_pre_condicoes_ok() {
    let mut doc = sample_custody();
    doc.authority_context = Some(sample_authority());
    doc.document_number = Some("2026/001".into());
    doc.status = DocumentStatus::Approved;
    let next = doc.finalize().unwrap();
    assert_eq!(next, DocumentStatus::Finalized);
}

#[test]
fn is_finalized_cobre_todos_estados_terminais() {
    let mut doc = sample_custody();
    doc.status = DocumentStatus::Finalized;
    assert!(doc.is_finalized());
    doc.status = DocumentStatus::Archived;
    assert!(doc.is_finalized());
    doc.status = DocumentStatus::Annulled;
    assert!(doc.is_finalized());
    doc.status = DocumentStatus::Draft;
    assert!(!doc.is_finalized());
}

#[test]
fn transition_invalida_devolve_erro() {
    let doc = sample_custody(); // Draft
    let err = doc.transition_to(DocumentStatus::Finalized).unwrap_err();
    assert!(matches!(
        err,
        DocumentalError::InvalidStatusTransition(_, _)
    ));
}

// ── DocumentEventType ─────────────────────────────────────────────────────────

#[test]
fn event_type_from_str_round_trip() {
    let types = [
        "created",
        "status_changed",
        "payload_updated",
        "number_assigned",
        "ndf_rendered",
        "signed",
        "relation_added",
        "attachment_added",
        "archived",
        "annulled",
    ];
    for s in types {
        let t = DocumentEventType::from_str(s).unwrap();
        assert_eq!(t.as_str(), s);
    }
}

#[test]
fn event_type_try_from_desconhecido_devolve_err() {
    assert!(DocumentEventType::try_from("unknown_type").is_err());
}

// ── verify_event_chain ────────────────────────────────────────────────────────

fn make_event(id: &str, prev: Option<&str>) -> DocumentEvent {
    DocumentEvent {
        id: DocumentEventId::new(id).unwrap(),
        document_id: DocumentId::new("doc-001").unwrap(),
        event_type: DocumentEventType::Created,
        actor: EventActor::Operator {
            user_id: UserId::new("user-1").unwrap(),
            position_id: OrgPositionId::new("pos-1").unwrap(),
        },
        occurred_at: Utc::now(),
        previous_hash: prev.map(str::to_string),
        data_json: None,
    }
}

#[test]
fn chain_vazia_e_valida() {
    assert!(crate::verify_event_chain(&[]).is_ok());
}

#[test]
fn chain_primeiro_evento_sem_previous_hash() {
    let events = vec![make_event("ev-1", None)];
    assert!(crate::verify_event_chain(&events).is_ok());
}

#[test]
fn chain_primeiro_evento_com_previous_hash_rejeita() {
    let events = vec![make_event("ev-1", Some("hash-anterior"))];
    assert!(matches!(
        crate::verify_event_chain(&events),
        Err(DocumentalError::EventChainBroken(_))
    ));
}

#[test]
fn chain_segundo_evento_sem_previous_hash_rejeita() {
    let events = vec![make_event("ev-1", None), make_event("ev-2", None)];
    assert!(matches!(
        crate::verify_event_chain(&events),
        Err(DocumentalError::EventChainBroken(_))
    ));
}

#[test]
fn chain_dois_eventos_validos() {
    let events = vec![
        make_event("ev-1", None),
        make_event("ev-2", Some("hash-ev1")),
    ];
    assert!(crate::verify_event_chain(&events).is_ok());
}

// ── validate_document_package ─────────────────────────────────────────────────

#[test]
fn package_valido_aceita() {
    assert!(validate_document_package(&sample_package()).is_ok());
}

#[test]
fn package_sem_document_id_rejeita() {
    let mut p = sample_package();
    p.document_id = "".into();
    assert!(validate_document_package(&p).is_err());
}

#[test]
fn package_sem_artefactos_rejeita() {
    let mut p = sample_package();
    p.artefacts.clear();
    assert!(matches!(
        validate_document_package(&p),
        Err(DocumentalError::InvalidPackage(_))
    ));
}

#[test]
fn package_artefacto_sem_hash_rejeita() {
    let mut p = sample_package();
    p.artefacts[0].hash_result.hash = "".into();
    assert!(matches!(
        validate_document_package(&p),
        Err(DocumentalError::InvalidPackage(_))
    ));
}

// ── AttachmentKind ────────────────────────────────────────────────────────────

#[test]
fn attachment_kind_round_trip() {
    for s in ["annex", "incoming"] {
        let k = AttachmentKind::from_str(s).unwrap();
        assert_eq!(k.as_str(), s);
    }
}

#[test]
fn attachment_kind_try_from_desconhecido_devolve_err() {
    assert!(AttachmentKind::try_from("unknown").is_err());
}

// ── DocumentTemplate ──────────────────────────────────────────────────────────

fn sample_template() -> DocumentTemplate {
    DocumentTemplate {
        id: TemplateId::new("tpl-001").unwrap(),
        code: "OFICIO_AT".into(),
        document_type: "oficio".into(),
        version: "v1".into(),
        content_ndt: "template content".into(),
        content_hash: "sha256:abc".into(),
        status: TemplateStatus::Draft,
        created_at: Utc::now(),
        created_by: sample_authority(),
    }
}

#[test]
fn template_draft_pode_ser_activado() {
    assert!(sample_template().activate().is_ok());
}

#[test]
fn template_active_e_imutavel() {
    let mut tpl = sample_template();
    tpl.status = TemplateStatus::Active;
    assert!(matches!(
        tpl.activate(),
        Err(DocumentalError::TemplateImmutable)
    ));
}

#[test]
fn template_deprecated_nao_activavel() {
    let mut tpl = sample_template();
    tpl.status = TemplateStatus::Deprecated;
    assert!(matches!(
        tpl.activate(),
        Err(DocumentalError::TemplateNotActivatable)
    ));
}

#[test]
fn template_verify_content_hash_ok() {
    let tpl = sample_template();
    assert!(tpl.verify_content_hash("sha256:abc").is_ok());
    assert!(matches!(
        tpl.verify_content_hash("sha256:diferente"),
        Err(DocumentalError::ContentHashMismatch)
    ));
}

// ── NdfRecord ─────────────────────────────────────────────────────────────────

#[test]
fn ndf_verify_integrity_ok() {
    let rec = NdfRecord {
        id: NdfRecordId::new("ndf-001").unwrap(),
        document_id: DocumentId::new("doc-001").unwrap(),
        ndf_json: "{\"a\":1}".into(),
        ndf_hash: "sha256:xyz".into(),
        template_hash: "sha256:tpl".into(),
        rendered_at: Utc::now(),
        rendered_by: sample_authority(),
    };
    assert!(rec.verify_integrity("sha256:xyz").is_ok());
    assert!(matches!(
        rec.verify_integrity("sha256:errado"),
        Err(DocumentalError::NdfHashMismatch)
    ));
}

// ── DocumentAttachment ────────────────────────────────────────────────────────

#[test]
fn attachment_validate_size_zero_rejeita() {
    let att = DocumentAttachment {
        id: AttachmentId::new("att-001").unwrap(),
        document_id: DocumentId::new("doc-001").unwrap(),
        kind: AttachmentKind::Incoming,
        original_filename: "doc.pdf".into(),
        content_type: "application/pdf".into(),
        content_hash: "sha256:abc".into(),
        size_bytes: 0,
        description: None,
        stored_at: Utc::now(),
        stored_by: sample_authority(),
    };
    assert!(matches!(
        att.validate(),
        Err(DocumentalError::EmptyField(_))
    ));
}

#[test]
fn attachment_storage_filename_e_content_hash() {
    let att = DocumentAttachment {
        id: AttachmentId::new("att-001").unwrap(),
        document_id: DocumentId::new("doc-001").unwrap(),
        kind: AttachmentKind::Annex,
        original_filename: "resultado.pdf".into(),
        content_type: "application/pdf".into(),
        content_hash: "sha256:deadbeef".into(),
        size_bytes: 512,
        description: None,
        stored_at: Utc::now(),
        stored_by: sample_authority(),
    };
    assert_eq!(att.storage_filename(), "sha256:deadbeef");
}

// ── AuthorityContext::from_user_context ───────────────────────────────────────

fn sample_identity() -> UserIdentity {
    UserIdentity {
        user_id: "joao.silva".into(),
        username: "joao.silva".into(),
        display_name: "João Silva".into(),
        email: None,
        role: UserRole::Utilizador,
    }
}

fn context_with_position() -> UserContext {
    let pos = OrgPositionRef::new("pos-001", "unit-001", "comp-001", None).unwrap();
    core_rh::identity::resolve_current_user(sample_identity())
        .unwrap()
        .with_position(pos)
}

fn context_without_position() -> UserContext {
    core_rh::identity::resolve_current_user(sample_identity()).unwrap()
}

#[test]
fn authority_from_user_context_com_posicao_ok() {
    let ctx = context_with_position();
    let auth = AuthorityContext::from_user_context(&ctx, Utc::now());
    assert!(auth.is_ok());
    let auth = auth.unwrap();
    assert_eq!(auth.user_id.as_str(), "joao.silva");
    assert_eq!(auth.position_id.as_str(), "pos-001");
    assert_eq!(auth.unit_id.as_str(), "unit-001");
    assert_eq!(auth.competency_id.as_str(), "comp-001");
    assert!(auth.delegation_id.is_none());
}

#[test]
fn authority_from_user_context_sem_posicao_rejeita() {
    let ctx = context_without_position();
    assert!(matches!(
        AuthorityContext::from_user_context(&ctx, Utc::now()),
        Err(DocumentalError::MissingAuthorityContext)
    ));
}

#[test]
fn authority_from_user_context_com_delegacao_ok() {
    let pos =
        OrgPositionRef::new("pos-001", "unit-001", "comp-001", Some("del-001".into())).unwrap();
    let ctx = core_rh::identity::resolve_current_user(sample_identity())
        .unwrap()
        .with_position(pos);
    let auth = AuthorityContext::from_user_context(&ctx, Utc::now()).unwrap();
    assert!(auth.delegation_id.is_some());
    assert_eq!(auth.delegation_id.unwrap().as_str(), "del-001");
}

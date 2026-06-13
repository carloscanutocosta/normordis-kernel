//! Testes unitários de `core-documental`.
//!
//! Cobrem invariantes de domínio: custódia, tipos documentais, conteúdo imutável,
//! estados custodiais, cadeia de eventos, templates, pacotes e anexos.

use chrono::Utc;

use crate::{
    archive::{NdfRecord, NdfRecordId},
    attachment::{AttachmentId, AttachmentKind, DocumentAttachment},
    authority::AuthoritySnapshot,
    custody::{
        DocumentContent, DocumentCustody, DocumentId, DocumentOrigin, DocumentStatus,
        DocumentTypeCode, EntryChannel, IntakeSpec, RetentionClass, RetentionPolicy,
        ValidationCode,
    },
    error::DocumentalError,
    events::{
        AccessPurpose, DocumentEvent, DocumentEventId, DocumentEventType, EventActor, EventFilter,
    },
    package::{
        validate_document_package, Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef,
    },
    service::authority_from_user_context,
    template::{DocumentTemplate, TemplateId, TemplateStatus},
};

use core_rh::{
    identity::{resolve_current_user, UserContext, UserIdentity},
    org::OrgPositionRef,
    UserRole,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sample_authority() -> AuthoritySnapshot {
    AuthoritySnapshot {
        user_id: "user-1".into(),
        position_id: "pos-1".into(),
        unit_id: "unit-1".into(),
        competency_id: "comp-1".into(),
        delegation_id: None,
        captured_at: Utc::now(),
    }
}

fn sample_custody() -> DocumentCustody {
    DocumentCustody {
        id: DocumentId::new("doc-001").unwrap(),
        document_type: DocumentTypeCode::new(DocumentTypeCode::OFICIO).unwrap(),
        document_number: None,
        validation_code: ValidationCode::new("NORD-K7P9-4Q2M-8X").unwrap(),
        template_id: Some(TemplateId::new("tpl-001").unwrap()),
        template_version: Some("v1".into()),
        origin: DocumentOrigin::Normordis,
        entry_channel: EntryChannel::new("core-ingest/normordis").unwrap(),
        authority: sample_authority(),
        content: Some(DocumentContent::new(r#"{"assunto":"Teste"}"#).unwrap()),
        status: DocumentStatus::Active,
        retention_policy: RetentionPolicy::permanent(),
        received_at: Utc::now(),
        custodied_at: Utc::now(),
    }
}

fn sample_template() -> DocumentTemplate {
    DocumentTemplate {
        id: TemplateId::new("tpl-001").unwrap(),
        code: "OFICIO_AT".into(),
        document_type: DocumentTypeCode::new(DocumentTypeCode::OFICIO).unwrap(),
        version: "v1".into(),
        content_ndt: "template content".into(),
        content_hash: "sha256:abc".into(),
        status: TemplateStatus::Active,
        created_at: Utc::now(),
        created_by: sample_authority(),
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

fn make_event(id: &str, prev: Option<&str>) -> DocumentEvent {
    DocumentEvent {
        id: DocumentEventId::new(id).unwrap(),
        document_id: DocumentId::new("doc-001").unwrap(),
        event_type: DocumentEventType::CustodyAccepted,
        actor: EventActor::Operator {
            user_id: "user-1".into(),
            position_id: "pos-1".into(),
        },
        occurred_at: Utc::now(),
        previous_hash: prev.map(str::to_string),
        data_json: None,
    }
}

// ── DocumentTypeCode ──────────────────────────────────────────────────────────

#[test]
fn document_type_code_aceita_tipo_valido() {
    let t = DocumentTypeCode::new("oficio").unwrap();
    assert_eq!(t.as_str(), "oficio");
}

#[test]
fn document_type_code_normaliza_para_minusculas() {
    let t = DocumentTypeCode::new("  Oficio  ").unwrap();
    assert_eq!(t.as_str(), "oficio");
}

#[test]
fn document_type_code_rejeita_vazio() {
    assert!(matches!(
        DocumentTypeCode::new(""),
        Err(DocumentalError::EmptyField(_))
    ));
    assert!(matches!(
        DocumentTypeCode::new("   "),
        Err(DocumentalError::EmptyField(_))
    ));
}

#[test]
fn document_type_code_rejeita_caracteres_invalidos() {
    assert!(matches!(
        DocumentTypeCode::new("ofício"), // com acento
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
    assert!(matches!(
        DocumentTypeCode::new("tipo documental"),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
}

#[test]
fn document_type_code_constantes_sao_validas() {
    for code in [
        DocumentTypeCode::OFICIO,
        DocumentTypeCode::INFORMACAO,
        DocumentTypeCode::PARECER,
        DocumentTypeCode::NOTIFICACAO,
        DocumentTypeCode::DECLARACAO,
        DocumentTypeCode::CERTIDAO,
        DocumentTypeCode::DESPACHO,
        DocumentTypeCode::REQUERIMENTO,
    ] {
        assert!(DocumentTypeCode::new(code).is_ok());
    }
}

// ── DocumentContent ───────────────────────────────────────────────────────────

#[test]
fn document_content_aceita_json_valido() {
    let c = DocumentContent::new(r#"{"assunto":"Teste"}"#).unwrap();
    assert_eq!(c.as_str(), r#"{"assunto":"Teste"}"#);
}

#[test]
fn document_content_rejeita_vazio() {
    assert!(matches!(
        DocumentContent::new(""),
        Err(DocumentalError::EmptyField(_))
    ));
}

// ── ValidationCode ────────────────────────────────────────────────────────────

#[test]
fn validation_code_aceita_codigo_valido() {
    let code = ValidationCode::new("NORD-K7P9-4Q2M-8X").unwrap();
    assert_eq!(code.as_str(), "NORD-K7P9-4Q2M-8X");
}

#[test]
fn validation_code_rejeita_vazio() {
    assert!(matches!(
        ValidationCode::new(""),
        Err(DocumentalError::EmptyField(_))
    ));
}

// ── ValidationCode — formato Crockford Base32 ─────────────────────────────────

#[test]
fn validation_code_rejeita_prefixo_errado() {
    assert!(matches!(
        ValidationCode::new("ATCUD-K7P9-4Q2M-8X"),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
}

#[test]
fn validation_code_rejeita_caracteres_ambiguos_ilou() {
    // I, L, O, U são excluídos do alfabeto Crockford Base32
    for code in [
        "NORD-IIII-4Q2M-8X",
        "NORD-K7P9-LLLL-8X",
        "NORD-K7P9-OOOO-8X",
        "NORD-K7P9-4Q2M-UU",
    ] {
        assert!(
            matches!(
                ValidationCode::new(code),
                Err(DocumentalError::InvalidIdentifier { .. })
            ),
            "deveria rejeitar: {code}"
        );
    }
}

#[test]
fn validation_code_rejeita_segmentos_de_tamanho_errado() {
    // Terceiro grupo deve ter 4 chars, não 3
    assert!(matches!(
        ValidationCode::new("NORD-K7P9-4Q2-8X"),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
    // Quarto grupo deve ter 2 chars, não 3
    assert!(matches!(
        ValidationCode::new("NORD-K7P9-4Q2M-8XA"),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
}

#[test]
fn validation_code_rejeita_grupos_a_menos() {
    assert!(matches!(
        ValidationCode::new("NORD-K7P9-4Q2M"),
        Err(DocumentalError::InvalidIdentifier { .. })
    ));
}

#[test]
fn validation_code_normaliza_para_maiusculas() {
    let code = ValidationCode::new("nord-k7p9-4q2m-8x").unwrap();
    assert_eq!(code.as_str(), "NORD-K7P9-4Q2M-8X");
}

#[test]
fn validation_code_generate_produz_formato_valido() {
    for _ in 0..20 {
        let code = ValidationCode::generate();
        assert!(
            ValidationCode::new(code.as_str()).is_ok(),
            "código inválido: {}",
            code.as_str()
        );
        let parts: Vec<&str> = code.as_str().split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "NORD");
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 2);
    }
}

#[test]
fn validation_code_generate_sem_caracteres_ambiguos() {
    for _ in 0..50 {
        let code = ValidationCode::generate();
        let payload = &code.as_str()[5..]; // remove "NORD-"
        for c in payload.chars() {
            assert!(
                !matches!(c, 'I' | 'L' | 'O' | 'U'),
                "carácter ambíguo '{c}' gerado: {}",
                code.as_str()
            );
        }
    }
}

// ── RetentionPolicy ───────────────────────────────────────────────────────────

#[test]
fn retention_permanent_nunca_expira() {
    let policy = RetentionPolicy::permanent();
    assert!(policy.is_permanent());
    assert!(policy.expires_at.is_none());
    assert!(!policy.is_expired(Utc::now()));
}

#[test]
fn retention_temporary_tem_data_expiracao() {
    let ts = Utc::now();
    let policy = RetentionPolicy::temporary(10, ts);
    assert!(!policy.is_permanent());
    assert!(policy.expires_at.is_some());
    assert!(matches!(
        policy.class,
        RetentionClass::Temporary { years: 10 }
    ));
}

#[test]
fn retention_temporary_expira_apos_periodo() {
    use chrono::Duration;
    let ts = Utc::now() - Duration::days(366 * 11); // 11 anos atrás
    let policy = RetentionPolicy::temporary(10, ts);
    assert!(policy.is_expired(Utc::now()));
}

#[test]
fn retention_temporary_nao_expira_antes_do_prazo() {
    let ts = Utc::now();
    let policy = RetentionPolicy::temporary(10, ts);
    assert!(!policy.is_expired(Utc::now()));
}

// ── DocumentCustody::accept ───────────────────────────────────────────────────

#[test]
fn custody_accept_factory_constroi_active() {
    let ts = Utc::now();
    let spec = IntakeSpec {
        id: DocumentId::new("doc-factory-001").unwrap(),
        document_type: DocumentTypeCode::new(DocumentTypeCode::OFICIO).unwrap(),
        validation_code: ValidationCode::new("NORD-K7P9-4Q2M-8X").unwrap(),
        origin: DocumentOrigin::Normordis,
        entry_channel: EntryChannel::new("ingest").unwrap(),
        authority: sample_authority(),
        content: None,
        template_id: None,
        template_version: None,
        retention_policy: RetentionPolicy::permanent(),
        received_at: ts,
        custodied_at: ts,
    };
    let doc = DocumentCustody::accept(spec).unwrap();
    assert_eq!(doc.status, DocumentStatus::Active);
    assert!(doc.document_number.is_none());
}

#[test]
fn custody_accept_factory_rejeita_validation_code_invalido() {
    let ts = Utc::now();
    let spec = IntakeSpec {
        id: DocumentId::new("doc-factory-002").unwrap(),
        document_type: DocumentTypeCode::new(DocumentTypeCode::OFICIO).unwrap(),
        validation_code: ValidationCode("INVALIDO".into()), // bypass new() propositadamente
        origin: DocumentOrigin::Normordis,
        entry_channel: EntryChannel::new("ingest").unwrap(),
        authority: sample_authority(),
        content: None,
        template_id: None,
        template_version: None,
        retention_policy: RetentionPolicy::permanent(),
        received_at: ts,
        custodied_at: ts,
    };
    assert!(DocumentCustody::accept(spec).is_err());
}

// ── DocumentEvent::canonical_bytes ────────────────────────────────────────────

#[test]
fn canonical_bytes_primeiro_evento_usa_genesis() {
    let ev = make_event("ev-genesis", None);
    let bytes = ev.canonical_bytes();
    let s = String::from_utf8(bytes).unwrap();
    assert!(
        s.ends_with(":GENESIS"),
        "esperado sufixo GENESIS, obteve: {s}"
    );
    assert!(s.starts_with("ev-genesis:"));
}

#[test]
fn canonical_bytes_evento_subsequente_usa_hash_anterior() {
    let ev = make_event("ev-2", Some("hash-do-ev-1"));
    let bytes = ev.canonical_bytes();
    let s = String::from_utf8(bytes).unwrap();
    assert!(
        s.ends_with(":hash-do-ev-1"),
        "esperado sufixo do hash anterior, obteve: {s}"
    );
}

#[test]
fn canonical_bytes_formato_determinista() {
    let ev1 = make_event("ev-a", None);
    let ev2 = make_event("ev-a", None);
    assert_eq!(ev1.canonical_bytes(), ev2.canonical_bytes());
}

#[test]
fn canonical_bytes_diferentes_ids_diferentes_bytes() {
    let ev1 = make_event("ev-x", None);
    let ev2 = make_event("ev-y", None);
    assert_ne!(ev1.canonical_bytes(), ev2.canonical_bytes());
}

// ── DocumentOrigin ────────────────────────────────────────────────────────────

#[test]
fn document_origin_round_trip() {
    for s in [
        "normordis",
        "email",
        "scanner",
        "upload",
        "api",
        "interop",
        "legacy",
    ] {
        let origin = DocumentOrigin::from_str(s).unwrap();
        assert_eq!(origin.as_str(), s);
    }
}

#[test]
fn document_origin_desconhecido_devolve_none() {
    assert!(DocumentOrigin::from_str("fax").is_none());
}

// ── DocumentId ────────────────────────────────────────────────────────────────

#[test]
fn document_id_aceita_chave_segura() {
    let id = DocumentId::new("doc-oficio_001.2026").unwrap();
    assert_eq!(id.as_str(), "doc-oficio_001.2026");
}

#[test]
fn document_id_rejeita_traversal_e_separadores() {
    for unsafe_id in [
        "../etc/passwd",
        "..\\config",
        "docs/001",
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
fn document_id_rejeita_caracteres_invalidos() {
    for unsafe_id in ["C:temp", "doc 001", "doc#001"] {
        assert!(matches!(
            DocumentId::new(unsafe_id),
            Err(DocumentalError::InvalidIdentifier { .. })
        ));
    }
}

#[test]
fn document_id_deserialize_rejeita_valor_inseguro() {
    let result = serde_json::from_str::<DocumentId>(r#""..\config.json""#);
    assert!(result.is_err());
}

// ── DocumentStatus ────────────────────────────────────────────────────────────

#[test]
fn transicoes_custodiais_validas_de_active() {
    assert!(DocumentStatus::Active.can_transition_to(&DocumentStatus::Archived));
    assert!(DocumentStatus::Active.can_transition_to(&DocumentStatus::Revoked));
    assert!(DocumentStatus::Active.can_transition_to(&DocumentStatus::Superseded));
    assert!(!DocumentStatus::Active.can_transition_to(&DocumentStatus::Active));
}

#[test]
fn transicao_archived_para_revoked() {
    assert!(DocumentStatus::Archived.can_transition_to(&DocumentStatus::Revoked));
    assert!(!DocumentStatus::Archived.can_transition_to(&DocumentStatus::Active));
    assert!(!DocumentStatus::Archived.can_transition_to(&DocumentStatus::Superseded));
}

#[test]
fn estados_terminais_nao_transitam() {
    assert!(!DocumentStatus::Revoked.can_transition_to(&DocumentStatus::Active));
    assert!(!DocumentStatus::Superseded.can_transition_to(&DocumentStatus::Active));
    assert!(!DocumentStatus::Superseded.can_transition_to(&DocumentStatus::Archived));
}

#[test]
fn document_status_from_str_round_trip() {
    for s in ["active", "archived", "revoked", "superseded"] {
        let status = DocumentStatus::from_str(s).unwrap();
        assert_eq!(status.as_str(), s);
    }
}

#[test]
fn document_status_workflow_states_nao_existem() {
    assert!(DocumentStatus::from_str("draft").is_none());
    assert!(DocumentStatus::from_str("finalized").is_none());
    assert!(DocumentStatus::from_str("pending_approval").is_none());
}

// ── DocumentCustody ───────────────────────────────────────────────────────────

#[test]
fn custody_validate_ok() {
    assert!(sample_custody().validate().is_ok());
}

#[test]
fn custody_validate_rejeita_document_type_vazio() {
    let mut doc = sample_custody();
    doc.document_type = DocumentTypeCode("  ".into());
    assert!(matches!(
        doc.validate(),
        Err(DocumentalError::EmptyField(_))
    ));
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
fn custody_is_in_active_custody() {
    assert!(sample_custody().is_in_active_custody());
    let mut archived = sample_custody();
    archived.status = DocumentStatus::Archived;
    assert!(!archived.is_in_active_custody());
}

#[test]
fn custody_transition_to_valida() {
    let doc = sample_custody();
    let next = doc.transition_to(DocumentStatus::Archived).unwrap();
    assert_eq!(next, DocumentStatus::Archived);
}

#[test]
fn custody_transition_invalida_devolve_erro() {
    let doc = sample_custody();
    assert!(matches!(
        doc.transition_to(DocumentStatus::Active),
        Err(DocumentalError::InvalidStatusTransition(_, _))
    ));
    let mut revoked = sample_custody();
    revoked.status = DocumentStatus::Revoked;
    assert!(matches!(
        revoked.transition_to(DocumentStatus::Archived),
        Err(DocumentalError::InvalidStatusTransition(_, _))
    ));
}

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
fn assign_number_fora_de_active_rejeita() {
    let mut doc = sample_custody();
    doc.status = DocumentStatus::Archived;
    assert!(matches!(
        doc.assign_number("2026/001"),
        Err(DocumentalError::DocumentImmutable)
    ));
}

// ── AccessPurpose ─────────────────────────────────────────────────────────────

#[test]
fn access_purpose_round_trip() {
    for s in [
        "consultation",
        "inspection",
        "export",
        "legal_proceeding",
        "audit",
        "internal_review",
    ] {
        let purpose = AccessPurpose::from_str(s).unwrap();
        assert_eq!(purpose.as_str(), s);
    }
}

#[test]
fn access_purpose_desconhecido_devolve_err() {
    assert!(AccessPurpose::try_from("leitura").is_err());
}

// ── EventFilter ───────────────────────────────────────────────────────────────

#[test]
fn event_filter_default_sem_restricoes() {
    let f = EventFilter::default();
    assert!(f.event_type.is_none());
    assert!(f.from.is_none());
    assert!(f.until.is_none());
    assert_eq!(f.limit, 100);
    assert_eq!(f.offset, 0);
}

// ── DocumentEventType ─────────────────────────────────────────────────────────

#[test]
fn event_type_from_str_round_trip() {
    for s in [
        "custody_accepted",
        "number_assigned",
        "accessed",
        "relation_added",
        "attachment_added",
        "status_changed",
    ] {
        let t = DocumentEventType::from_str(s).unwrap();
        assert_eq!(t.as_str(), s);
    }
}

#[test]
fn event_type_workflow_states_nao_existem() {
    assert!(DocumentEventType::from_str("created").is_none());
    assert!(DocumentEventType::from_str("payload_updated").is_none());
    assert!(DocumentEventType::from_str("signed").is_none());
}

// ── verify_event_chain ────────────────────────────────────────────────────────

#[test]
fn chain_vazia_e_valida() {
    assert!(crate::verify_event_chain(&[]).is_ok());
}

#[test]
fn chain_primeiro_evento_sem_previous_hash() {
    assert!(crate::verify_event_chain(&[make_event("ev-1", None)]).is_ok());
}

#[test]
fn chain_primeiro_evento_com_previous_hash_rejeita() {
    assert!(matches!(
        crate::verify_event_chain(&[make_event("ev-1", Some("hash"))]),
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

// ── DocumentTemplate ──────────────────────────────────────────────────────────

#[test]
fn template_e_sempre_imutavel() {
    assert!(sample_template().is_immutable());
    let mut tpl = sample_template();
    tpl.status = TemplateStatus::Deprecated;
    assert!(tpl.is_immutable());
}

#[test]
fn template_status_round_trip() {
    for s in ["active", "deprecated"] {
        let status = TemplateStatus::from_str(s).unwrap();
        assert_eq!(status.as_str(), s);
    }
}

#[test]
fn template_status_draft_nao_existe() {
    assert!(TemplateStatus::from_str("draft").is_none());
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

// ── AuthoritySnapshot::from_user_context ─────────────────────────────────────

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
    resolve_current_user(sample_identity())
        .unwrap()
        .with_position(pos)
}

fn context_without_position() -> UserContext {
    resolve_current_user(sample_identity()).unwrap()
}

#[test]
fn authority_from_user_context_com_posicao_ok() {
    let ctx = context_with_position();
    let snap = authority_from_user_context(&ctx, Utc::now()).unwrap();
    assert_eq!(snap.user_id, "joao.silva");
    assert_eq!(snap.position_id, "pos-001");
    assert_eq!(snap.unit_id, "unit-001");
    assert_eq!(snap.competency_id, "comp-001");
    assert!(snap.delegation_id.is_none());
}

#[test]
fn authority_from_user_context_sem_posicao_rejeita() {
    let ctx = context_without_position();
    assert!(matches!(
        authority_from_user_context(&ctx, Utc::now()),
        Err(DocumentalError::MissingAuthorityContext)
    ));
}

#[test]
fn authority_from_user_context_com_delegacao_ok() {
    let pos =
        OrgPositionRef::new("pos-001", "unit-001", "comp-001", Some("del-001".into())).unwrap();
    let ctx = resolve_current_user(sample_identity())
        .unwrap()
        .with_position(pos);
    let snap = authority_from_user_context(&ctx, Utc::now()).unwrap();
    assert_eq!(snap.delegation_id.as_deref(), Some("del-001"));
}

// ── AuthoritySnapshot campos são primitivos ───────────────────────────────────

#[test]
fn authority_snapshot_campos_sao_strings() {
    let snap = sample_authority();
    // Confirma que os campos são String nativos — sem tipos de outros bounded contexts
    let _: &str = &snap.user_id;
    let _: &str = &snap.position_id;
    let _: &str = &snap.unit_id;
    let _: &str = &snap.competency_id;
}

// ── EventActor sem tipos externos ─────────────────────────────────────────────

#[test]
fn event_actor_operator_usa_strings() {
    let actor = EventActor::Operator {
        user_id: "user-1".into(),
        position_id: "pos-1".into(),
    };
    if let EventActor::Operator {
        user_id,
        position_id,
    } = actor
    {
        assert_eq!(user_id, "user-1");
        assert_eq!(position_id, "pos-1");
    }
}

#[test]
fn event_actor_authority_usa_authority_snapshot() {
    let actor = EventActor::Authority(sample_authority());
    assert!(matches!(actor, EventActor::Authority(_)));
}

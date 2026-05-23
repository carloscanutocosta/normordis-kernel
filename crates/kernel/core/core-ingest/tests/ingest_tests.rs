use chrono::{TimeZone, Utc};
use core_documental::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};
use core_exports::{build_export_receipt, canonical_bytes, BuildSnapshotConfig, SourceRef};
use core_ingest::{
    build_ingest_audit_event, process_export_snapshot, validate_ingest_evidence,
    validate_ingest_request, AuditEvidence, DeterministicScanner, HashEvidence, IngestConfig,
    IngestError, IngestEvidence, IngestRequest, IngestSource, MemoryRouter,
    RouteEvidence, RouteInput, RouteResult, Router, ScanEvidence, ValidationEvidence,
    DECISION_ACCEPTED, DECISION_REJECTED, HASH_MISMATCH, INVALID_REQUEST, MISSING_FIELD,
    ROUTE_UNAVAILABLE, SCAN_FAILED, SCAN_REJECTED,
};
use core_validation::sha256_bytes;
use serde_json::json;
use std::collections::HashMap;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap()
}

fn sample_package() -> DocumentPackage {
    let t = ts();
    DocumentPackage {
        document_id: "doc:config_profile:dev:1.0.0".into(),
        created_at: t,
        template: TemplateRef {
            template_id: "config-profile-export".into(),
            template_version: "v1".into(),
            valid_at: None,
        },
        engine: EngineRef {
            engine_id: "config-json".into(),
            engine_version: "v1".into(),
        },
        artefacts: vec![Artefact {
            kind: "config_profile_json".into(),
            artefact_ref: "config-profile:dev:1.0.0".into(),
            hash_result: HashResult {
                algorithm: "SHA-256".into(),
                hash: "sha256:abc123".into(),
                timestamp: t,
                input_kind: Some("config_profile".into()),
                input_ref: Some("dev@1.0.0".into()),
                meta: None,
            },
            mime: Some("application/json".into()),
            size_bytes: Some(64),
        }],
        subject: Some(json!({ "kind": "config_profile", "profile_id": "dev" })),
        meta: Some(json!({ "source": "unit-test", "version": "1.0.0" })),
    }
}

fn sample_snapshot() -> core_exports::ExportSnapshot {
    let source = SourceRef {
        kind: "config_profile".into(),
        subject_id: "dev".into(),
        version: "1.0.0".into(),
    };
    let cfg = BuildSnapshotConfig {
        exported_at: Some(ts()),
        actor: "daemon:apid".into(),
        correlation_id: "corr-sample".into(),
        transport: None,
    };
    build_export_receipt(sample_package(), source, cfg)
        .expect("sample snapshot deve ser válido")
        .snapshot
}

fn sample_request() -> IngestRequest {
    let snapshot = sample_snapshot();
    let payload = canonical_bytes(&snapshot).expect("canonical_bytes");
    let hash = format!("sha256:{}", sha256_bytes(&payload));
    IngestRequest {
        request_id: "ing:req:test:001".into(),
        received_at: ts(),
        source: IngestSource {
            kind: "config_export_bundle".into(),
            subject_id: "dev".into(),
            version: "1.0.0".into(),
        },
        expected_hash: hash,
        bundle: snapshot,
        meta: None,
    }
}

fn default_cfg() -> IngestConfig {
    IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: Some(|| Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 1).unwrap()),
        allowed_source_kinds: None,
    }
}

fn minimal_valid_evidence() -> IngestEvidence {
    IngestEvidence {
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        decision: DECISION_REJECTED.into(),
        received_at: ts(),
        processed_at: ts(),
        source: IngestSource {
            kind: "config_export_bundle".into(),
            subject_id: "dev".into(),
            version: "1.0.0".into(),
        },
        bundle_ref: "snap-ref-001".into(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            expected_hash: "sha256:abc".into(),
            actual_hash: "sha256:abc".into(),
            verified: true,
        },
        scan: ScanEvidence { adapter: "not_run".into(), verdict: "not_run".into(), reason: None },
        validation: ValidationEvidence {
            contract: "core-exports/export_snapshot".into(),
            valid: false,
        },
        route: RouteEvidence::default(),
        audit: AuditEvidence {
            required: true,
            emitted: false,
            action: "ingest.rejected".into(),
            event_id: None,
        },
        meta: None,
    }
}

// ── Testes de processo completo ────────────────────────────────────────────────

#[test]
fn aceita_bundle_config_valido() {
    let req = sample_request();
    let outcome = process_export_snapshot(&req, "corr-ingest-ok", &default_cfg())
        .expect_accepted("bundle válido deve ser aceite");

    assert_eq!(outcome.evidence.decision, DECISION_ACCEPTED);
    assert!(outcome.evidence.route.routed);
    assert_eq!(outcome.audit_event.event_type, "ingest.accepted");
    assert!(outcome.evidence.audit.emitted, "emitted deve ser true no caminho canónico");
    assert!(outcome.evidence.audit.event_id.is_some());
}

#[test]
fn rejeita_hash_mismatch() {
    let mut req = sample_request();
    req.expected_hash =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();

    let (outcome, err) =
        process_export_snapshot(&req, "corr-ingest-hash", &default_cfg()).expect_rejected("hash errado");

    assert_eq!(outcome.evidence.decision, DECISION_REJECTED);
    assert_eq!(outcome.audit_event.event_type, "ingest.rejected");
    assert_eq!(err.code(), HASH_MISMATCH);
}

#[test]
fn rejeita_scan_rejeitado() {
    let req = sample_request();
    let payload = canonical_bytes(&req.bundle).unwrap();
    let actual_hash = format!("sha256:{}", sha256_bytes(&payload));

    let mut rejected = HashMap::new();
    rejected.insert(actual_hash, "malware-simulated".into());

    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner {
            adapter_name: "deterministic".into(),
            rejected_hashes: rejected,
        })),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) =
        process_export_snapshot(&req, "corr-ingest-scan", &cfg).expect_rejected("scan deve rejeitar");

    assert_eq!(err.code(), SCAN_REJECTED);
    assert_eq!(outcome.evidence.scan.adapter, "deterministic");
    assert_eq!(outcome.evidence.scan.verdict, "rejected");
    assert_eq!(outcome.evidence.scan.reason.as_deref(), Some("malware-simulated"));
}

#[test]
fn rejeita_falha_tecnica_do_scanner() {
    struct ErrorScanner;
    impl core_ingest::ScanAdapter for ErrorScanner {
        fn scan(&self, _: &core_ingest::ScanInput) -> Result<core_ingest::ScanResult, IngestError> {
            Err(IngestError::ScanFailed)
        }
        fn adapter_id(&self) -> &str {
            "clamav"
        }
    }

    let req = sample_request();
    let cfg = IngestConfig {
        scanner: Some(Box::new(ErrorScanner)),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) =
        process_export_snapshot(&req, "corr-ingest-scan-error", &cfg).expect_rejected("scan técnico falhou");

    assert_eq!(err.code(), SCAN_FAILED);
    assert!(err.is_retryable());
    assert_eq!(outcome.evidence.scan.adapter, "clamav");
    assert_eq!(outcome.evidence.scan.verdict, "error");
}

#[test]
fn rejeita_scanner_nao_configurado() {
    let req = sample_request();
    let cfg = IngestConfig {
        scanner: None,
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) =
        process_export_snapshot(&req, "corr-ingest-no-scanner", &cfg).expect_rejected("scanner ausente");

    assert_eq!(err.code(), SCAN_FAILED);
    assert!(err.is_retryable());
    assert_eq!(outcome.evidence.scan.adapter, "not_configured");
    assert_eq!(outcome.evidence.scan.verdict, "error");
}

#[test]
fn bundle_repetido_preserva_route_ref() {
    use std::sync::Arc;

    struct SharedRouter(Arc<MemoryRouter>);
    impl Router for SharedRouter {
        fn route(&self, input: &RouteInput) -> Result<RouteResult, IngestError> {
            self.0.route(input)
        }
    }

    let shared = Arc::new(MemoryRouter::new("ingest/config-bundle"));
    let make_cfg = || IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        router: Some(Box::new(SharedRouter(Arc::clone(&shared)))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let req = sample_request();
    let first =
        process_export_snapshot(&req, "corr-ingest-r1", &make_cfg()).expect_accepted("primeira ingestão");
    let second =
        process_export_snapshot(&req, "corr-ingest-r2", &make_cfg()).expect_accepted("segunda ingestão");

    assert_eq!(
        first.evidence.route.route_ref, second.evidence.route.route_ref,
        "route_ref deve ser idempotente para o mesmo bundle"
    );
}

#[test]
fn rejeita_route_indisponivel() {
    struct FailingRouter;
    impl Router for FailingRouter {
        fn route(&self, _: &RouteInput) -> Result<RouteResult, IngestError> {
            Err(IngestError::RouteUnavailable)
        }
    }

    let req = sample_request();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        router: Some(Box::new(FailingRouter)),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) =
        process_export_snapshot(&req, "corr-ingest-route", &cfg).expect_rejected("route falhou");

    assert_eq!(err.code(), ROUTE_UNAVAILABLE);
    assert!(err.is_retryable());
}

#[test]
fn rejeita_bundle_oversized() {
    let req = sample_request();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: Some(1),
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) =
        process_export_snapshot(&req, "corr-ingest-oversized", &cfg).expect_rejected("bundle grande");

    assert!(matches!(err, IngestError::Oversized { .. }));
}

#[test]
fn rejeita_correlation_id_vazio() {
    let req = sample_request();
    let (_, err) =
        process_export_snapshot(&req, "", &default_cfg()).expect_rejected("correlation_id vazio");

    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn hash_mismatch_mantem_scan_not_run() {
    let mut req = sample_request();
    req.expected_hash =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();

    struct ErrorScanner;
    impl core_ingest::ScanAdapter for ErrorScanner {
        fn scan(&self, _: &core_ingest::ScanInput) -> Result<core_ingest::ScanResult, IngestError> {
            panic!("scanner não deve ser chamado após hash mismatch")
        }
        fn adapter_id(&self) -> &str {
            "clamav"
        }
    }

    let cfg = IngestConfig {
        scanner: Some(Box::new(ErrorScanner)),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, _) =
        process_export_snapshot(&req, "corr-ingest-hash-scan", &cfg).expect_rejected("hash errado");

    assert_eq!(outcome.evidence.scan.adapter, "not_run");
    assert_eq!(outcome.evidence.scan.verdict, "not_run");
}

#[test]
fn rejected_outcome_tem_emitted_true_quando_evidence_valida() {
    let mut req = sample_request();
    req.expected_hash =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();

    let (outcome, _) =
        process_export_snapshot(&req, "corr-ingest-emitted", &default_cfg()).expect_rejected("hash errado");

    assert!(outcome.evidence.audit.emitted, "evidence válida → emitted=true mesmo em rejected");
}

// ── Testes de validate_ingest_request ─────────────────────────────────────────

#[test]
fn valida_request_rejeita_request_id_vazio() {
    let mut req = sample_request();
    req.request_id = String::new();
    let err = validate_ingest_request(&req).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_request_aceita_qualquer_source_kind() {
    let mut req = sample_request();
    req.source.kind = "qualquer_kind".into();
    assert!(validate_ingest_request(&req).is_ok(), "validate_ingest_request is kind-agnostic");
}

#[test]
fn rejeita_source_kind_nao_permitido_na_config() {
    let mut req = sample_request();
    req.source.kind = "outro_kind".into();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        router: Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: Some(vec!["config_export_bundle".into()]),
    };
    let (_, err) =
        process_export_snapshot(&req, "corr-kind-test", &cfg).expect_rejected("kind não permitido");
    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn valida_request_rejeita_subject_id_vazio() {
    let mut req = sample_request();
    req.source.subject_id = String::new();
    let err = validate_ingest_request(&req).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_request_rejeita_version_vazia() {
    let mut req = sample_request();
    req.source.version = String::new();
    let err = validate_ingest_request(&req).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_request_rejeita_expected_hash_vazio() {
    let mut req = sample_request();
    req.expected_hash = String::new();
    let err = validate_ingest_request(&req).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_request_rejeita_mismatch_subject_id_bundle() {
    let mut req = sample_request();
    req.source.subject_id = "outro-sujeito".into();
    let err = validate_ingest_request(&req).unwrap_err();
    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn valida_request_aceita_request_valido() {
    let req = sample_request();
    assert!(validate_ingest_request(&req).is_ok());
}

// ── Testes de validate_ingest_evidence ────────────────────────────────────────

#[test]
fn valida_evidence_rejeita_request_id_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.request_id = String::new();
    let err = validate_ingest_evidence(&ev).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_evidence_rejeita_correlation_id_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.correlation_id = String::new();
    let err = validate_ingest_evidence(&ev).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_evidence_rejeita_decision_invalida() {
    let mut ev = minimal_valid_evidence();
    ev.decision = "pendente".into();
    let err = validate_ingest_evidence(&ev).unwrap_err();
    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn valida_evidence_rejeita_bundle_ref_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.bundle_ref = String::new();
    let err = validate_ingest_evidence(&ev).unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn valida_evidence_rejeita_accepted_sem_route() {
    let mut ev = minimal_valid_evidence();
    ev.decision = DECISION_ACCEPTED.into();
    ev.audit.action = "ingest.accepted".into();
    // route.routed = false por default
    let err = validate_ingest_evidence(&ev).unwrap_err();
    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn valida_evidence_aceita_evidence_valida() {
    let ev = minimal_valid_evidence();
    assert!(validate_ingest_evidence(&ev).is_ok());
}

// ── Testes de build_ingest_audit_event ────────────────────────────────────────

#[test]
fn build_audit_event_requer_actor() {
    let req = sample_request();
    let outcome =
        process_export_snapshot(&req, "corr-audit", &default_cfg()).expect_accepted("deve aceitar");

    let err = build_ingest_audit_event(&outcome.evidence, "").unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn build_audit_event_accepted_tem_action_correcta() {
    let req = sample_request();
    let outcome =
        process_export_snapshot(&req, "corr-audit-ok", &default_cfg()).expect_accepted("deve aceitar");

    let event = build_ingest_audit_event(&outcome.evidence, "apid").unwrap();
    assert_eq!(event.event_type, "ingest.accepted");
    let details = event.details_json.unwrap();
    assert_eq!(details["decision"], DECISION_ACCEPTED);
    assert_eq!(details["correlation_id"], "corr-audit-ok");
}

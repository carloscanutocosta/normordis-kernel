use chrono::{TimeZone, Utc};
use core_ingest::{
    build_ingest_audit_event, process_bundle, validate_ingest_bundle, validate_ingest_evidence,
    AuditEvidence, DeterministicScanner, HashEvidence, IngestBundle, IngestConfig, IngestDecision,
    IngestError, IngestEvidence, IngestSource, IngestStoragePort, MemoryStoragePort,
    PassthroughContentValidator, RejectingContentValidator, ScanAdapter, ScanEvidence, ScanInput,
    ScanResult, ValidationEvidence, CONTENT_VALIDATION_FAILED, HASH_MISMATCH, INVALID_REQUEST,
    MISSING_FIELD, OVERSIZED, SCAN_FAILED, SCAN_REJECTED, STORE_FAILED,
};
use core_validation::sha256_bytes;
use std::collections::HashMap;

// ── Helpers ────────────────────────────────────────────────────────────────────

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap()
}

fn sample_raw() -> Vec<u8> {
    br#"{"kind":"test","subject":"unit-test","version":"1.0.0"}"#.to_vec()
}

fn sample_bundle() -> IngestBundle {
    let raw = sample_raw();
    let hash = format!("sha256:{}", sha256_bytes(&raw));
    IngestBundle {
        bundle_id: "bnd:test:001".into(),
        received_at: ts(),
        source: IngestSource {
            kind: "test-document".into(),
            subject_id: "unit-test".into(),
            version: "1.0.0".into(),
        },
        raw,
        content_type: "application/json".into(),
        declared_hash: Some(hash),
        meta: None,
    }
}

fn default_cfg() -> IngestConfig {
    IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: Some(|| Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 1).unwrap()),
        allowed_source_kinds: None,
    }
}

fn minimal_valid_evidence() -> IngestEvidence {
    IngestEvidence {
        bundle_id: "bnd-001".into(),
        correlation_id: "corr-001".into(),
        decision: IngestDecision::Rejected,
        received_at: ts(),
        processed_at: ts(),
        source: IngestSource {
            kind: "test-document".into(),
            subject_id: "unit-test".into(),
            version: "1.0.0".into(),
        },
        content_type: "application/json".into(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            declared_hash: "sha256:abc".into(),
            actual_hash: "sha256:abc".into(),
            verified: true,
        },
        scan: ScanEvidence {
            adapter: "not_run".into(),
            verdict: "not_run".into(),
            reason: None,
        },
        validation: ValidationEvidence {
            content_type: "application/json".into(),
            valid: false,
            reason: None,
        },
        document_ref: None,
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
fn aceita_bundle_valido() {
    let bundle = sample_bundle();
    let outcome = process_bundle(&bundle, "corr-ok", &default_cfg())
        .expect_accepted("bundle válido deve ser aceite");

    assert_eq!(outcome.evidence.decision, IngestDecision::Accepted);
    assert!(outcome.evidence.document_ref.is_some());
    assert_eq!(outcome.audit_event.event_type, "ingest.accepted");
    assert!(outcome.evidence.audit.emitted, "emitted deve ser true no caminho canónico");
    assert!(outcome.evidence.audit.event_id.is_some());
    assert!(outcome.evidence.hash.verified);
}

#[test]
fn aceita_bundle_sem_declared_hash_nao_verifica_mas_regista() {
    let raw = sample_raw();
    let bundle = IngestBundle {
        bundle_id: "bnd:no-hash:001".into(),
        received_at: ts(),
        source: IngestSource {
            kind: "test-document".into(),
            subject_id: "unit-test".into(),
            version: "1.0.0".into(),
        },
        raw,
        content_type: "application/json".into(),
        declared_hash: None,
        meta: None,
    };

    let outcome = process_bundle(&bundle, "corr-no-hash", &default_cfg())
        .expect_accepted("bundle sem declared_hash deve ser aceite");

    assert!(!outcome.evidence.hash.verified, "sem declared_hash não há verificação");
    assert!(!outcome.evidence.hash.actual_hash.is_empty(), "hash deve ser calculado mesmo assim");
}

#[test]
fn rejeita_hash_mismatch() {
    let mut bundle = sample_bundle();
    bundle.declared_hash =
        Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());

    let (outcome, err) =
        process_bundle(&bundle, "corr-hash", &default_cfg()).expect_rejected("hash errado");

    assert_eq!(outcome.evidence.decision, IngestDecision::Rejected);
    assert_eq!(outcome.audit_event.event_type, "ingest.rejected");
    assert_eq!(err.code(), HASH_MISMATCH);
}

#[test]
fn rejeita_scan_rejeitado() {
    let bundle = sample_bundle();
    let raw = bundle.raw.clone();
    let actual_hash = format!("sha256:{}", sha256_bytes(&raw));

    let mut rejected = HashMap::new();
    rejected.insert(actual_hash, "malware-simulated".into());

    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner {
            adapter_name: "deterministic".into(),
            rejected_hashes: rejected,
        })),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) = process_bundle(&bundle, "corr-scan", &cfg).expect_rejected("scan rejeita");

    assert_eq!(err.code(), SCAN_REJECTED);
    assert_eq!(outcome.evidence.scan.adapter, "deterministic");
    assert_eq!(outcome.evidence.scan.verdict, "rejected");
    assert_eq!(outcome.evidence.scan.reason.as_deref(), Some("malware-simulated"));
}

#[test]
fn rejeita_falha_tecnica_do_scanner() {
    struct ErrorScanner;
    impl ScanAdapter for ErrorScanner {
        fn scan(&self, _: &ScanInput) -> Result<ScanResult, IngestError> {
            Err(IngestError::ScanFailed)
        }
        fn adapter_id(&self) -> &str {
            "clamav"
        }
    }

    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(ErrorScanner)),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) =
        process_bundle(&bundle, "corr-scan-err", &cfg).expect_rejected("scan técnico falhou");

    assert_eq!(err.code(), SCAN_FAILED);
    assert!(err.is_retryable());
    assert_eq!(outcome.evidence.scan.adapter, "clamav");
    assert_eq!(outcome.evidence.scan.verdict, "error");
}

#[test]
fn rejeita_scanner_nao_configurado() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: None,
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, err) =
        process_bundle(&bundle, "corr-no-scanner", &cfg).expect_rejected("scanner ausente");

    assert_eq!(err.code(), SCAN_FAILED);
    assert!(err.is_retryable());
    assert_eq!(outcome.evidence.scan.adapter, "not_configured");
}

#[test]
fn rejeita_content_validation_failed() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(RejectingContentValidator {
            rejected_content_types: vec!["application/json".into()],
            reason: "XXE detected".into(),
        })),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) = process_bundle(&bundle, "corr-content-val", &cfg)
        .expect_rejected("content validation falhou");

    assert_eq!(err.code(), CONTENT_VALIDATION_FAILED);
    assert!(!err.is_retryable());
}

#[test]
fn aceita_sem_content_validator_configurado() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: None,
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let outcome =
        process_bundle(&bundle, "corr-no-validator", &cfg).expect_accepted("sem validator aceita");

    assert_eq!(outcome.evidence.decision, IngestDecision::Accepted);
}

#[test]
fn rejeita_storage_nao_configurado() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: None,
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) =
        process_bundle(&bundle, "corr-no-storage", &cfg).expect_rejected("storage ausente");

    assert_eq!(err.code(), STORE_FAILED);
    assert!(err.is_retryable());
}

#[test]
fn rejeita_storage_falha() {
    struct FailingStorage;
    impl IngestStoragePort for FailingStorage {
        fn store(&self, _: &IngestBundle, _: &str) -> Result<String, IngestError> {
            Err(IngestError::StoreFailed("minio unavailable".into()))
        }
    }

    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(FailingStorage)),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) =
        process_bundle(&bundle, "corr-storage-fail", &cfg).expect_rejected("storage falhou");

    assert_eq!(err.code(), STORE_FAILED);
    assert!(err.is_retryable());
}

#[test]
fn bundle_repetido_preserva_document_ref() {
    use std::sync::Arc;

    struct SharedStorage(Arc<MemoryStoragePort>);
    impl IngestStoragePort for SharedStorage {
        fn store(&self, bundle: &IngestBundle, hash: &str) -> Result<String, IngestError> {
            self.0.store(bundle, hash)
        }
    }

    let shared = Arc::new(MemoryStoragePort::default());
    let make_cfg = || IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(SharedStorage(Arc::clone(&shared)))),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let bundle = sample_bundle();
    let first =
        process_bundle(&bundle, "corr-r1", &make_cfg()).expect_accepted("primeira ingestão");
    let second =
        process_bundle(&bundle, "corr-r2", &make_cfg()).expect_accepted("segunda ingestão");

    assert_eq!(
        first.evidence.document_ref, second.evidence.document_ref,
        "document_ref deve ser idempotente para o mesmo bundle_id"
    );
}

#[test]
fn rejeita_bundle_oversized() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: Some(1),
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (_, err) = process_bundle(&bundle, "corr-oversized", &cfg).expect_rejected("bundle grande");

    assert!(matches!(err, IngestError::Oversized { .. }));
    assert_eq!(err.code(), OVERSIZED);
}

#[test]
fn rejeita_correlation_id_vazio() {
    let bundle = sample_bundle();
    let (_, err) =
        process_bundle(&bundle, "", &default_cfg()).expect_rejected("correlation_id vazio");

    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn rejeita_source_kind_nao_permitido() {
    let bundle = sample_bundle();
    let cfg = IngestConfig {
        scanner: Some(Box::new(DeterministicScanner::default())),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: Some(vec!["cius-pt-invoice".into()]),
    };

    let (_, err) = process_bundle(&bundle, "corr-kind", &cfg).expect_rejected("kind não permitido");

    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn hash_mismatch_mantem_scan_not_run() {
    let mut bundle = sample_bundle();
    bundle.declared_hash =
        Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());

    struct PanicScanner;
    impl ScanAdapter for PanicScanner {
        fn scan(&self, _: &ScanInput) -> Result<ScanResult, IngestError> {
            panic!("scanner não deve ser chamado após hash mismatch")
        }
        fn adapter_id(&self) -> &str {
            "panic"
        }
    }

    let cfg = IngestConfig {
        scanner: Some(Box::new(PanicScanner)),
        content_validator: Some(Box::new(PassthroughContentValidator)),
        storage: Some(Box::new(MemoryStoragePort::default())),
        max_bundle_bytes: None,
        actor: "apid".into(),
        now: None,
        allowed_source_kinds: None,
    };

    let (outcome, _) =
        process_bundle(&bundle, "corr-hash-scan", &cfg).expect_rejected("hash errado");

    assert_eq!(outcome.evidence.scan.adapter, "not_run");
    assert_eq!(outcome.evidence.scan.verdict, "not_run");
}

#[test]
fn rejected_tem_emitted_true_quando_evidence_valida() {
    let mut bundle = sample_bundle();
    bundle.declared_hash =
        Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());

    let (outcome, _) =
        process_bundle(&bundle, "corr-emitted", &default_cfg()).expect_rejected("hash errado");

    assert!(
        outcome.evidence.audit.emitted,
        "evidence válida → emitted=true mesmo em rejected"
    );
}

// ── Testes de validate_ingest_bundle ──────────────────────────────────────────

#[test]
fn valida_bundle_rejeita_bundle_id_vazio() {
    let mut b = sample_bundle();
    b.bundle_id = String::new();
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_rejeita_source_kind_vazio() {
    let mut b = sample_bundle();
    b.source.kind = String::new();
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_rejeita_subject_id_vazio() {
    let mut b = sample_bundle();
    b.source.subject_id = String::new();
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_rejeita_version_vazia() {
    let mut b = sample_bundle();
    b.source.version = String::new();
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_rejeita_content_type_vazio() {
    let mut b = sample_bundle();
    b.content_type = String::new();
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_rejeita_raw_vazio() {
    let mut b = sample_bundle();
    b.raw = vec![];
    assert_eq!(validate_ingest_bundle(&b).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_bundle_aceita_bundle_valido() {
    assert!(validate_ingest_bundle(&sample_bundle()).is_ok());
}

// ── Testes de validate_ingest_evidence ────────────────────────────────────────

#[test]
fn valida_evidence_rejeita_bundle_id_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.bundle_id = String::new();
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_evidence_rejeita_correlation_id_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.correlation_id = String::new();
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), MISSING_FIELD);
}

#[test]
fn valida_evidence_rejeita_accepted_sem_document_ref() {
    let mut ev = minimal_valid_evidence();
    ev.decision = IngestDecision::Accepted;
    ev.audit.action = "ingest.accepted".into();
    // document_ref = None por defeito
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), INVALID_REQUEST);
}

#[test]
fn valida_evidence_aceita_evidence_valida() {
    assert!(validate_ingest_evidence(&minimal_valid_evidence()).is_ok());
}

#[test]
fn valida_evidence_rejeita_timestamps_invertidos() {
    let mut ev = minimal_valid_evidence();
    ev.processed_at = ev.received_at - chrono::Duration::seconds(1);
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), INVALID_REQUEST);
}

#[test]
fn valida_evidence_rejeita_verified_com_declared_hash_vazio() {
    let mut ev = minimal_valid_evidence();
    ev.hash.declared_hash = String::new();
    ev.hash.verified = true;
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), INVALID_REQUEST);
}

#[test]
fn valida_evidence_rejeita_verified_com_hash_diferente() {
    let mut ev = minimal_valid_evidence();
    ev.hash.declared_hash =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    ev.hash.actual_hash =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    ev.hash.verified = true;
    assert_eq!(validate_ingest_evidence(&ev).unwrap_err().code(), INVALID_REQUEST);
}

// ── Testes de build_ingest_audit_event ────────────────────────────────────────

#[test]
fn build_audit_event_requer_actor() {
    let bundle = sample_bundle();
    let outcome =
        process_bundle(&bundle, "corr-audit", &default_cfg()).expect_accepted("deve aceitar");

    let err = build_ingest_audit_event(&outcome.evidence, "").unwrap_err();
    assert_eq!(err.code(), MISSING_FIELD);
}

#[test]
fn build_audit_event_accepted_tem_campos_correctos() {
    let bundle = sample_bundle();
    let outcome =
        process_bundle(&bundle, "corr-audit-ok", &default_cfg()).expect_accepted("deve aceitar");

    let event = build_ingest_audit_event(&outcome.evidence, "apid").unwrap();
    assert_eq!(event.event_type, "ingest.accepted");
    let details = event.details_json.unwrap();
    assert_eq!(details["decision"], "accepted");
    assert_eq!(details["correlation_id"], "corr-audit-ok");
    assert_eq!(details["content_type"], "application/json");
    assert!(details["document_ref"].is_string());
}

#[test]
fn ingest_decision_serializa_lowercase() {
    let accepted = serde_json::to_string(&IngestDecision::Accepted).unwrap();
    let rejected = serde_json::to_string(&IngestDecision::Rejected).unwrap();
    assert_eq!(accepted, "\"accepted\"");
    assert_eq!(rejected, "\"rejected\"");
}

#[test]
fn ingest_decision_desserializa_lowercase() {
    let a: IngestDecision = serde_json::from_str("\"accepted\"").unwrap();
    let r: IngestDecision = serde_json::from_str("\"rejected\"").unwrap();
    assert_eq!(a, IngestDecision::Accepted);
    assert_eq!(r, IngestDecision::Rejected);
}

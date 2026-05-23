use adapter_scanner::{ScanFormat, ScannedDocument};
use chrono::{TimeZone, Utc};
use core_ingest::{IngestConfig, IngestError, MemoryRouter, DECISION_ACCEPTED, INVALID_REQUEST};
use ingest_scanner::ScanIngestError;
use ingest_scanner::{
    ingest_scanned_document, scanned_document_ingest_config, AlwaysCleanScanner,
    ScanIngestRequest, SCANNED_DOCUMENT_KIND,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn sample_pdf() -> ScannedDocument {
    ScannedDocument {
        format: ScanFormat::Pdf,
        data: b"%PDF-1.4 fake scan content for testing purposes".to_vec(),
        content_type: "application/pdf".into(),
    }
}

fn sample_req() -> ScanIngestRequest {
    ScanIngestRequest {
        request_id: "scan:req:test:001".into(),
        subject_id: "processo:2026-001".into(),
        version: "1".into(),
        document: sample_pdf(),
    }
}

fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap()
}

fn default_cfg() -> IngestConfig {
    IngestConfig {
        scanner: Some(Box::new(AlwaysCleanScanner::default())),
        router: Some(Box::new(MemoryRouter::new("scan/processed"))),
        max_bundle_bytes: None,
        actor: "scanner:apid".into(),
        now: Some(fixed_now),
        allowed_source_kinds: Some(vec![SCANNED_DOCUMENT_KIND.into()]),
    }
}

// ── Happy path ────────────────────────────────────────────────────────────────

#[test]
fn ingest_documento_digitalizado_aceito() {
    let outcome = ingest_scanned_document(&sample_req(), "corr-scan-001", default_cfg())
        .expect("bridge deve construir snapshot")
        .expect_accepted("documento válido deve ser aceite");

    assert_eq!(outcome.evidence.decision, DECISION_ACCEPTED);
    assert!(outcome.evidence.route.routed);
    assert_eq!(outcome.evidence.source.kind, SCANNED_DOCUMENT_KIND);
    assert_eq!(outcome.audit_event.event_type, "ingest.accepted");
    assert!(outcome.evidence.audit.emitted);
    assert!(outcome.evidence.audit.event_id.is_some());
}

#[test]
fn evidence_contem_hash_do_documento() {
    let outcome = ingest_scanned_document(&sample_req(), "corr-scan-hash", default_cfg())
        .expect("bridge")
        .expect_accepted("deve aceitar");

    assert!(outcome.evidence.hash.verified);
    assert!(outcome.evidence.hash.actual_hash.starts_with("sha256:"));
    assert_eq!(outcome.evidence.hash.expected_hash, outcome.evidence.hash.actual_hash);
}

#[test]
fn evidence_tem_source_kind_scanned_document() {
    let outcome = ingest_scanned_document(&sample_req(), "corr-scan-kind", default_cfg())
        .expect("bridge")
        .expect_accepted("deve aceitar");

    assert_eq!(outcome.evidence.source.kind, "scanned_document");
    assert_eq!(outcome.evidence.source.subject_id, "processo:2026-001");
    assert_eq!(outcome.evidence.source.version, "1");
}

#[test]
fn always_clean_scanner_verdict_e_clean() {
    let outcome = ingest_scanned_document(&sample_req(), "corr-scan-clean", default_cfg())
        .expect("bridge")
        .expect_accepted("deve aceitar");

    assert_eq!(outcome.evidence.scan.adapter, "always-clean");
    assert_eq!(outcome.evidence.scan.verdict, "clean");
    assert!(outcome.evidence.scan.reason.is_none());
}

// ── Rejection paths ───────────────────────────────────────────────────────────

#[test]
fn ingest_rejeita_correlation_id_vazio() {
    // build_export_receipt validates correlation_id before the pipeline runs,
    // so the bridge returns Err(SnapshotBuild) rather than Ok(Rejected).
    let result = ingest_scanned_document(&sample_req(), "", default_cfg());
    match result {
        Err(ScanIngestError::SnapshotBuild(_)) => {}
        Ok(outcome) => assert!(outcome.error().is_some(), "pipeline deve rejeitar"),
        Err(e) => panic!("unexpected bridge error: {e}"),
    }
}

#[test]
fn ingest_rejeita_request_id_vazio() {
    let mut req = sample_req();
    req.request_id = String::new();
    let (_, err) = ingest_scanned_document(&req, "corr-no-reqid", default_cfg())
        .expect("bridge")
        .expect_rejected("request_id vazio");

    assert!(matches!(err, IngestError::MissingField { .. }));
}

#[test]
fn ingest_rejeita_subject_id_vazio() {
    let mut req = sample_req();
    req.subject_id = String::new();
    let result = ingest_scanned_document(&req, "corr-no-subject", default_cfg());
    // Either bridge fails or pipeline rejects
    match result {
        Err(_) => {} // bridge-level failure (snapshot build)
        Ok(outcome) => {
            assert!(outcome.error().is_some(), "pipeline deve rejeitar subject_id vazio");
        }
    }
}

#[test]
fn ingest_rejeita_source_kind_nao_permitido() {
    let cfg = IngestConfig {
        scanner: Some(Box::new(AlwaysCleanScanner::default())),
        router: Some(Box::new(MemoryRouter::new("scan/processed"))),
        max_bundle_bytes: None,
        actor: "scanner:apid".into(),
        now: Some(fixed_now),
        allowed_source_kinds: Some(vec!["config_export_bundle".into()]),
    };
    let (_, err) = ingest_scanned_document(&sample_req(), "corr-kind-rejected", cfg)
        .expect("bridge")
        .expect_rejected("scanned_document não permitido nesta config");
    assert_eq!(err.code(), INVALID_REQUEST);
}

#[test]
fn ingest_rejeita_bundle_oversized() {
    let cfg = IngestConfig {
        scanner: Some(Box::new(AlwaysCleanScanner::default())),
        router: Some(Box::new(MemoryRouter::new("scan/processed"))),
        max_bundle_bytes: Some(1),
        actor: "scanner:apid".into(),
        now: Some(fixed_now),
        allowed_source_kinds: Some(vec![SCANNED_DOCUMENT_KIND.into()]),
    };
    let (_, err) = ingest_scanned_document(&sample_req(), "corr-oversized", cfg)
        .expect("bridge")
        .expect_rejected("bundle demasiado grande");
    assert!(matches!(err, IngestError::Oversized { .. }));
}

// ── Config helper ─────────────────────────────────────────────────────────────

#[test]
fn config_helper_define_allowed_kind_correcto() {
    let cfg = scanned_document_ingest_config(
        Box::new(MemoryRouter::new("scan/processed")),
        "scanner:apid".into(),
    );
    let allowed = cfg.allowed_source_kinds.as_ref().unwrap();
    assert_eq!(allowed, &vec!["scanned_document".to_string()]);
}

#[test]
fn config_helper_inclui_always_clean_scanner() {
    let cfg = scanned_document_ingest_config(
        Box::new(MemoryRouter::new("scan/processed")),
        "scanner:apid".into(),
    );
    assert!(cfg.scanner.is_some());
    let input = core_ingest::ScanInput {
        request_id: "r".into(),
        correlation_id: "c".into(),
        bundle_hash: "sha256:abc".into(),
        bundle: {
            use core_exports::{build_export_receipt, BuildSnapshotConfig, SourceRef};
            use core_documental::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};
            let ts = fixed_now();
            let pkg = DocumentPackage {
                document_id: "doc:test:sub:1".into(),
                created_at: ts,
                template: TemplateRef { template_id: "t".into(), template_version: "v1".into(), valid_at: None },
                engine: EngineRef { engine_id: "e".into(), engine_version: "v1".into() },
                artefacts: vec![Artefact {
                    kind: "k".into(), artefact_ref: "r".into(),
                    hash_result: HashResult { algorithm: "SHA-256".into(), hash: "sha256:abc".into(), timestamp: ts, input_kind: None, input_ref: None, meta: None },
                    mime: None, size_bytes: None,
                }],
                subject: None, meta: None,
            };
            build_export_receipt(pkg, SourceRef { kind: "scanned_document".into(), subject_id: "sub".into(), version: "1".into() },
                BuildSnapshotConfig { exported_at: Some(ts), actor: "a".into(), correlation_id: "c".into(), transport: None }).unwrap().snapshot
        },
        payload: vec![],
    };
    let result = cfg.scanner.as_ref().unwrap().scan(&input).unwrap();
    assert_eq!(result.verdict, "clean");
}

// ── Dependency guard ──────────────────────────────────────────────────────────

#[test]
fn ingest_scanner_nao_depende_de_sqlite() {
    let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(!m.contains("rusqlite") && !m.contains("adapter-sqlite"));
}

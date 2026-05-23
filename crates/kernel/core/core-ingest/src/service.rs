use chrono::{DateTime, Utc};
use serde_json::json;

use core_audit::{AuditActor, AuditEvent, AuditTarget};
use core_exports::{canonical_bytes, validate_export_snapshot};
use core_validation::sha256_bytes;

use crate::error::IngestError;
use crate::types::{
    AuditEvidence, HashEvidence, IngestConfig, IngestEvidence, IngestOutcome, IngestRequest,
    Outcome, RouteEvidence, RouteInput, ScanEvidence, ScanInput, ValidationEvidence,
    DECISION_ACCEPTED, DECISION_REJECTED,
};

const HASH_PREFIX: &str = "sha256:";
const DEFAULT_ACTOR: &str = "core-ingest";
const EXPORT_CONTRACT: &str = "core-exports/export_snapshot";

// ── API pública ────────────────────────────────────────────────────────────────

/// Processa um `IngestRequest` pelo pipeline validate→hash→scan→route→audit.
///
/// Retorna `IngestOutcome::Accepted` ou `IngestOutcome::Rejected`.
/// Em ambos os casos o `Outcome` interior contém a evidence completa e o audit event.
pub fn process_export_snapshot(
    req: &IngestRequest,
    correlation_id: &str,
    cfg: &IngestConfig,
) -> IngestOutcome {
    let now = cfg.now.map(|f| f()).unwrap_or_else(Utc::now);
    let actor = effective_actor(&cfg.actor);

    if correlation_id.trim().is_empty() {
        return fallback_rejection(req, "", now, actor, IngestError::MissingField {
            field: "correlation_id".into(),
        });
    }

    if let Some(allowed) = &cfg.allowed_source_kinds {
        if !allowed.iter().any(|k| k == &req.source.kind) {
            return reject(
                req, correlation_id, now, RejectionSpec::default(), actor,
                IngestError::InvalidRequest {
                    message: format!(
                        "source.kind '{}' not allowed in this pipeline", req.source.kind
                    ),
                },
            );
        }
    }

    if let Err(e) = validate_ingest_request(req) {
        return reject(req, correlation_id, now, RejectionSpec::default(), actor, e);
    }

    if let Err(e) = validate_export_snapshot(&req.bundle) {
        return reject(
            req, correlation_id, now,
            RejectionSpec { expected_hash: req.expected_hash.clone(), ..Default::default() },
            actor,
            IngestError::InvalidRequest { message: format!("bundle inválido para ingest: {e}") },
        );
    }

    let payload = match canonical_bytes(&req.bundle) {
        Ok(p) => p,
        Err(e) => {
            return fallback_rejection(req, correlation_id, now, actor,
                IngestError::MarshalFailed(e.to_string()));
        }
    };

    if let Some(limit) = cfg.max_bundle_bytes {
        if payload.len() > limit {
            return reject(
                req, correlation_id, now,
                RejectionSpec { expected_hash: req.expected_hash.clone(), ..Default::default() },
                actor,
                IngestError::Oversized { limit_bytes: limit, actual_bytes: payload.len() },
            );
        }
    }

    let actual_hash = format!("{}{}", HASH_PREFIX, sha256_bytes(&payload));
    if actual_hash != req.expected_hash {
        return reject(
            req, correlation_id, now,
            RejectionSpec {
                expected_hash: req.expected_hash.clone(),
                actual_hash: actual_hash.clone(),
                hash_verified: true,
                ..Default::default()
            },
            actor,
            IngestError::HashMismatch {
                expected: req.expected_hash.clone(),
                actual: actual_hash,
            },
        );
    }

    let scanner = match cfg.scanner.as_ref() {
        Some(s) => s,
        None => {
            return reject(
                req, correlation_id, now,
                RejectionSpec {
                    expected_hash: req.expected_hash.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence { adapter: "not_configured".into(), verdict: "error".into(),
                        reason: Some("scanner unavailable".into()) },
                    hash_verified: true,
                    ..Default::default()
                },
                actor,
                IngestError::ScanFailed,
            );
        }
    };

    let scan_adapter_id = scanner.adapter_id().to_owned();
    let scan_result = match scanner.scan(&ScanInput {
        request_id: req.request_id.clone(),
        correlation_id: correlation_id.to_owned(),
        bundle_hash: actual_hash.clone(),
        bundle: req.bundle.clone(),
        payload,
    }) {
        Ok(r) => r,
        Err(_) => {
            return reject(
                req, correlation_id, now,
                RejectionSpec {
                    expected_hash: req.expected_hash.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence { adapter: scan_adapter_id.clone(), verdict: "error".into(),
                        reason: Some("scan failure".into()) },
                    hash_verified: true,
                    ..Default::default()
                },
                actor,
                IngestError::ScanFailed,
            );
        }
    };

    let scan_adapter = non_empty_str(&scan_result.adapter).unwrap_or(&scan_adapter_id);
    let scan_verdict = non_empty_str(&scan_result.verdict).unwrap_or("error");

    if scan_verdict != "clean" {
        return reject(
            req, correlation_id, now,
            RejectionSpec {
                expected_hash: req.expected_hash.clone(),
                actual_hash: actual_hash.clone(),
                scan: ScanEvidence { adapter: scan_adapter.into(), verdict: scan_verdict.into(),
                    reason: scan_result.reason.clone() },
                hash_verified: true,
                ..Default::default()
            },
            actor,
            IngestError::ScanRejected { adapter: scan_adapter.into(), verdict: scan_verdict.into() },
        );
    }

    let router = match cfg.router.as_ref() {
        Some(r) => r,
        None => {
            return reject(
                req, correlation_id, now,
                RejectionSpec {
                    expected_hash: req.expected_hash.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence { adapter: scan_adapter.into(), verdict: scan_verdict.into(),
                        reason: None },
                    hash_verified: true,
                    ..Default::default()
                },
                actor,
                IngestError::RouteUnavailable,
            );
        }
    };

    let mut evidence = base_evidence(req, correlation_id, now, &req.expected_hash, &actual_hash, true);
    evidence.decision = DECISION_ACCEPTED.into();
    evidence.scan = ScanEvidence {
        adapter: scan_adapter.into(),
        verdict: scan_verdict.into(),
        reason: scan_result.reason,
    };
    evidence.validation = ValidationEvidence { contract: EXPORT_CONTRACT.into(), valid: true };

    let routed = match router.route(&RouteInput {
        request_id: req.request_id.clone(),
        correlation_id: correlation_id.to_owned(),
        bundle_hash: actual_hash.clone(),
        bundle: req.bundle.clone(),
        evidence: evidence.clone(),
    }) {
        Ok(r) => r,
        Err(_) => {
            evidence.decision = DECISION_REJECTED.into();
            evidence.audit.action = "ingest.rejected".into();
            return finalize(evidence, actor, Some(IngestError::RouteUnavailable));
        }
    };

    evidence.route = crate::types::RouteEvidence {
        routed: true,
        target: Some(routed.target),
        route_ref: Some(routed.route_ref),
    };
    evidence.audit = AuditEvidence {
        required: true,
        emitted: false,
        action: "ingest.accepted".into(),
        event_id: None,
    };

    finalize(evidence, actor, None)
}

/// Valida os campos obrigatórios de um `IngestRequest`.
pub fn validate_ingest_request(req: &IngestRequest) -> Result<(), IngestError> {
    if req.request_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "request_id".into() });
    }
    // received_at: DateTime<Utc> não tem zero-value semântico em Rust (não existe "ano 1"
    // como no Go). Confiamos na serialização/construção do caller para garantir o campo.
    if req.source.subject_id.trim().is_empty() || req.source.version.trim().is_empty() {
        return Err(IngestError::MissingField {
            field: "source.subject_id / source.version".into(),
        });
    }
    if req.expected_hash.trim().is_empty() {
        return Err(IngestError::MissingField { field: "expected_hash".into() });
    }
    if req.bundle.source.subject_id != req.source.subject_id
        || req.bundle.source.version != req.source.version
    {
        return Err(IngestError::InvalidRequest {
            message: "source do pedido não corresponde ao bundle".into(),
        });
    }
    Ok(())
}

/// Valida a coerência interna de um `IngestEvidence`.
pub fn validate_ingest_evidence(evidence: &IngestEvidence) -> Result<(), IngestError> {
    if evidence.request_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "request_id".into() });
    }
    if evidence.correlation_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "correlation_id".into() });
    }
    if evidence.decision != DECISION_ACCEPTED && evidence.decision != DECISION_REJECTED {
        return Err(IngestError::InvalidRequest {
            message: format!("decision inválida: {}", evidence.decision),
        });
    }
    if evidence.bundle_ref.trim().is_empty() {
        return Err(IngestError::MissingField { field: "bundle_ref".into() });
    }
    if evidence.audit.action.trim().is_empty() || !evidence.audit.required {
        return Err(IngestError::InvalidRequest {
            message: "audit evidence obrigatória".into(),
        });
    }
    if evidence.decision == DECISION_ACCEPTED && !evidence.route.routed {
        return Err(IngestError::InvalidRequest {
            message: "route é obrigatória para accepted".into(),
        });
    }
    Ok(())
}

/// Constrói um `AuditEvent` a partir de um `IngestEvidence` validado.
pub fn build_ingest_audit_event(
    evidence: &IngestEvidence,
    actor: &str,
) -> Result<AuditEvent, IngestError> {
    if actor.trim().is_empty() {
        return Err(IngestError::MissingField { field: "actor".into() });
    }
    validate_ingest_evidence(evidence)?;

    let subject_id = if evidence.source.subject_id.trim().is_empty() {
        "unknown"
    } else {
        &evidence.source.subject_id
    };

    let audit_actor = AuditActor::new(actor)
        .map_err(|e| IngestError::AuditError(e.to_string()))?;
    let audit_target = AuditTarget::new("ingest", subject_id)
        .map_err(|e| IngestError::AuditError(e.to_string()))?;

    let details = json!({
        "correlation_id": evidence.correlation_id,
        "request_id":     evidence.request_id,
        "decision":       evidence.decision,
        "bundle_ref":     evidence.bundle_ref,
        "bundle_hash":    evidence.hash.actual_hash,
        "scan_verdict":   evidence.scan.verdict,
        "route_target":   evidence.route.target,
    });

    AuditEvent::new(evidence.audit.action.as_str(), audit_actor, audit_target, Some(details))
        .map_err(|e| IngestError::AuditError(e.to_string()))
}

// ── Internos ───────────────────────────────────────────────────────────────────

/// Especificação do ponto de falha para construção de rejected outcomes.
/// Usa `Default` para "nada correu ainda" (hashes vazios, scan not_run).
struct RejectionSpec {
    expected_hash: String,
    actual_hash: String,
    scan: ScanEvidence,
    hash_verified: bool,
    route: RouteEvidence,
}

impl Default for RejectionSpec {
    fn default() -> Self {
        Self {
            expected_hash: String::new(),
            actual_hash: String::new(),
            scan: not_run_scan(),
            hash_verified: false,
            route: RouteEvidence::default(),
        }
    }
}

fn reject(
    req: &IngestRequest,
    correlation_id: &str,
    processed_at: DateTime<Utc>,
    spec: RejectionSpec,
    actor: &str,
    error: IngestError,
) -> IngestOutcome {
    let mut evidence = base_evidence(
        req, correlation_id, processed_at,
        &spec.expected_hash, &spec.actual_hash, spec.hash_verified,
    );
    evidence.scan = spec.scan;
    evidence.validation.valid = spec.hash_verified;
    evidence.route = spec.route;
    finalize(evidence, actor, Some(error))
}

/// Usado quando não é possível construir evidence completa (ex.: correlation_id vazio).
fn fallback_rejection(
    req: &IngestRequest,
    correlation_id: &str,
    now: DateTime<Utc>,
    actor: &str,
    error: IngestError,
) -> IngestOutcome {
    let evidence = IngestEvidence {
        request_id: req.request_id.clone(),
        correlation_id: correlation_id.to_owned(),
        decision: DECISION_REJECTED.into(),
        received_at: req.received_at,
        processed_at: now,
        source: req.source.clone(),
        bundle_ref: req.bundle.snapshot_id.clone(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            expected_hash: String::new(),
            actual_hash: String::new(),
            verified: false,
        },
        scan: not_run_scan(),
        validation: ValidationEvidence { contract: EXPORT_CONTRACT.into(), valid: false },
        route: RouteEvidence::default(),
        audit: AuditEvidence {
            required: true,
            emitted: false,
            action: "ingest.rejected".into(),
            event_id: None,
        },
        meta: Some(json!({ "slice": "config-bundle-first" })),
    };
    finalize(evidence, actor, Some(error))
}

/// Finaliza o outcome: tenta construir o audit event pelo caminho canónico.
///
/// `emitted = true` apenas quando o evento é construído via `build_ingest_audit_event`.
/// Se a evidence for inválida (ex.: bundle_ref vazio num fallback), usa o evento de
/// emergência e mantém `emitted = false`.
fn finalize(
    mut evidence: IngestEvidence,
    actor: &str,
    error: Option<IngestError>,
) -> IngestOutcome {
    match build_ingest_audit_event(&evidence, actor) {
        Ok(event) => {
            evidence.audit.emitted = true;
            evidence.audit.event_id = Some(event.event_id.clone());
            let outcome = Outcome { evidence, audit_event: event };
            match error {
                None => IngestOutcome::Accepted(outcome),
                Some(e) => IngestOutcome::Rejected { outcome, error: e },
            }
        }
        Err(_) => {
            // evidence inválida (ex.: bundle_ref vazio) — emitted fica false
            let event = emergency_audit_event(&evidence, actor);
            evidence.audit.event_id = Some(event.event_id.clone());
            let outcome = Outcome { evidence, audit_event: event };
            let err = error.unwrap_or_else(|| {
                IngestError::AuditError("audit event não pôde ser gerado".into())
            });
            IngestOutcome::Rejected { outcome, error: err }
        }
    }
}

/// Audit event de emergência quando a evidence é inválida mas é necessário um evento.
/// `emitted` não é marcado a `true` neste caso.
fn emergency_audit_event(evidence: &IngestEvidence, actor: &str) -> AuditEvent {
    let actor_id = if actor.trim().is_empty() { DEFAULT_ACTOR } else { actor };
    let subject = non_empty_str(&evidence.source.subject_id).unwrap_or("unknown");
    let action = non_empty_str(&evidence.audit.action).unwrap_or("ingest.rejected");

    AuditEvent::new(
        action,
        AuditActor::new(actor_id).expect("emergency actor is valid"),
        AuditTarget::new("ingest", subject).expect("emergency target is valid"),
        None,
    )
    .expect("emergency audit event is valid")
}

fn base_evidence(
    req: &IngestRequest,
    correlation_id: &str,
    processed_at: DateTime<Utc>,
    expected_hash: &str,
    actual_hash: &str,
    verified: bool,
) -> IngestEvidence {
    IngestEvidence {
        request_id: req.request_id.clone(),
        correlation_id: correlation_id.to_owned(),
        decision: DECISION_REJECTED.into(),
        received_at: req.received_at,
        processed_at,
        source: req.source.clone(),
        bundle_ref: req.bundle.snapshot_id.clone(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            expected_hash: expected_hash.to_owned(),
            actual_hash: actual_hash.to_owned(),
            verified,
        },
        scan: not_run_scan(),
        validation: ValidationEvidence { contract: EXPORT_CONTRACT.into(), valid: false },
        route: RouteEvidence::default(),
        audit: AuditEvidence {
            required: true,
            emitted: false,
            action: "ingest.rejected".into(),
            event_id: None,
        },
        meta: Some(json!({ "slice": "config-bundle-first" })),
    }
}

fn not_run_scan() -> ScanEvidence {
    ScanEvidence { adapter: "not_run".into(), verdict: "not_run".into(), reason: None }
}

fn effective_actor(actor: &str) -> &str {
    if actor.trim().is_empty() { DEFAULT_ACTOR } else { actor }
}

fn non_empty_str(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

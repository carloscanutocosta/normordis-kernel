use chrono::{DateTime, Utc};
use serde_json::json;

use core_audit::{AuditActor, AuditEvent, AuditOutcome, AuditTarget};
use core_validation::sha256_bytes;

use crate::error::IngestError;
use crate::types::{
    AuditEvidence, HashEvidence, IngestBundle, IngestConfig, IngestDecision, IngestEvidence,
    IngestOutcome, Outcome, ScanEvidence, ScanInput, ValidationEvidence,
};

const HASH_PREFIX: &str = "sha256:";
const DEFAULT_ACTOR: &str = "core-ingest";

// ── API pública ────────────────────────────────────────────────────────────────

/// Processa um `IngestBundle` pelo pipeline hash→size→scan→validate→store→audit.
///
/// O hash é calculado sobre `bundle.raw` antes de qualquer parsing.
/// Retorna `IngestOutcome::Accepted` ou `IngestOutcome::Rejected`.
/// Em ambos os casos o `Outcome` interior contém a evidence completa e o audit event.
pub fn process_bundle(
    bundle: &IngestBundle,
    correlation_id: &str,
    cfg: &IngestConfig,
) -> IngestOutcome {
    let now = cfg.now.map(|f| f()).unwrap_or_else(Utc::now);
    let actor = effective_actor(&cfg.actor);

    // ── 1. correlation_id ──────────────────────────────────────────────────────
    if correlation_id.trim().is_empty() {
        return fallback_rejection(
            bundle,
            "",
            now,
            actor,
            IngestError::MissingField { field: "correlation_id".into() },
        );
    }

    // ── 2. source.kind allowlist ───────────────────────────────────────────────
    if let Some(allowed) = &cfg.allowed_source_kinds {
        if !allowed.iter().any(|k| k == &bundle.source.kind) {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec::default(),
                actor,
                IngestError::InvalidRequest {
                    message: format!("source.kind '{}' não permitido", bundle.source.kind),
                },
            );
        }
    }

    // ── 3. campos obrigatórios do bundle ───────────────────────────────────────
    if let Err(e) = validate_ingest_bundle(bundle) {
        return reject(bundle, correlation_id, now, RejectionSpec::default(), actor, e);
    }

    // ── 4. size check ──────────────────────────────────────────────────────────
    if let Some(limit) = cfg.max_bundle_bytes {
        if bundle.raw.len() > limit {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec::default(),
                actor,
                IngestError::Oversized { limit_bytes: limit, actual_bytes: bundle.raw.len() },
            );
        }
    }

    // ── 5. hash sobre bytes raw ────────────────────────────────────────────────
    let actual_hash = format!("{}{}", HASH_PREFIX, sha256_bytes(&bundle.raw));
    let (declared_hash_str, hash_verified) = match &bundle.declared_hash {
        Some(declared) => {
            if &actual_hash != declared {
                return reject(
                    bundle,
                    correlation_id,
                    now,
                    RejectionSpec {
                        declared_hash: declared.clone(),
                        actual_hash: actual_hash.clone(),
                        // verified=false: os hashes foram comparados mas não coincidem (INGEST-R15)
                        hash_verified: false,
                        ..Default::default()
                    },
                    actor,
                    IngestError::HashMismatch {
                        expected: declared.clone(),
                        actual: actual_hash,
                    },
                );
            }
            (declared.clone(), true)
        }
        None => (String::new(), false),
    };

    // ── 6. scan antimalware ────────────────────────────────────────────────────
    let scanner = match cfg.scanner.as_ref() {
        Some(s) => s,
        None => {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec {
                    declared_hash: declared_hash_str.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence {
                        adapter: "not_configured".into(),
                        verdict: "error".into(),
                        reason: Some("scanner unavailable".into()),
                    },
                    hash_verified,
                },
                actor,
                IngestError::ScanFailed,
            );
        }
    };

    let scan_adapter_id = scanner.adapter_id().to_owned();
    let scan_result = match scanner.scan(&ScanInput {
        bundle_id: bundle.bundle_id.clone(),
        correlation_id: correlation_id.to_owned(),
        bundle_hash: actual_hash.clone(),
        content_type: bundle.content_type.clone(),
        raw: bundle.raw.clone(),
    }) {
        Ok(r) => r,
        Err(_) => {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec {
                    declared_hash: declared_hash_str.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence {
                        adapter: scan_adapter_id.clone(),
                        verdict: "error".into(),
                        reason: Some("scan failure".into()),
                    },
                    hash_verified,
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
            bundle,
            correlation_id,
            now,
            RejectionSpec {
                declared_hash: declared_hash_str.clone(),
                actual_hash: actual_hash.clone(),
                scan: ScanEvidence {
                    adapter: scan_adapter.into(),
                    verdict: scan_verdict.into(),
                    reason: scan_result.reason,
                },
                hash_verified,
            },
            actor,
            IngestError::ScanRejected {
                adapter: scan_adapter.into(),
                verdict: scan_verdict.into(),
            },
        );
    }

    // ── 7. validação de conteúdo (opcional) ───────────────────────────────────
    let validation = if let Some(validator) = cfg.content_validator.as_ref() {
        match validator.validate(&bundle.raw, &bundle.content_type) {
            Ok(()) => ValidationEvidence {
                content_type: bundle.content_type.clone(),
                valid: true,
                reason: None,
            },
            Err(e) => {
                return reject(
                    bundle,
                    correlation_id,
                    now,
                    RejectionSpec {
                        declared_hash: declared_hash_str.clone(),
                        actual_hash: actual_hash.clone(),
                        scan: ScanEvidence {
                            adapter: scan_adapter.into(),
                            verdict: scan_verdict.into(),
                            reason: scan_result.reason,
                        },
                        hash_verified,
                    },
                    actor,
                    e,
                );
            }
        }
    } else {
        ValidationEvidence {
            content_type: bundle.content_type.clone(),
            valid: true,
            reason: None,
        }
    };

    // ── 8. armazenamento via IngestStoragePort ─────────────────────────────────
    let storage = match cfg.storage.as_ref() {
        Some(s) => s,
        None => {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec {
                    declared_hash: declared_hash_str.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence {
                        adapter: scan_adapter.into(),
                        verdict: scan_verdict.into(),
                        reason: scan_result.reason,
                    },
                    hash_verified,
                },
                actor,
                IngestError::StoreFailed("storage not configured".into()),
            );
        }
    };

    let document_ref = match storage.store(bundle, &actual_hash) {
        Ok(r) => r,
        Err(e) => {
            return reject(
                bundle,
                correlation_id,
                now,
                RejectionSpec {
                    declared_hash: declared_hash_str.clone(),
                    actual_hash: actual_hash.clone(),
                    scan: ScanEvidence {
                        adapter: scan_adapter.into(),
                        verdict: scan_verdict.into(),
                        reason: scan_result.reason,
                    },
                    hash_verified,
                },
                actor,
                e,
            );
        }
    };

    // ── 9. evidence aceite + audit ─────────────────────────────────────────────
    let evidence = IngestEvidence {
        bundle_id: bundle.bundle_id.clone(),
        correlation_id: correlation_id.to_owned(),
        decision: IngestDecision::Accepted,
        received_at: bundle.received_at,
        processed_at: now,
        source: bundle.source.clone(),
        content_type: bundle.content_type.clone(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            declared_hash: declared_hash_str,
            actual_hash,
            verified: hash_verified,
        },
        scan: ScanEvidence {
            adapter: scan_adapter.into(),
            verdict: scan_verdict.into(),
            reason: scan_result.reason,
        },
        validation,
        document_ref: Some(document_ref),
        audit: AuditEvidence {
            required: true,
            emitted: false,
            action: "ingest.accepted".into(),
            event_id: None,
        },
        meta: None,
    };

    finalize(evidence, actor, None)
}

/// Valida os campos obrigatórios de um `IngestBundle`.
pub fn validate_ingest_bundle(bundle: &IngestBundle) -> Result<(), IngestError> {
    if bundle.bundle_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "bundle_id".into() });
    }
    if bundle.source.kind.trim().is_empty() {
        return Err(IngestError::MissingField { field: "source.kind".into() });
    }
    if bundle.source.subject_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "source.subject_id".into() });
    }
    if bundle.source.version.trim().is_empty() {
        return Err(IngestError::MissingField { field: "source.version".into() });
    }
    if bundle.content_type.trim().is_empty() {
        return Err(IngestError::MissingField { field: "content_type".into() });
    }
    if bundle.raw.is_empty() {
        return Err(IngestError::MissingField { field: "raw".into() });
    }
    Ok(())
}

/// Valida a coerência interna de um `IngestEvidence`.
pub fn validate_ingest_evidence(evidence: &IngestEvidence) -> Result<(), IngestError> {
    if evidence.bundle_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "bundle_id".into() });
    }
    if evidence.correlation_id.trim().is_empty() {
        return Err(IngestError::MissingField { field: "correlation_id".into() });
    }
    if evidence.content_type.trim().is_empty() {
        return Err(IngestError::MissingField { field: "content_type".into() });
    }
    if evidence.audit.action.trim().is_empty() || !evidence.audit.required {
        return Err(IngestError::InvalidRequest {
            message: "audit evidence obrigatória".into(),
        });
    }
    if evidence.decision == IngestDecision::Accepted && evidence.document_ref.is_none() {
        return Err(IngestError::InvalidRequest {
            message: "document_ref obrigatório para accepted".into(),
        });
    }
    // INGEST-R14: invariante temporal
    if evidence.processed_at < evidence.received_at {
        return Err(IngestError::InvalidRequest {
            message: "processed_at não pode ser anterior a received_at (INGEST-R14)".into(),
        });
    }
    // INGEST-R15: verified=true implica declared_hash == actual_hash e ambos não vazios
    if evidence.hash.verified
        && (evidence.hash.declared_hash.is_empty()
            || evidence.hash.declared_hash != evidence.hash.actual_hash)
    {
        return Err(IngestError::InvalidRequest {
            message: "hash.verified=true implica declared_hash não vazio e igual a actual_hash (INGEST-R15)"
                .into(),
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

    let audit_actor =
        AuditActor::new(actor).map_err(|e| IngestError::AuditError(e.to_string()))?;
    let audit_target = AuditTarget::new("ingest", subject_id)
        .map_err(|e| IngestError::AuditError(e.to_string()))?;

    let details = json!({
        "correlation_id": evidence.correlation_id,
        "bundle_id":      evidence.bundle_id,
        "decision":       evidence.decision.to_string(),
        "content_type":   evidence.content_type,
        "bundle_hash":    evidence.hash.actual_hash,
        "scan_verdict":   evidence.scan.verdict,
        "document_ref":   evidence.document_ref,
    });

    AuditEvent::new(
        evidence.audit.action.as_str(),
        audit_actor,
        audit_target,
        AuditOutcome::Success,
        None,
        Some(details),
    )
    .map_err(|e| IngestError::AuditError(e.to_string()))
}

// ── Internos ───────────────────────────────────────────────────────────────────

struct RejectionSpec {
    declared_hash: String,
    actual_hash: String,
    scan: ScanEvidence,
    hash_verified: bool,
}

impl Default for RejectionSpec {
    fn default() -> Self {
        Self {
            declared_hash: String::new(),
            actual_hash: String::new(),
            scan: not_run_scan(),
            hash_verified: false,
        }
    }
}

fn reject(
    bundle: &IngestBundle,
    correlation_id: &str,
    processed_at: DateTime<Utc>,
    spec: RejectionSpec,
    actor: &str,
    error: IngestError,
) -> IngestOutcome {
    let evidence = IngestEvidence {
        bundle_id: bundle.bundle_id.clone(),
        correlation_id: correlation_id.to_owned(),
        decision: IngestDecision::Rejected,
        received_at: bundle.received_at,
        processed_at,
        source: bundle.source.clone(),
        content_type: bundle.content_type.clone(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            declared_hash: spec.declared_hash,
            actual_hash: spec.actual_hash,
            verified: spec.hash_verified,
        },
        scan: spec.scan,
        validation: ValidationEvidence {
            content_type: bundle.content_type.clone(),
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
    };
    finalize(evidence, actor, Some(error))
}

/// Usado quando não é possível construir evidence completa (ex.: correlation_id vazio).
fn fallback_rejection(
    bundle: &IngestBundle,
    correlation_id: &str,
    now: DateTime<Utc>,
    actor: &str,
    error: IngestError,
) -> IngestOutcome {
    let evidence = IngestEvidence {
        bundle_id: bundle.bundle_id.clone(),
        correlation_id: correlation_id.to_owned(),
        decision: IngestDecision::Rejected,
        received_at: bundle.received_at,
        processed_at: now,
        source: bundle.source.clone(),
        content_type: bundle.content_type.clone(),
        hash: HashEvidence {
            algorithm: "SHA-256".into(),
            declared_hash: String::new(),
            actual_hash: String::new(),
            verified: false,
        },
        scan: not_run_scan(),
        validation: ValidationEvidence {
            content_type: bundle.content_type.clone(),
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
    };
    finalize(evidence, actor, Some(error))
}

/// Finaliza o outcome: tenta construir o audit event pelo caminho canónico.
///
/// `emitted = true` apenas quando o evento é construído via `build_ingest_audit_event`.
/// Se a evidence for inválida (ex.: correlation_id vazio num fallback), usa o evento de
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

fn emergency_audit_event(evidence: &IngestEvidence, actor: &str) -> AuditEvent {
    let actor_id = if actor.trim().is_empty() { DEFAULT_ACTOR } else { actor };
    let subject = non_empty_str(&evidence.source.subject_id).unwrap_or("unknown");
    let action = non_empty_str(&evidence.audit.action).unwrap_or("ingest.rejected");

    AuditEvent::new(
        action,
        AuditActor::new(actor_id).expect("emergency actor is valid"),
        AuditTarget::new("ingest", subject).expect("emergency target is valid"),
        AuditOutcome::Success,
        None,
        None,
    )
    .expect("emergency audit event is valid")
}

fn not_run_scan() -> ScanEvidence {
    ScanEvidence {
        adapter: "not_run".into(),
        verdict: "not_run".into(),
        reason: None,
    }
}

fn effective_actor(actor: &str) -> &str {
    if actor.trim().is_empty() { DEFAULT_ACTOR } else { actor }
}

fn non_empty_str(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

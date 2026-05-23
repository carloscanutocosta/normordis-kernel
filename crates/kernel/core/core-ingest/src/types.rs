use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use core_audit::AuditEvent;
use core_exports::ExportSnapshot;

use crate::error::IngestError;

pub const DECISION_ACCEPTED: &str = "accepted";
pub const DECISION_REJECTED: &str = "rejected";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestSource {
    pub kind: String,
    pub subject_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub request_id: String,
    pub received_at: DateTime<Utc>,
    pub source: IngestSource,
    pub expected_hash: String,
    pub bundle: ExportSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEvidence {
    pub algorithm: String,
    pub expected_hash: String,
    pub actual_hash: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEvidence {
    pub adapter: String,
    pub verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationEvidence {
    pub contract: String,
    pub valid: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteEvidence {
    pub routed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvidence {
    pub required: bool,
    pub emitted: bool,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEvidence {
    pub request_id: String,
    pub correlation_id: String,
    pub decision: String,
    pub received_at: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
    pub source: IngestSource,
    pub bundle_ref: String,
    pub hash: HashEvidence,
    pub scan: ScanEvidence,
    pub validation: ValidationEvidence,
    pub route: RouteEvidence,
    pub audit: AuditEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ScanInput {
    pub request_id: String,
    pub correlation_id: String,
    pub bundle_hash: String,
    pub bundle: ExportSnapshot,
    pub payload: Vec<u8>,
}

/// Resultado de um scan. `reason` é `Some` apenas quando o verdict não é "clean".
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub adapter: String,
    pub verdict: String,
    pub reason: Option<String>,
}

pub trait ScanAdapter: Send + Sync {
    fn scan(&self, input: &ScanInput) -> Result<ScanResult, IngestError>;
    fn adapter_id(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct RouteInput {
    pub request_id: String,
    pub correlation_id: String,
    pub bundle_hash: String,
    pub bundle: ExportSnapshot,
    pub evidence: IngestEvidence,
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub target: String,
    pub route_ref: String,
}

pub trait Router: Send + Sync {
    fn route(&self, input: &RouteInput) -> Result<RouteResult, IngestError>;
}

pub struct IngestConfig {
    pub scanner: Option<Box<dyn ScanAdapter>>,
    pub router: Option<Box<dyn Router>>,
    pub max_bundle_bytes: Option<usize>,
    pub actor: String,
    pub now: Option<fn() -> DateTime<Utc>>,
    /// Allowed `source.kind` values. `None` accepts any kind (no restriction).
    pub allowed_source_kinds: Option<Vec<String>>,
}

/// Evidence + audit event de um ingest concluído (aceite ou rejeitado).
#[derive(Debug, Clone)]
pub struct Outcome {
    pub evidence: IngestEvidence,
    pub audit_event: AuditEvent,
}

/// Resultado de `process_export_snapshot`, com o estado encoded no tipo.
///
/// - `Accepted(Outcome)` — bundle aceite e encaminhado; `outcome.evidence.decision == "accepted"`.
/// - `Rejected { outcome, error }` — bundle rejeitado; `outcome.evidence` descreve o ponto
///   de falha, `error` é o erro canonical com `code()` e `is_retryable()`.
#[derive(Debug)]
pub enum IngestOutcome {
    Accepted(Outcome),
    Rejected { outcome: Outcome, error: IngestError },
}

impl IngestOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted(_))
    }

    pub fn outcome(&self) -> &Outcome {
        match self {
            Self::Accepted(o) | Self::Rejected { outcome: o, .. } => o,
        }
    }

    pub fn into_outcome(self) -> Outcome {
        match self {
            Self::Accepted(o) | Self::Rejected { outcome: o, .. } => o,
        }
    }

    pub fn error(&self) -> Option<&IngestError> {
        match self {
            Self::Accepted(_) => None,
            Self::Rejected { error, .. } => Some(error),
        }
    }

    /// Extrai o `Outcome` em caso de sucesso; entra em pânico com `msg` em caso de rejeição.
    /// Útil em testes.
    pub fn expect_accepted(self, msg: &str) -> Outcome {
        match self {
            Self::Accepted(o) => o,
            Self::Rejected { error, .. } => panic!("{msg}: {error}"),
        }
    }

    /// Extrai `(Outcome, IngestError)` em caso de rejeição; entra em pânico em caso de sucesso.
    /// Útil em testes.
    pub fn expect_rejected(self, msg: &str) -> (Outcome, IngestError) {
        match self {
            Self::Rejected { outcome, error } => (outcome, error),
            Self::Accepted(_) => panic!("{msg}: esperado rejected, foi accepted"),
        }
    }
}

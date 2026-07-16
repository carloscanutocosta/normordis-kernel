use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use core_audit::AuditEvent;

use crate::error::IngestError;

mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(de)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

// ── Tipos de fronteira ─────────────────────────────────────────────────────────

/// Tipo de fronteira real: representa dados externos tal como chegaram.
///
/// O hash é calculado sobre `raw` antes de qualquer parsing — esta ordem é obrigatória
/// para segurança e auditabilidade. `declared_hash` é o hash que o remetente declarou;
/// quando `None`, o pipeline calcula e regista mas não verifica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestBundle {
    pub bundle_id: String,
    pub received_at: DateTime<Utc>,
    pub source: IngestSource,
    /// Bytes raw tal como chegaram — não parsados. Serializado como base64 standard (RFC 4648).
    #[serde(with = "base64_bytes")]
    pub raw: Vec<u8>,
    /// MIME type declarado pelo remetente (ex.: "application/pdf", "application/xml").
    pub content_type: String,
    /// Hash SHA-256 declarado pelo remetente, no formato "sha256:<hex>".
    /// Se presente, o pipeline verifica; se ausente, apenas regista.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestSource {
    /// Tipo semântico do bundle (ex.: "cius-pt-invoice", "saft-pt", "pdf-official").
    pub kind: String,
    /// Identificador do sujeito (entidade, processo, documento).
    pub subject_id: String,
    /// Versão declarada pelo remetente.
    pub version: String,
}

// ── Decisão ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestDecision {
    Accepted,
    Rejected,
}

impl fmt::Display for IngestDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted => write!(f, "accepted"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

// ── Sub-tipos de evidência ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEvidence {
    pub algorithm: String,
    pub declared_hash: String,
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
    pub content_type: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvidence {
    pub required: bool,
    pub emitted: bool,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

// ── Evidência completa ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEvidence {
    pub bundle_id: String,
    pub correlation_id: String,
    pub decision: IngestDecision,
    pub received_at: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
    pub source: IngestSource,
    pub content_type: String,
    pub hash: HashEvidence,
    pub scan: ScanEvidence,
    pub validation: ValidationEvidence,
    /// Referência ao documento armazenado em core-documental. Presente apenas em `Accepted`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ref: Option<String>,
    pub audit: AuditEvidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

// ── Outcome ────────────────────────────────────────────────────────────────────

/// Evidence + audit event de um ingest concluído.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub evidence: IngestEvidence,
    pub audit_event: AuditEvent,
}

/// Resultado de `process_bundle`, com o estado encoded no tipo.
///
/// - `Accepted(Outcome)` — bundle aceite e guardado; `evidence.decision == Accepted`.
/// - `Rejected { outcome, error }` — bundle rejeitado; `evidence` descreve o ponto
///   de falha, `error` tem `code()` e `is_retryable()`.
#[derive(Debug)]
pub enum IngestOutcome {
    Accepted(Outcome),
    Rejected {
        outcome: Outcome,
        error: IngestError,
    },
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

    pub fn expect_accepted(self, msg: &str) -> Outcome {
        match self {
            Self::Accepted(o) => o,
            Self::Rejected { error, .. } => panic!("{msg}: {error}"),
        }
    }

    pub fn expect_rejected(self, msg: &str) -> (Outcome, IngestError) {
        match self {
            Self::Rejected { outcome, error } => (outcome, error),
            Self::Accepted(_) => panic!("{msg}: esperado rejected, foi accepted"),
        }
    }
}

// ── Traits de extensão ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScanInput {
    pub bundle_id: String,
    pub correlation_id: String,
    pub bundle_hash: String,
    pub content_type: String,
    pub raw: Vec<u8>,
}

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

/// Valida o conteúdo de um bundle com base no seu MIME type.
///
/// Para XML: deve implementar XXE prevention antes de qualquer parsing e pode
/// validar contra XSD quando `schema_id` estiver declarado na source.
/// Para PDF: pode verificar magic bytes e estrutura básica.
pub trait ContentValidator: Send + Sync {
    fn validate(&self, raw: &[u8], content_type: &str) -> Result<(), IngestError>;
}

/// Port de armazenamento — core-ingest define a interface; a infra implementa com core-documental.
///
/// Retorna um `document_ref` opaco que identifica o documento armazenado.
pub trait IngestStoragePort: Send + Sync {
    fn store(&self, bundle: &IngestBundle, verified_hash: &str) -> Result<String, IngestError>;
}

// ── Configuração ──────────────────────────────────────────────────────────────

pub struct IngestConfig {
    pub scanner: Option<Box<dyn ScanAdapter>>,
    pub content_validator: Option<Box<dyn ContentValidator>>,
    pub storage: Option<Box<dyn IngestStoragePort>>,
    pub max_bundle_bytes: Option<usize>,
    pub actor: String,
    pub now: Option<fn() -> DateTime<Utc>>,
    /// Allowed `source.kind` values. `None` accepts any kind.
    pub allowed_source_kinds: Option<Vec<String>>,
}

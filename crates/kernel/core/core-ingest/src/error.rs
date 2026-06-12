use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const INGEST_COMPONENT: &str = "core-ingest";
pub const MISSING_FIELD: &str = "MINI.INGEST.MISSING_FIELD";
pub const INVALID_REQUEST: &str = "MINI.INGEST.INVALID_REQUEST";
pub const HASH_MISMATCH: &str = "MINI.INGEST.HASH_MISMATCH";
pub const SCAN_FAILED: &str = "MINI.INGEST.SCAN_FAILED";
pub const SCAN_REJECTED: &str = "MINI.INGEST.SCAN_REJECTED";
pub const CONTENT_VALIDATION_FAILED: &str = "MINI.INGEST.CONTENT_VALIDATION_FAILED";
pub const STORE_FAILED: &str = "MINI.INGEST.STORE_FAILED";
pub const OVERSIZED: &str = "MINI.INGEST.OVERSIZED";
pub const MARSHAL_FAILED: &str = "MINI.INGEST.MARSHAL_FAILED";
pub const AUDIT_ERROR: &str = "MINI.INGEST.AUDIT_ERROR";

#[derive(Debug, Clone, Error)]
pub enum IngestError {
    #[error("campo obrigatório em falta: {field}")]
    MissingField { field: String },

    #[error("pedido de ingest inválido: {message}")]
    InvalidRequest { message: String },

    #[error("hash do bundle não corresponde: esperado={expected}, obtido={actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("scan do bundle falhou")]
    ScanFailed,

    #[error("bundle rejeitado pelo scan: adapter={adapter}, verdict={verdict}")]
    ScanRejected { adapter: String, verdict: String },

    /// Validação de conteúdo falhou — inclui XXE detection em XML, estrutura inválida
    /// em PDF, ou schema XSD não conforme.
    #[error("validação de conteúdo falhou ({content_type}): {reason}")]
    ContentValidationFailed { content_type: String, reason: String },

    #[error("falha ao guardar bundle: {0}")]
    StoreFailed(String),

    #[error("bundle excede limite: limite={limit_bytes}B, tamanho={actual_bytes}B")]
    Oversized {
        limit_bytes: usize,
        actual_bytes: usize,
    },

    #[error("falha a serializar: {0}")]
    MarshalFailed(String),

    #[error("erro de audit: {0}")]
    AuditError(String),
}

impl IngestError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingField { .. } => MISSING_FIELD,
            Self::InvalidRequest { .. } => INVALID_REQUEST,
            Self::HashMismatch { .. } => HASH_MISMATCH,
            Self::ScanFailed => SCAN_FAILED,
            Self::ScanRejected { .. } => SCAN_REJECTED,
            Self::ContentValidationFailed { .. } => CONTENT_VALIDATION_FAILED,
            Self::StoreFailed(_) => STORE_FAILED,
            Self::Oversized { .. } => OVERSIZED,
            Self::MarshalFailed(_) => MARSHAL_FAILED,
            Self::AuditError(_) => AUDIT_ERROR,
        }
    }

    /// Erros retryable resultam de falhas transitórias de infra.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::ScanFailed | Self::StoreFailed(_))
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-ingest error codes are valid"),
            Component::new(INGEST_COMPONENT).expect("core-ingest component is valid"),
            self.to_string(),
        )
    }
}

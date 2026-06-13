//! Templates NDT versionados — imutáveis após chegada ao core-documental,
//! com verificação de integridade por hash SHA-256.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AuthoritySnapshot, DocumentTypeCode, DocumentalError};

// ── TemplateId ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateId(pub String);

impl TemplateId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(DocumentalError::EmptyField("template_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── TemplateStatus ────────────────────────────────────────────────────────────

/// Estado custodial de um template NDT.
///
/// Templates chegam ao core-documental já `Active`.
/// `Deprecated` preserva o template para reconstituição histórica de documentos existentes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    Active,
    Deprecated,
}

impl TemplateStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deprecated => "deprecated",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "deprecated" => Some(Self::Deprecated),
            _ => None,
        }
    }
}

impl TryFrom<&str> for TemplateStatus {
    type Error = DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            DocumentalError::OperationFailed(format!("estado de template desconhecido: {s}"))
        })
    }
}

// ── DocumentTemplate ──────────────────────────────────────────────────────────

/// Template NDT versionado e imutável sob custódia institucional.
///
/// O conteúdo NDT é verificado por hash SHA-256 calculado externamente.
/// `created_by` captura o snapshot de autoridade de quem entregou o template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTemplate {
    pub id: TemplateId,
    pub code: String,
    pub document_type: DocumentTypeCode,
    pub version: String,
    pub content_ndt: String,
    pub content_hash: String,
    pub status: TemplateStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: AuthoritySnapshot,
}

impl DocumentTemplate {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        if self.code.trim().is_empty() {
            return Err(DocumentalError::EmptyField("code".into()));
        }
        if self.document_type.0.trim().is_empty() {
            return Err(DocumentalError::EmptyField("document_type".into()));
        }
        if self.version.trim().is_empty() {
            return Err(DocumentalError::EmptyField("version".into()));
        }
        if self.content_ndt.trim().is_empty() {
            return Err(DocumentalError::EmptyField("content_ndt".into()));
        }
        if self.content_hash.trim().is_empty() {
            return Err(DocumentalError::EmptyField("content_hash".into()));
        }
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, TemplateStatus::Active)
    }

    /// Templates são sempre imutáveis no core-documental.
    pub fn is_immutable(&self) -> bool {
        true
    }

    /// Verifica que o hash pré-computado corresponde ao hash registado.
    pub fn verify_content_hash(&self, computed_hash: &str) -> Result<(), DocumentalError> {
        if computed_hash != self.content_hash {
            return Err(DocumentalError::ContentHashMismatch);
        }
        Ok(())
    }
}

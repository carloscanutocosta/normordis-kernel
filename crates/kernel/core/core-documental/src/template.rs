//! Templates NDT versionados — write-once após activação, com verificação de
//! integridade por hash SHA-256.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AuthorityContext, DocumentalError};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    Draft,
    Active,
    Deprecated,
}

/// Template NDT versionado e imutável após activação.
///
/// Cada tipo de documento tem uma série de versões numeradas.
/// Versões `Active` são imutáveis — qualquer alteração cria nova versão.
/// O conteúdo NDT é armazenado em texto e verificado por hash SHA-256.
/// O hash é calculado externamente (infra/service layer) e verificado
/// pelo domínio via `verify_content_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTemplate {
    pub id: TemplateId,
    pub code: String,
    pub document_type: String,
    pub version: String,
    pub content_ndt: String,
    pub content_hash: String,
    pub status: TemplateStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: AuthorityContext,
}

impl DocumentTemplate {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        if self.code.trim().is_empty() {
            return Err(DocumentalError::EmptyField("code".into()));
        }
        if self.document_type.trim().is_empty() {
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

    /// Templates `Active` e `Deprecated` são imutáveis — não podem ser editados.
    /// Qualquer alteração exige criar nova versão com novo id.
    pub fn is_immutable(&self) -> bool {
        matches!(
            self.status,
            TemplateStatus::Active | TemplateStatus::Deprecated
        )
    }

    /// Verifica que o template pode ser activado (estado deve ser `Draft`).
    /// A transição efectiva é da responsabilidade do `TemplateRepository::activate`.
    pub fn activate(&self) -> Result<(), DocumentalError> {
        match self.status {
            TemplateStatus::Draft => Ok(()),
            TemplateStatus::Active => Err(DocumentalError::TemplateImmutable),
            TemplateStatus::Deprecated => Err(DocumentalError::TemplateNotActivatable),
        }
    }

    /// Verifica que o hash pré-computado corresponde ao hash registado.
    /// O cálculo SHA-256 é responsabilidade do chamador (infra/service layer).
    pub fn verify_content_hash(&self, computed_hash: &str) -> Result<(), DocumentalError> {
        if computed_hash != self.content_hash {
            return Err(DocumentalError::ContentHashMismatch);
        }
        Ok(())
    }
}

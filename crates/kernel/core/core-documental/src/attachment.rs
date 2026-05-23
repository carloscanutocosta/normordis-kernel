//! Documentos binários em guarda institucional — metadados e port de armazenamento.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AuthorityContext, DocumentId, DocumentalError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttachmentId(pub String);

impl AttachmentId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(DocumentalError::EmptyField("attachment_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    /// Anexo produzido em conjunto com o documento (ex: PDF gerado, certidão).
    Annex,
    /// Documento entrado que não deve ser convertido (ex: requerimento digitalizado,
    /// declaração em papel, ofício recebido).
    Incoming,
}

impl AttachmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Annex => "annex",
            Self::Incoming => "incoming",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "annex" => Some(Self::Annex),
            "incoming" => Some(Self::Incoming),
            _ => None,
        }
    }
}

impl TryFrom<&str> for AttachmentKind {
    type Error = crate::DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            crate::DocumentalError::OperationFailed(format!("tipo de anexo desconhecido: {s}"))
        })
    }
}

/// Documento binário em guarda institucional.
///
/// O conteúdo é endereçado pelo hash SHA-256 (`content_hash`), que é também
/// o nome do ficheiro em armazenamento. Garante deduplicação implícita e
/// imutabilidade: o mesmo conteúdo tem sempre o mesmo identificador.
///
/// O agregado guarda apenas metadados — nunca o conteúdo binário em memória.
/// O conteúdo é gerido exclusivamente pelo port `AttachmentStore`.
///
/// O `original_filename` serve apenas para exibição ao utilizador;
/// o nome real em armazenamento é sempre o `content_hash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentAttachment {
    pub id: AttachmentId,
    pub document_id: DocumentId,
    pub kind: AttachmentKind,
    /// Nome original do ficheiro, apenas para exibição.
    pub original_filename: String,
    /// MIME type (ex: "application/pdf", "image/jpeg", "application/msword").
    pub content_type: String,
    /// SHA-256 hex do conteúdo — também é o nome do ficheiro em armazenamento.
    pub content_hash: String,
    pub size_bytes: u64,
    pub description: Option<String>,
    pub stored_at: DateTime<Utc>,
    pub stored_by: AuthorityContext,
}

impl DocumentAttachment {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.document_id.validate()?;
        if self.original_filename.trim().is_empty() {
            return Err(DocumentalError::EmptyField("original_filename".into()));
        }
        if self.content_type.trim().is_empty() {
            return Err(DocumentalError::EmptyField("content_type".into()));
        }
        if self.content_hash.trim().is_empty() {
            return Err(DocumentalError::EmptyField("content_hash".into()));
        }
        if self.size_bytes == 0 {
            return Err(DocumentalError::EmptyField("size_bytes".into()));
        }
        Ok(())
    }

    /// Nome do ficheiro em armazenamento — igual ao hash do conteúdo.
    pub fn storage_filename(&self) -> &str {
        &self.content_hash
    }

    /// Verifica que o hash pré-computado corresponde ao hash registado.
    /// O cálculo SHA-256 é responsabilidade do chamador (infra/service layer).
    pub fn verify_content_integrity(&self, computed_hash: &str) -> Result<(), DocumentalError> {
        if computed_hash != self.content_hash {
            return Err(DocumentalError::ContentHashMismatch);
        }
        Ok(())
    }
}

//! Arquivo NDF write-once — registo imutável de documentos renderizados para
//! custódia definitiva e verificação de integridade histórica.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AuthoritySnapshot, DocumentId, DocumentalError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NdfRecordId(pub String);

impl NdfRecordId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(DocumentalError::EmptyField("ndf_record_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Registo de arquivo NDF — write-once.
///
/// Uma vez escrito, o registo não pode ser modificado. O `ndf_hash` é verificado
/// para garantir integridade. O `template_hash` permite reconstruir o documento
/// exactamente como foi emitido, mesmo que o template evolua.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NdfRecord {
    pub id: NdfRecordId,
    pub document_id: DocumentId,
    pub ndf_json: String,
    pub ndf_hash: String,
    pub template_hash: String,
    pub rendered_at: DateTime<Utc>,
    pub rendered_by: AuthoritySnapshot,
}

impl NdfRecord {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.document_id.validate()?;
        if self.ndf_json.trim().is_empty() {
            return Err(DocumentalError::EmptyField("ndf_json".into()));
        }
        if self.ndf_hash.trim().is_empty() {
            return Err(DocumentalError::EmptyField("ndf_hash".into()));
        }
        if self.template_hash.trim().is_empty() {
            return Err(DocumentalError::EmptyField("template_hash".into()));
        }
        Ok(())
    }

    pub fn verify_integrity(&self, computed_hash: &str) -> Result<(), DocumentalError> {
        if computed_hash != self.ndf_hash {
            return Err(DocumentalError::NdfHashMismatch);
        }
        Ok(())
    }
}

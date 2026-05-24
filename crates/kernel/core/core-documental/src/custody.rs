//! Agregado central de custódia documental — ciclo de vida, estados, relações
//! e invariantes de finalização.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AuthorityContext, DocumentalError, TemplateId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        validate_document_id(&id)?;
        Ok(Self(id))
    }

    pub fn validate(&self) -> Result<(), DocumentalError> {
        validate_document_id(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for DocumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        Self::new(id).map_err(serde::de::Error::custom)
    }
}

fn validate_document_id(id: &str) -> Result<(), DocumentalError> {
    if id.trim().is_empty() {
        return Err(DocumentalError::EmptyField("document_id".into()));
    }
    if id != id.trim() {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode ter espaços no início ou no fim".into(),
        });
    }
    if id.len() > 128 {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode exceder 128 bytes".into(),
        });
    }
    if id == "." || id == ".." || id.contains("..") {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode conter segmentos de navegação".into(),
        });
    }
    if id.contains('/') || id.contains('\\') {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode conter separadores de caminho".into(),
        });
    }
    if !id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "só pode conter ASCII alfanumérico, hífen, underscore ou ponto".into(),
        });
    }
    Ok(())
}

impl DocumentId {
    pub fn from_existing(id: impl Into<String>) -> Result<Self, DocumentalError> {
        Self::new(id)
    }

    pub fn ensure_safe_storage_key(&self) -> Result<(), DocumentalError> {
        self.validate()
    }
}

impl DocumentRelation {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.from_id.validate()?;
        self.to_id.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    PendingApproval,
    Approved,
    Finalized,
    Archived,
    Annulled,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Finalized => "finalized",
            Self::Archived => "archived",
            Self::Annulled => "annulled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "pending_approval" => Some(Self::PendingApproval),
            "approved" => Some(Self::Approved),
            "finalized" => Some(Self::Finalized),
            "archived" => Some(Self::Archived),
            "annulled" => Some(Self::Annulled),
            _ => None,
        }
    }
}

impl TryFrom<&str> for DocumentStatus {
    type Error = crate::DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            crate::DocumentalError::OperationFailed(format!("estado desconhecido: {s}"))
        })
    }
}

impl DocumentStatus {
    /// Transições válidas:
    /// - Draft → PendingApproval | Archived
    /// - PendingApproval → Approved | Draft (rejeição)
    /// - Approved → Finalized | Draft (rejeição)
    /// - Finalized → Archived | Annulled
    /// - Archived e Annulled são estados terminais
    pub fn can_transition_to(&self, next: &DocumentStatus) -> bool {
        use DocumentStatus::*;
        matches!(
            (self, next),
            (Draft, PendingApproval)
                | (Draft, Archived)
                | (PendingApproval, Approved)
                | (PendingApproval, Draft)
                | (Approved, Finalized)
                | (Approved, Draft)
                | (Finalized, Archived)
                | (Finalized, Annulled)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    ReplyTo,
    References,
    Supersedes,
    Annuls,
    /// Documento formal que é anexo de outro documento (ex: mapa, quadro, escritura).
    /// Distinto de `AttachmentStore`: aqui ambos os lados são documentos custodiados;
    /// `AttachmentStore` gere blobs binários que não são documentos autónomos.
    AnnexDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRelation {
    pub relation_type: RelationType,
    pub from_id: DocumentId,
    pub to_id: DocumentId,
    pub established_at: DateTime<Utc>,
}

/// Agregado central de custódia documental.
///
/// Representa o ciclo de vida de um documento institucional.
/// O `authority_context` fica `None` até à finalização — a captura da
/// autoridade jurídica é o acto que confere validade ao documento.
/// O `document_number` é atribuído exactamente uma vez (invariante de domínio).
///
/// As relações entre documentos são geridas exclusivamente pelo port
/// `DocumentCustodyRepository` — não estão embutidas no agregado para
/// evitar leitura desnecessária e inconsistência de estado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentCustody {
    pub id: DocumentId,
    pub document_type: String,
    pub template_id: TemplateId,
    pub template_version: String,
    pub status: DocumentStatus,
    pub payload_json: Value,
    pub authority_context: Option<AuthorityContext>,
    pub document_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DocumentCustody {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.id.validate()?;
        if self.document_type.trim().is_empty() {
            return Err(DocumentalError::EmptyField("document_type".into()));
        }
        if self.template_version.trim().is_empty() {
            return Err(DocumentalError::EmptyField("template_version".into()));
        }
        Ok(())
    }

    pub fn is_finalized(&self) -> bool {
        matches!(
            self.status,
            DocumentStatus::Finalized | DocumentStatus::Archived | DocumentStatus::Annulled
        )
    }

    pub fn transition_to(&self, next: DocumentStatus) -> Result<DocumentStatus, DocumentalError> {
        if !self.status.can_transition_to(&next) {
            return Err(DocumentalError::InvalidStatusTransition(
                self.status.as_str().to_string(),
                next.as_str().to_string(),
            ));
        }
        Ok(next)
    }

    /// Atribui número exactamente uma vez.
    pub fn assign_number(&self, number: &str) -> Result<(), DocumentalError> {
        if self.document_number.is_some() {
            return Err(DocumentalError::NumberAlreadyAssigned);
        }
        if number.trim().is_empty() {
            return Err(DocumentalError::EmptyDocumentNumber);
        }
        Ok(())
    }

    /// Verifica que o documento está pronto para finalização:
    /// - autoridade jurídica capturada
    /// - número de documento atribuído
    pub fn check_ready_to_finalize(&self) -> Result<(), DocumentalError> {
        if self.authority_context.is_none() {
            return Err(DocumentalError::MissingAuthorityContext);
        }
        if self.document_number.is_none() {
            return Err(DocumentalError::MissingDocumentNumber);
        }
        Ok(())
    }

    /// Finalização atómica: verifica as pré-condições e devolve o próximo status.
    /// Equivale a `check_ready_to_finalize()` + `transition_to(Finalized)` numa
    /// única operação que não pode ser chamada a meio — ou tudo ou nada.
    /// O chamador deve usar este método em vez de compor as duas chamadas separadas.
    ///
    /// TODO: fluxos multi-assinatura (informação → parecer → despacho)
    /// O documento é agnóstico das fases de assinatura — não as conhece nem as enforça.
    /// A validação de fases pertence a um futuro `DocumentSigningService` que:
    ///   1. Lê o template NDT para extrair as fases obrigatórias/facultativas
    ///   2. Verifica o event log (eventos `Signed` com `data_json.fase`) contra essas fases
    ///   3. Só depois chama `doc.finalize()` se todas as fases obrigatórias estiverem satisfeitas
    ///
    /// Decisão pendente: formato no NDT para declarar fases de assinatura.
    pub fn finalize(&self) -> Result<DocumentStatus, DocumentalError> {
        self.check_ready_to_finalize()?;
        self.transition_to(DocumentStatus::Finalized)
    }
}

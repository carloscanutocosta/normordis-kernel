//! Runtime partilhado de mini-apps AT.
//!
//! Camada de orquestração fina: constrói `DocumentCustody`, regista o primeiro
//! evento documental e delega numeração e auditoria para os respectivos sub-sistemas.
//! Não conhece SQLite, filesystem, Tauri ou UI.

use chrono::{DateTime, Utc};
use core_audit::{AuditActor, AuditEvent, AuditOutcome, AuditStore, AuditTarget};
use core_documental::{
    DocumentCustody, DocumentEvent, DocumentEventId, DocumentEventType, DocumentId, DocumentStatus,
    EventActor, TemplateId,
};
use core_org::{OrgPositionId, OrgUnit};
use core_rh::{AuthorMetadata, UserContext, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct MiniAppContext {
    pub app_name: String,
    pub user_context: UserContext,
    pub org_config: OrgUnit,
}

/// Pedido de criação de um documento custodiado.
///
/// `document_type`, `template_id` e `template_version` substituem o antigo
/// `DocumentDefinition` — os metadados de template ficam no pedido, não
/// num agregado separado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub document_id: String,
    pub document_type: String,
    pub template_id: String,
    pub template_version: String,
    pub payload_json: Value,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("nome da app vazio")]
    EmptyAppName,
    #[error("utilizador não tem posição orgânica activa na sessão")]
    MissingPosition,
    #[error("erro de documento: {0}")]
    DocumentError(String),
    #[error("erro de numeração: {0}")]
    Numbering(String),
    #[error("erro de auditoria: {0}")]
    Audit(String),
}

impl MiniAppContext {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.app_name.trim().is_empty() {
            return Err(RuntimeError::EmptyAppName);
        }
        Ok(())
    }
}

/// Constrói um novo `DocumentCustody` em estado `Draft`.
///
/// Substitui o par `(DocumentDefinition, create_document_instance)` do runtime anterior.
/// O template é referenciado por `request.template_id` e `request.template_version`.
pub fn create_document_instance(
    context: &MiniAppContext,
    request: CreateDocumentRequest,
    now: DateTime<Utc>,
) -> Result<DocumentCustody, RuntimeError> {
    context.validate()?;
    let id = DocumentId::new(request.document_id)
        .map_err(|e| RuntimeError::DocumentError(e.to_string()))?;
    let template_id = TemplateId::new(request.template_id)
        .map_err(|e| RuntimeError::DocumentError(e.to_string()))?;
    Ok(DocumentCustody {
        id,
        document_type: request.document_type,
        template_id,
        template_version: request.template_version,
        status: DocumentStatus::Draft,
        payload_json: request.payload_json,
        authority_context: None,
        document_number: None,
        created_at: now,
        updated_at: now,
    })
}

/// Constrói o evento `Created` para o primeiro evento da cadeia documental.
///
/// `previous_hash` é sempre `None` — este é o evento inaugural do documento.
/// Requer que `context.user_context.org_position` esteja preenchido.
pub fn create_document_created_event(
    document: &DocumentCustody,
    context: &MiniAppContext,
    at: DateTime<Utc>,
) -> Result<DocumentEvent, RuntimeError> {
    let pos = context
        .user_context
        .org_position
        .as_ref()
        .ok_or(RuntimeError::MissingPosition)?;

    let user_id = UserId::new(context.user_context.current_user.user_id.clone())
        .map_err(|e| RuntimeError::DocumentError(e.to_string()))?;
    let position_id = OrgPositionId::new(pos.position_id.clone())
        .map_err(|e| RuntimeError::DocumentError(e.to_string()))?;
    let event_id = DocumentEventId::new(format!("evt-created-{}", document.id.as_str()))
        .map_err(|e| RuntimeError::DocumentError(e.to_string()))?;

    Ok(DocumentEvent {
        id: event_id,
        document_id: document.id.clone(),
        event_type: DocumentEventType::Created,
        actor: EventActor::Operator {
            user_id,
            position_id,
        },
        occurred_at: at,
        previous_hash: None,
        data_json: None,
    })
}

pub fn record_document_created<R: AuditStore>(
    recorder: &R,
    context: &MiniAppContext,
    document: &DocumentCustody,
    at: DateTime<Utc>,
) -> Result<(), RuntimeError> {
    let actor = AuditActor::with_metadata(
        context.user_context.current_user.user_id.clone(),
        Some(context.user_context.current_user.display_name.clone()),
        Some("local-user".to_string()),
    )
    .map_err(|e| RuntimeError::Audit(e.to_string()))?;
    let target = AuditTarget::new("document", document.id.as_str().to_string())
        .map_err(|e| RuntimeError::Audit(e.to_string()))?;
    let event = AuditEvent::with_id_and_time(
        format!("document-created-{}", document.id.as_str()),
        "document.created",
        actor,
        target,
        at,
        AuditOutcome::Success,
        None,
        Some(serde_json::json!({
            "app": context.app_name,
            "template_id": document.template_id.as_str(),
            "org_unit": context.org_config.service_code.clone()
                .unwrap_or_else(|| context.org_config.short_name.clone())
        })),
    )
    .map_err(|e| RuntimeError::Audit(e.to_string()))?;
    recorder
        .record(&event)
        .map_err(|e| RuntimeError::Audit(e.to_string()))
}

pub fn author_from_context(user_context: &UserContext) -> AuthorMetadata {
    user_context.current_user.author_metadata()
}

//! Domínio de custódia documental institucional do Mini-Kernel RS.
//!
//! O core-documental é o custodiante institucional dos documentos NORMORDIS.
//! Guarda, prova e rastreia documentos já finalizados ao longo do seu ciclo
//! de vida custodial. Não produz, não renderiza, não exporta.

#[cfg(test)]
mod tests;

pub mod archive;
pub mod attachment;
pub mod authority;
pub mod custody;
pub mod error;
pub mod events;
pub mod package;
pub mod ports;
pub mod service;
pub mod template;

// ── archive ───────────────────────────────────────────────────────────────────
pub use archive::{NdfRecord, NdfRecordId};

// ── attachment ────────────────────────────────────────────────────────────────
pub use attachment::{AttachmentId, AttachmentKind, DocumentAttachment};

// ── authority ─────────────────────────────────────────────────────────────────
pub use authority::AuthoritySnapshot;

// ── custody ───────────────────────────────────────────────────────────────────
pub use custody::{
    DocumentContent, DocumentCustody, DocumentId, DocumentOrigin, DocumentRelation, DocumentStatus,
    DocumentTypeCode, EntryChannel, IntakeSpec, RelationType, RetentionClass, RetentionPolicy,
    ValidationCode,
};

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::DocumentalError;

// ── events ────────────────────────────────────────────────────────────────────
pub use events::{
    verify_event_chain, AccessPurpose, DocumentEvent, DocumentEventId, DocumentEventType,
    EventActor, EventFilter,
};

// ── package ───────────────────────────────────────────────────────────────────
pub use package::{
    validate_document_package, Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef,
};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{
    AttachmentStore, DocumentCustodyRepository, DocumentEventLog, NdfArchive, TemplateRepository,
};

// ── service ───────────────────────────────────────────────────────────────────
pub use service::{authority_from_user_context, DocumentCustodyService};

// ── template ──────────────────────────────────────────────────────────────────
pub use template::{DocumentTemplate, TemplateId, TemplateStatus};

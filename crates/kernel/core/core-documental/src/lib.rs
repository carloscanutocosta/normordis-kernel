#![allow(clippy::should_implement_trait)]

//! Domínio de custódia documental institucional do Mini-Kernel RS.
//!
//! Cobre o ciclo de vida completo de documentos institucionais: criação, edição,
//! aprovação, finalização, arquivo e anulação. Exporta tipos, invariantes e ports
//! de persistência. Não conhece SQLite, filesystem, Tauri ou UI.

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
pub mod template;

// ── archive ───────────────────────────────────────────────────────────────────
pub use archive::{NdfRecord, NdfRecordId};

// ── attachment ────────────────────────────────────────────────────────────────
pub use attachment::{AttachmentId, AttachmentKind, DocumentAttachment};

// ── authority ─────────────────────────────────────────────────────────────────
pub use authority::AuthorityContext;

// ── custody ───────────────────────────────────────────────────────────────────
pub use custody::{DocumentCustody, DocumentId, DocumentRelation, DocumentStatus, RelationType};

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::DocumentalError;

// ── events ────────────────────────────────────────────────────────────────────
pub use events::{
    verify_event_chain, DocumentEvent, DocumentEventId, DocumentEventType, EventActor,
};

// ── package ───────────────────────────────────────────────────────────────────
pub use package::{
    validate_document_package, Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef,
};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{
    AttachmentStore, DocumentCustodyRepository, DocumentEventLog, NdfArchive, TemplateRepository,
};

// ── template ──────────────────────────────────────────────────────────────────
pub use template::{DocumentTemplate, TemplateId, TemplateStatus};

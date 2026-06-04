//! Domínio de identidade e gestão de utilizadores do Mini-Kernel RS.
//!
//! Cobre identificação de utilizadores (`UserId`, `UserProfile`), papéis funcionais
//! (`UserRole`, `Role`), sessões, referências orgânicas, afetações temporais
//! pessoa ↔ posição, e infraestrutura de evidência COSO (outbox, auditoria).
//! Exporta tipos, invariantes e funções de validação. Não conhece SQLite, filesystem,
//! Tauri ou UI.

#[cfg(test)]
mod tests;

pub mod assignment;
pub mod audit;
pub mod controls;
pub mod error;
pub mod identity;
pub mod org;
pub mod ports;
pub mod role;
pub mod service;
pub mod session;
pub mod user;
pub mod validate;

// ── assignment ────────────────────────────────────────────────────────────────
pub use assignment::{PersonAssignment, PersonAssignmentId};

// ── audit ─────────────────────────────────────────────────────────────────────
pub use audit::{
    audit_actor_from_user, RhAuditAction, RhAuditEvent, RhAuditPort, RhEventOutcome, RhNoopAudit,
};

// ── controls ──────────────────────────────────────────────────────────────────
pub use controls::{ASSIGN_PERSON, CLOSE_ASSIGNMENT, DEACTIVATE_USER, UPSERT_USER};

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{RhError, COMPONENT};

// ── identity ──────────────────────────────────────────────────────────────────
pub use identity::{resolve_current_user, AuthorMetadata, UserContext, UserIdentity};

// ── org ───────────────────────────────────────────────────────────────────────
pub use org::{OrgPositionRef, OrgUnitRef};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{PersonAssignmentRepository, RhAuditOutbox, UserRepository};

// ── role ──────────────────────────────────────────────────────────────────────
pub use role::{Role, RoleId, RoleRepository, UserRole};

// ── service ───────────────────────────────────────────────────────────────────
pub use service::{PersonAssignmentService, UserService};

// ── session ───────────────────────────────────────────────────────────────────
pub use session::{CurrentSession, CurrentUser};

// ── user ──────────────────────────────────────────────────────────────────────
pub use user::{UserId, UserProfile};

// ── validate ──────────────────────────────────────────────────────────────────
pub use validate::{
    validate_competency_id, validate_optional_email, validate_org_unit_id, validate_position_id,
    validate_required_display_name, validate_role_id, validate_user_id_value, validate_username,
    USER_ID_MAX_LENGTH,
};

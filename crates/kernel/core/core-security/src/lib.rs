//! Controlos soberanos de segurança do NORMAXIS.
//!
//! Define invariantes mínimas de segurança (zero-trust), políticas soberanas,
//! delegações temporárias de permissão, e o motor de autorização (`SecurityService`).
//! Sem dependência de SQLite, UI ou IAM concreto.

pub mod audit_log;
pub mod delegation;
pub mod error;
pub mod memory;
pub mod page;
pub mod policy;
pub mod principal;
pub mod repository;
pub mod role;
pub mod service;
pub mod write_invariant;

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{SecurityError, COMPONENT};

// ── policy ────────────────────────────────────────────────────────────────────
pub use policy::{validate_policy, Policy, PolicyMode, Rule};

// ── principal ─────────────────────────────────────────────────────────────────
pub use principal::{PrincipalKind, VerifiedPrincipal};

// ── write_invariant ───────────────────────────────────────────────────────────
pub use write_invariant::{validate_write_invariant, WriteInvariantContext};

// ── delegation ────────────────────────────────────────────────────────────────
pub use delegation::{Delegation, DelegationId, DelegationRequest, RevocationRequest};

// ── page ──────────────────────────────────────────────────────────────────────
pub use page::ListOptions;

// ── repository ────────────────────────────────────────────────────────────────
pub use repository::SecurityPolicyRepository;

// ── audit_log ─────────────────────────────────────────────────────────────────
pub use audit_log::{AuditDecision, NoopSecurityAuditLog, SecurityAuditLog, SecurityAuthDecision};
#[cfg(any(test, feature = "test-helpers"))]
pub use audit_log::InMemoryAuditLog;

// ── service ───────────────────────────────────────────────────────────────────
pub use service::{AuthorizationToken, GrantedBy, SecurityService};

// ── role ──────────────────────────────────────────────────────────────────────
pub use role::{NoopRoleMembership, RoleId, RoleMembershipRepository};
#[cfg(any(test, feature = "test-helpers"))]
pub use role::InMemoryRoleMembership;

// ── memory ────────────────────────────────────────────────────────────────────
#[cfg(any(test, feature = "test-helpers"))]
pub use memory::InMemorySecurityPolicyRepository;

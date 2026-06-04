//! Controlos soberanos de segurança do NORMORDIS.
//!
//! Define invariantes mínimas de segurança (zero-trust), políticas soberanas,
//! delegações temporárias de permissão, e o motor de autorização (`SecurityService`).
//! Sem dependência de SQLite, UI ou IAM concreto.
//!
//! ## Módulos principais
//!
//! | Módulo            | Responsabilidade                                         |
//! |-------------------|----------------------------------------------------------|
//! | `principal`       | `VerifiedPrincipal` — identidade verificada              |
//! | `write_invariant` | `WriteInvariantContext` — invariante zero-trust mínima   |
//! | `policy`          | `Policy`, `PolicyMode`, `Rule` — políticas soberanas     |
//! | `delegation`      | `Delegation` — permissões temporárias com cascata        |
//! | `service`         | `SecurityService` — motor de autorização                 |
//! | `auth_level`      | `AuthLevel` — nível de autenticação                      |
//! | `classification`  | `ResourceClassification` — sensibilidade de recursos     |
//! | `context`         | `SecurityContext`, `OrgScope`, `SessionRef`              |
//! | `authz`           | `AuthzRequest`, `AuthzDecision`, `EvidenceLevel`         |
//! | `event`           | `SecurityEvent`, `SecurityEventPublisher`                |
//! | `sod`             | `SodRule`, `check_sod` — segregação de funções           |

pub mod audit_log;
pub mod auth_level;
pub mod authz;
pub mod classification;
pub mod context;
pub mod delegation;
pub mod error;
pub mod event;
pub mod memory;
pub mod page;
pub mod policy;
pub mod ports;
pub mod principal;
pub mod repository;
pub mod role;
pub mod service;
pub mod sod;
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
#[cfg(any(test, feature = "test-helpers"))]
pub use audit_log::InMemoryAuditLog;
pub use audit_log::{AuditDecision, NoopSecurityAuditLog, SecurityAuditLog, SecurityAuthDecision};

// ── service ───────────────────────────────────────────────────────────────────
pub use service::{
    AuthorizationToken, BootstrapAuthorization, GrantedBy, SecurityFailureMode,
    SecurityRuntimePolicy, SecurityService,
};

// ── role ──────────────────────────────────────────────────────────────────────
#[cfg(any(test, feature = "test-helpers"))]
pub use role::InMemoryRoleMembership;
pub use role::{NoopRoleMembership, RoleId, RoleMembershipRepository};

// ── memory ────────────────────────────────────────────────────────────────────
#[cfg(any(test, feature = "test-helpers"))]
pub use memory::InMemorySecurityPolicyRepository;

// ── auth_level ────────────────────────────────────────────────────────────────
pub use auth_level::AuthLevel;

// ── classification ────────────────────────────────────────────────────────────
pub use classification::ResourceClassification;

// ── context ───────────────────────────────────────────────────────────────────
pub use context::{OrgScope, SecurityContext, SessionRef};

// ── authz ─────────────────────────────────────────────────────────────────────
pub use authz::{AuthzDecision, AuthzOutcome, AuthzRequest, EvidenceLevel, ResourceAttributes};

// ── event ─────────────────────────────────────────────────────────────────────
#[cfg(any(test, feature = "test-helpers"))]
pub use event::InMemorySecurityEventPublisher;
pub use event::{
    NoopSecurityEventPublisher, SecurityEvent, SecurityEventKind, SecurityEventPublisher,
};

// ── sod ───────────────────────────────────────────────────────────────────────
pub use sod::{check_sod, SodRule, SodViolation};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{
    NoopOrgScopeValidator, NoopSodHistoryProvider, OrgScopeValidator, SodHistoryProvider,
};

// ── delegation conditions ─────────────────────────────────────────────────────
pub use delegation::DelegationCondition;

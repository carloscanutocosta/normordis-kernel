//! Domínio de estrutura orgânica institucional do Mini-Kernel RS.
//!
//! Cobre a hierarquia de unidades orgânicas, cargos, competências, delegações
//! e instrumentos jurídicos que as fundamentam. Exporta tipos, invariantes,
//! ports de persistência, camada de serviço, porto de auditoria e porto de
//! eventos de domínio. Não conhece SQLite, filesystem, Tauri ou UI.

#[cfg(test)]
mod tests;

pub mod audit;
pub mod competency;
pub mod controls;
pub mod delegation;
pub mod domain_events;
pub mod drainer;
pub mod error;
pub mod instrument;
pub mod pagination;
pub mod ports;
pub mod position;
pub mod service;
pub mod unit;

// ── audit ─────────────────────────────────────────────────────────────────────
pub use audit::{OrgAuditAction, OrgAuditEvent, OrgAuditPort, OrgEventOutcome, OrgNoopAudit};

// ── competency ────────────────────────────────────────────────────────────────
pub use competency::{Competency, CompetencyId};

// ── delegation ────────────────────────────────────────────────────────────────
pub use delegation::{Delegation, DelegationId};

// ── domain_events ─────────────────────────────────────────────────────────────
pub use domain_events::{OrgDomainEvent, OrgDomainEventPort, OrgNoopDomainEvents};

// ── drainer ───────────────────────────────────────────────────────────────────
pub use drainer::{DrainStats, OrgOutboxDrainer};

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{OrgError, COMPONENT};

// ── instrument ────────────────────────────────────────────────────────────────
pub use instrument::{InstrumentKind, LegalInstrument, LegalInstrumentId};

// ── pagination ────────────────────────────────────────────────────────────────
pub use pagination::{OrgPage, PagedResult};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{
    CompetencyRepository, DelegationRepository, LegalInstrumentRepository, OrgAuditOutbox,
    OrgPositionRepository, OrgUnitRepository,
};

// ── position ──────────────────────────────────────────────────────────────────
pub use position::{OrgPosition, OrgPositionId, OrgPositionStatus, PositionKind};

// ── service ───────────────────────────────────────────────────────────────────
pub use service::{CompetencyService, DelegationService, OrgPositionService, OrgUnitService};

// ── unit ──────────────────────────────────────────────────────────────────────
pub use unit::{OrgAddress, OrgContacts, OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus};

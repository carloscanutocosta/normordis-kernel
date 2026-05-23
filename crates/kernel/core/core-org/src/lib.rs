//! Domínio de estrutura orgânica institucional do Mini-Kernel RS.
//!
//! Cobre a hierarquia de unidades orgânicas, cargos, competências, delegações
//! e instrumentos jurídicos que as fundamentam. Exporta tipos, invariantes e
//! ports de persistência. Não conhece SQLite, filesystem, Tauri ou UI.

#[cfg(test)]
mod tests;

pub mod competency;
pub mod delegation;
pub mod error;
pub mod instrument;
pub mod ports;
pub mod position;
pub mod unit;

// ── competency ────────────────────────────────────────────────────────────────
pub use competency::{Competency, CompetencyId};

// ── delegation ────────────────────────────────────────────────────────────────
pub use delegation::{Delegation, DelegationId};

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{OrgError, COMPONENT};

// ── instrument ────────────────────────────────────────────────────────────────
pub use instrument::{InstrumentKind, LegalInstrument, LegalInstrumentId};

// ── ports ─────────────────────────────────────────────────────────────────────
pub use ports::{
    CompetencyRepository, DelegationRepository, LegalInstrumentRepository, OrgPositionRepository,
    OrgUnitRepository,
};

// ── position ──────────────────────────────────────────────────────────────────
pub use position::{OrgPosition, OrgPositionId};

// ── unit ──────────────────────────────────────────────────────────────────────
pub use unit::{OrgAddress, OrgContacts, OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus};

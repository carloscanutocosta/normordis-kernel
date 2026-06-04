//! Porto de eventos de domínio de `core-org` (driven/secondary port).
//!
//! Permite que outros bounded contexts (ex.: `core-rh`) sejam notificados de
//! alterações estruturais sem acoplamento directo entre domínios.
//! `core-org` define o contrato; a implementação concreta vive na camada de
//! aplicação ou num adaptador de mensageria.

use serde::{Deserialize, Serialize};

use crate::{
    OrgError, OrgLevel, OrgPositionId, OrgPositionStatus, OrgUnitId, OrgUnitStatus, PositionKind,
};

// ── Eventos ───────────────────────────────────────────────────────────────────

/// Serializável para viajar no outbox de eventos de domínio (captura atómica com
/// o estado, entrega fiável a outros bounded contexts — ver `org-sqlite`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrgDomainEvent {
    /// Uma unidade orgânica foi criada.
    UnitCreated {
        id: OrgUnitId,
        short_name: String,
        level: OrgLevel,
    },
    /// Uma unidade orgânica foi importada (modo histórico).
    UnitImported {
        id: OrgUnitId,
        short_name: String,
        level: OrgLevel,
    },
    /// Os dados de uma unidade orgânica foram actualizados.
    UnitUpdated { id: OrgUnitId },
    /// Uma unidade orgânica foi desactivada (extinta).
    UnitDeactivated { id: OrgUnitId },
    /// O estado de uma unidade orgânica foi alterado.
    UnitStatusChanged {
        id: OrgUnitId,
        new_status: OrgUnitStatus,
    },
    /// Uma posição orgânica foi criada.
    PositionCreated {
        id: OrgPositionId,
        unit_id: OrgUnitId,
        kind: PositionKind,
        title: String,
    },
    /// Os dados de uma posição orgânica foram actualizados.
    PositionUpdated { id: OrgPositionId },
    /// Uma posição orgânica foi desactivada (extinta).
    PositionDeactivated { id: OrgPositionId },
    /// O estado de uma posição orgânica foi alterado.
    PositionStatusChanged {
        id: OrgPositionId,
        new_status: OrgPositionStatus,
    },
}

// ── Porto ─────────────────────────────────────────────────────────────────────

/// Porto de publicação de eventos de domínio.
/// Implementado pela camada de aplicação (ex.: barramento de eventos, canal Tauri, log).
pub trait OrgDomainEventPort {
    fn publish(&self, event: OrgDomainEvent) -> Result<(), OrgError>;
}

// ── Implementação nula ────────────────────────────────────────────────────────

/// Implementação nula — descarta todos os eventos. Usar em testes e contextos
/// onde a integração entre bounded contexts ainda não está configurada.
pub struct OrgNoopDomainEvents;

impl OrgDomainEventPort for OrgNoopDomainEvents {
    fn publish(&self, _event: OrgDomainEvent) -> Result<(), OrgError> {
        Ok(())
    }
}

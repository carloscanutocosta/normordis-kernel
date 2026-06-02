//! Camada de serviço de `core-org`.
//!
//! Orquestra invariantes de domínio, emite eventos de auditoria (com payload)
//! e publica eventos de domínio para integração com outros bounded contexts
//! (ex.: `core-rh` precisa de saber quando uma `OrgPosition` é criada).
//!
//! Cada serviço aceita três parâmetros genéricos:
//! - `R` — repositório (port de persistência)
//! - `A` — porto de auditoria (`OrgAuditPort`)
//! - `E` — porto de eventos de domínio (`OrgDomainEventPort`)
//!
//! Em contextos sem auditoria ou eventos reais, usar `OrgNoopAudit` e
//! `OrgNoopDomainEvents` respectivamente.

use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};

use crate::{
    audit::{OrgAuditAction, OrgAuditEvent, OrgAuditPort},
    domain_events::{OrgDomainEvent, OrgDomainEventPort},
    error::OrgError,
    ports::{OrgPositionRepository, OrgUnitRepository},
    position::{OrgPosition, OrgPositionId, OrgPositionStatus},
    unit::{OrgUnit, OrgUnitId, OrgUnitStatus},
};

// ── OrgUnitService ────────────────────────────────────────────────────────────

/// Serviço de unidades orgânicas.
/// Garante todas as invariantes antes de persistir, emite audit com payload
/// e publica eventos de domínio.
pub struct OrgUnitService<R, A, E>
where
    R: OrgUnitRepository,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub repo: R,
    pub audit: A,
    pub events: E,
}

impl<R: OrgUnitRepository, A: OrgAuditPort, E: OrgDomainEventPort> OrgUnitService<R, A, E> {
    pub fn new(repo: R, audit: A, events: E) -> Self {
        Self {
            repo,
            audit,
            events,
        }
    }

    /// Cria uma unidade orgânica em modo operacional.
    /// Exige `created_by` ou `legal_reference`. Valida hierarquia e contactos.
    pub fn create(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        unit.validate_strict()?;
        self.enforce_parent_invariants(&unit)?;
        self.repo.create(&unit)?;
        let payload = unit_payload(&unit);
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Created,
            "OrgUnit",
            unit.id.as_str(),
            Utc::now(),
            Some(payload),
        ))?;
        self.events.publish(OrgDomainEvent::UnitCreated {
            id: unit.id.clone(),
            short_name: unit.short_name.clone(),
            level: unit.level,
        })?;
        Ok(())
    }

    /// Importa uma unidade de dados históricos (sem obrigatoriedade de instrumento).
    pub fn import(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        unit.validate()?;
        if let Some(ref parent_id) = unit.parent_id {
            let parent = self
                .repo
                .get(parent_id)?
                .ok_or_else(|| OrgError::UnitNotFound(parent_id.as_str().into()))?;
            unit.validate_level_against_parent(&parent)?;
        }
        self.repo.save(&unit)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Imported,
            "OrgUnit",
            unit.id.as_str(),
            Utc::now(),
            Some(unit_payload(&unit)),
        ))?;
        self.events.publish(OrgDomainEvent::UnitImported {
            id: unit.id.clone(),
            short_name: unit.short_name.clone(),
            level: unit.level,
        })?;
        Ok(())
    }

    /// Actualiza uma unidade orgânica com OCC. Valida hierarquia e contactos.
    pub fn update(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        unit.validate_strict()?;
        self.enforce_parent_invariants(&unit)?;
        self.repo.update(&unit)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Updated,
            "OrgUnit",
            unit.id.as_str(),
            Utc::now(),
            Some(unit_payload(&unit)),
        ))?;
        self.events.publish(OrgDomainEvent::UnitUpdated {
            id: unit.id.clone(),
        })?;
        Ok(())
    }

    /// Desactiva a unidade via máquina de estados do domínio.
    pub fn deactivate(
        &self,
        id: &OrgUnitId,
        valid_until: NaiveDate,
        actor: &str,
    ) -> Result<(), OrgError> {
        let unit = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::UnitNotFound(id.as_str().into()))?;
        unit.transition_status(OrgUnitStatus::Extinct)?;
        self.repo.deactivate(id, valid_until)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Deactivated,
            "OrgUnit",
            id.as_str(),
            Utc::now(),
            Some(json!({ "valid_until": valid_until.to_string() })),
        ))?;
        self.events
            .publish(OrgDomainEvent::UnitDeactivated { id: id.clone() })?;
        Ok(())
    }

    /// Suspende a unidade (Active → Suspended).
    pub fn suspend(&self, id: &OrgUnitId, actor: &str) -> Result<(), OrgError> {
        let unit = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::UnitNotFound(id.as_str().into()))?;
        unit.transition_status(OrgUnitStatus::Suspended)?;
        let updated = OrgUnit {
            status: OrgUnitStatus::Suspended,
            ..unit
        };
        self.repo.update(&updated)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::StatusChanged {
                from: "active",
                to: "suspended",
            },
            "OrgUnit",
            id.as_str(),
            Utc::now(),
            Some(json!({ "status": "suspended" })),
        ))?;
        self.events.publish(OrgDomainEvent::UnitStatusChanged {
            id: id.clone(),
            new_status: OrgUnitStatus::Suspended,
        })?;
        Ok(())
    }

    /// Reactiva a unidade (Suspended → Active).
    pub fn reactivate(&self, id: &OrgUnitId, actor: &str) -> Result<(), OrgError> {
        let unit = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::UnitNotFound(id.as_str().into()))?;
        unit.transition_status(OrgUnitStatus::Active)?;
        let updated = OrgUnit {
            status: OrgUnitStatus::Active,
            ..unit
        };
        self.repo.update(&updated)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::StatusChanged {
                from: "suspended",
                to: "active",
            },
            "OrgUnit",
            id.as_str(),
            Utc::now(),
            Some(json!({ "status": "active" })),
        ))?;
        self.events.publish(OrgDomainEvent::UnitStatusChanged {
            id: id.clone(),
            new_status: OrgUnitStatus::Active,
        })?;
        Ok(())
    }

    // ── Invariantes privadas ──────────────────────────────────────────────────

    fn enforce_parent_invariants(&self, unit: &OrgUnit) -> Result<(), OrgError> {
        let Some(ref parent_id) = unit.parent_id else {
            return Ok(());
        };
        let parent = self
            .repo
            .get(parent_id)?
            .ok_or_else(|| OrgError::UnitNotFound(parent_id.as_str().into()))?;
        unit.validate_level_against_parent(&parent)?;
        let ancestors = self.repo.hierarchy_at(parent_id, unit.valid_from)?;
        let ancestor_ids: Vec<&OrgUnitId> = ancestors.iter().map(|u| &u.id).collect();
        unit.validate_parent_chain(&ancestor_ids)?;
        Ok(())
    }
}

// ── OrgPositionService ────────────────────────────────────────────────────────

/// Serviço de posições orgânicas.
/// Garante invariantes, detecta ciclos de substituição, emite audit com payload
/// e publica eventos de domínio.
pub struct OrgPositionService<R, A, E>
where
    R: OrgPositionRepository,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub repo: R,
    pub audit: A,
    pub events: E,
}

impl<R: OrgPositionRepository, A: OrgAuditPort, E: OrgDomainEventPort> OrgPositionService<R, A, E> {
    pub fn new(repo: R, audit: A, events: E) -> Self {
        Self {
            repo,
            audit,
            events,
        }
    }

    /// Cria uma posição orgânica validando ciclos de substituição.
    pub fn create(&self, position: OrgPosition, actor: &str) -> Result<(), OrgError> {
        position.validate()?;
        self.enforce_substitute_invariants(&position)?;
        self.repo.create(&position)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Created,
            "OrgPosition",
            position.id.as_str(),
            Utc::now(),
            Some(position_payload(&position)),
        ))?;
        self.events.publish(OrgDomainEvent::PositionCreated {
            id: position.id.clone(),
            unit_id: position.unit_id.clone(),
            kind: position.kind.clone(),
            title: position.title.clone(),
        })?;
        Ok(())
    }

    /// Actualiza uma posição com OCC e re-valida ciclos de substituição.
    pub fn update(&self, position: OrgPosition, actor: &str) -> Result<(), OrgError> {
        position.validate()?;
        self.enforce_substitute_invariants(&position)?;
        self.repo.update(&position)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Updated,
            "OrgPosition",
            position.id.as_str(),
            Utc::now(),
            Some(position_payload(&position)),
        ))?;
        self.events.publish(OrgDomainEvent::PositionUpdated {
            id: position.id.clone(),
        })?;
        Ok(())
    }

    /// Desactiva a posição via máquina de estados do domínio.
    pub fn deactivate(
        &self,
        id: &OrgPositionId,
        valid_until: NaiveDate,
        actor: &str,
    ) -> Result<(), OrgError> {
        let pos = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::PositionNotFound(id.as_str().into()))?;
        pos.transition_status(OrgPositionStatus::Extinct)?;
        self.repo.deactivate(id, valid_until)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::Deactivated,
            "OrgPosition",
            id.as_str(),
            Utc::now(),
            Some(json!({ "valid_until": valid_until.to_string() })),
        ))?;
        self.events
            .publish(OrgDomainEvent::PositionDeactivated { id: id.clone() })?;
        Ok(())
    }

    /// Suspende a posição (Active → Suspended).
    pub fn suspend(&self, id: &OrgPositionId, actor: &str) -> Result<(), OrgError> {
        let pos = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::PositionNotFound(id.as_str().into()))?;
        pos.transition_status(OrgPositionStatus::Suspended)?;
        let updated = OrgPosition {
            status: OrgPositionStatus::Suspended,
            ..pos
        };
        self.repo.update(&updated)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::StatusChanged {
                from: "active",
                to: "suspended",
            },
            "OrgPosition",
            id.as_str(),
            Utc::now(),
            Some(json!({ "status": "suspended" })),
        ))?;
        self.events.publish(OrgDomainEvent::PositionStatusChanged {
            id: id.clone(),
            new_status: OrgPositionStatus::Suspended,
        })?;
        Ok(())
    }

    /// Reactiva a posição (Suspended → Active).
    pub fn reactivate(&self, id: &OrgPositionId, actor: &str) -> Result<(), OrgError> {
        let pos = self
            .repo
            .get(id)?
            .ok_or_else(|| OrgError::PositionNotFound(id.as_str().into()))?;
        pos.transition_status(OrgPositionStatus::Active)?;
        let updated = OrgPosition {
            status: OrgPositionStatus::Active,
            ..pos
        };
        self.repo.update(&updated)?;
        self.audit.record(OrgAuditEvent::new(
            actor,
            OrgAuditAction::StatusChanged {
                from: "suspended",
                to: "active",
            },
            "OrgPosition",
            id.as_str(),
            Utc::now(),
            Some(json!({ "status": "active" })),
        ))?;
        self.events.publish(OrgDomainEvent::PositionStatusChanged {
            id: id.clone(),
            new_status: OrgPositionStatus::Active,
        })?;
        Ok(())
    }

    // ── Invariantes privadas ──────────────────────────────────────────────────

    fn enforce_substitute_invariants(&self, position: &OrgPosition) -> Result<(), OrgError> {
        let Some(ref target_id) = position.substitutes else {
            return Ok(());
        };
        let target = self
            .repo
            .get(target_id)?
            .ok_or_else(|| OrgError::PositionNotFound(target_id.as_str().into()))?;
        let mut current = target;
        for _ in 0..50 {
            let Some(ref next_id) = current.substitutes.clone() else {
                break;
            };
            if next_id == &position.id {
                return Err(OrgError::SubstitutionCycle);
            }
            current = self
                .repo
                .get(next_id)?
                .ok_or_else(|| OrgError::PositionNotFound(next_id.as_str().into()))?;
        }
        Ok(())
    }
}

// ── Helpers de payload ────────────────────────────────────────────────────────

fn unit_payload(u: &OrgUnit) -> Value {
    json!({
        "id": u.id.as_str(),
        "short_name": u.short_name,
        "level": u.level.as_u8(),
        "status": u.status.as_str(),
        "version": u.version,
    })
}

fn position_payload(p: &OrgPosition) -> Value {
    json!({
        "id": p.id.as_str(),
        "code": p.code,
        "title": p.title,
        "kind": p.kind.as_str(),
        "unit_id": p.unit_id.as_str(),
        "status": p.status.as_str(),
        "version": p.version,
    })
}

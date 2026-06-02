//! Camada de serviço de `core-org`.
//!
//! Ponto de entrada único para escrita governada. Cada operação:
//! 1. valida invariantes (puro);
//! 2. persiste o estado **e** captura a evidência (`OrgAuditEvent`) e o evento de
//!    domínio (`OrgDomainEvent`) nos outboxes, tudo na **mesma transação**
//!    (variantes `*_audited` das repos);
//! 3. entrega evidência e eventos aos respectivos portos (drain idempotente,
//!    resiliente a falhas e a poison messages).
//!
//! ## Evidência COSO
//! - **Sucesso e falha** são evidenciados: uma operação rejeitada (validação,
//!   `VersionConflict`, guarda de extinção) emite um evento `Failure` antes de
//!   propagar o erro.
//! - Cada operação refere o `control_id` COSO primário (módulo [`controls`]).
//! - A captura de evidência **e dos eventos de domínio** é atómica com o estado
//!   (outboxes no mesmo `org.db`); a entrega é idempotente e sem perda.
//!
//! [`controls`]: crate::controls

use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};

use crate::{
    audit::{OrgAuditAction, OrgAuditEvent, OrgAuditPort, OrgEventOutcome},
    competency::Competency,
    controls,
    delegation::Delegation,
    domain_events::{OrgDomainEvent, OrgDomainEventPort},
    error::OrgError,
    ports::{
        CompetencyRepository, DelegationRepository, OrgAuditOutbox, OrgPositionRepository,
        OrgUnitRepository,
    },
    position::{OrgPosition, OrgPositionId, OrgPositionStatus},
    unit::{OrgUnit, OrgUnitId, OrgUnitStatus},
};

// ── Helpers de evento ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn event(
    actor: &str,
    action: OrgAuditAction,
    kind: &str,
    id: &str,
    outcome: OrgEventOutcome,
    control_id: &str,
    payload: Value,
) -> OrgAuditEvent {
    OrgAuditEvent::new(
        actor,
        action,
        kind,
        id,
        Utc::now(),
        outcome,
        Some(control_id.to_string()),
        Some(payload),
    )
}

fn failure_payload(err: &OrgError) -> Value {
    json!({ "error_code": err.code(), "error": err.to_string() })
}

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

fn competency_payload(c: &Competency) -> Value {
    json!({
        "id": c.id.as_str(),
        "code": c.code,
        "assigned_to": c.assigned_to.as_str(),
        "version": c.version,
    })
}

fn delegation_payload(d: &Delegation) -> Value {
    json!({
        "id": d.id.as_str(),
        "competency_id": d.competency_id.as_str(),
        "from": d.from_position.as_str(),
        "to": d.to_position.as_str(),
        "version": d.version,
    })
}

// ── OrgUnitService ────────────────────────────────────────────────────────────

pub struct OrgUnitService<R, A, E>
where
    R: OrgUnitRepository + OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub repo: R,
    pub audit: A,
    pub events: E,
}

impl<R, A, E> OrgUnitService<R, A, E>
where
    R: OrgUnitRepository + OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub fn new(repo: R, audit: A, events: E) -> Self {
        Self {
            repo,
            audit,
            events,
        }
    }

    /// Entrega evidência e eventos pendentes. Best-effort: o que não for entregue
    /// permanece no outbox (sem perda) para uma drenagem posterior.
    fn deliver(&self) {
        let _ = self.repo.drain_audit_outbox(&self.audit);
        let _ = self.repo.drain_domain_outbox(&self.events);
    }

    fn record_failure(
        &self,
        actor: &str,
        action: OrgAuditAction,
        id: &str,
        control_id: &str,
        err: &OrgError,
    ) {
        let ev = event(
            actor,
            action,
            "OrgUnit",
            id,
            OrgEventOutcome::Failure,
            control_id,
            failure_payload(err),
        );
        let _ = self.repo.enqueue_audit(&ev);
        self.deliver();
    }

    /// Drena manualmente ambos os outboxes (para jobs de fundo). Devolve
    /// `(auditoria_entregue, eventos_entregues)`.
    pub fn drain(&self) -> Result<(usize, usize), OrgError> {
        let a = self.repo.drain_audit_outbox(&self.audit)?;
        let e = self.repo.drain_domain_outbox(&self.events)?;
        Ok((a, e))
    }

    pub fn pending_audit(&self) -> Result<u64, OrgError> {
        self.repo.pending_audit_count()
    }

    /// Cria uma unidade orgânica em modo operacional.
    pub fn create(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_UNIT_CHANGE;
        if let Err(e) = unit
            .validate_strict()
            .and_then(|_| self.enforce_parent_invariants(&unit))
        {
            self.record_failure(actor, OrgAuditAction::Created, unit.id.as_str(), ctrl, &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Created,
            "OrgUnit",
            unit.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            unit_payload(&unit),
        );
        let domain = OrgDomainEvent::UnitCreated {
            id: unit.id.clone(),
            short_name: unit.short_name.clone(),
            level: unit.level,
        };
        if let Err(e) = self.repo.create_audited(&unit, &ev, Some(&domain)) {
            self.record_failure(actor, OrgAuditAction::Created, unit.id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    /// Importa uma unidade de dados históricos (sem obrigatoriedade de instrumento).
    pub fn import(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_UNIT_CHANGE;
        let check = unit.validate().and_then(|_| {
            if let Some(ref parent_id) = unit.parent_id {
                let parent = self
                    .repo
                    .get(parent_id)?
                    .ok_or_else(|| OrgError::UnitNotFound(parent_id.as_str().into()))?;
                unit.validate_level_against_parent(&parent)?;
            }
            Ok(())
        });
        if let Err(e) = check {
            self.record_failure(actor, OrgAuditAction::Imported, unit.id.as_str(), ctrl, &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Imported,
            "OrgUnit",
            unit.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            unit_payload(&unit),
        );
        let domain = OrgDomainEvent::UnitImported {
            id: unit.id.clone(),
            short_name: unit.short_name.clone(),
            level: unit.level,
        };
        self.repo.save_audited(&unit, &ev, Some(&domain))?;
        self.deliver();
        Ok(())
    }

    /// Actualiza uma unidade orgânica com OCC.
    pub fn update(&self, unit: OrgUnit, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_UNIT_CHANGE;
        if let Err(e) = unit
            .validate_strict()
            .and_then(|_| self.enforce_parent_invariants(&unit))
        {
            self.record_failure(actor, OrgAuditAction::Updated, unit.id.as_str(), ctrl, &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Updated,
            "OrgUnit",
            unit.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            unit_payload(&unit),
        );
        let domain = OrgDomainEvent::UnitUpdated {
            id: unit.id.clone(),
        };
        if let Err(e) = self.repo.update_audited(&unit, &ev, Some(&domain)) {
            self.record_failure(actor, OrgAuditAction::Updated, unit.id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    /// Desactiva a unidade via máquina de estados do domínio.
    pub fn deactivate(
        &self,
        id: &OrgUnitId,
        valid_until: NaiveDate,
        actor: &str,
    ) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_UNIT_LIFECYCLE;
        let check = (|| {
            let unit = self
                .repo
                .get(id)?
                .ok_or_else(|| OrgError::UnitNotFound(id.as_str().into()))?;
            unit.transition_status(OrgUnitStatus::Extinct)?;
            Ok(())
        })();
        if let Err(e) = check {
            self.record_failure(actor, OrgAuditAction::Deactivated, id.as_str(), ctrl, &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Deactivated,
            "OrgUnit",
            id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            json!({ "valid_until": valid_until.to_string() }),
        );
        let domain = OrgDomainEvent::UnitDeactivated { id: id.clone() };
        if let Err(e) = self
            .repo
            .deactivate_audited(id, valid_until, &ev, Some(&domain))
        {
            self.record_failure(actor, OrgAuditAction::Deactivated, id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    /// Suspende a unidade (Active → Suspended).
    pub fn suspend(&self, id: &OrgUnitId, actor: &str) -> Result<(), OrgError> {
        self.transition(id, actor, OrgUnitStatus::Suspended, "active", "suspended")
    }

    /// Reactiva a unidade (Suspended → Active).
    pub fn reactivate(&self, id: &OrgUnitId, actor: &str) -> Result<(), OrgError> {
        self.transition(id, actor, OrgUnitStatus::Active, "suspended", "active")
    }

    fn transition(
        &self,
        id: &OrgUnitId,
        actor: &str,
        next: OrgUnitStatus,
        from: &str,
        to: &str,
    ) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_UNIT_LIFECYCLE;
        let action = OrgAuditAction::StatusChanged {
            from: from.into(),
            to: to.into(),
        };
        let prepared = (|| {
            let unit = self
                .repo
                .get(id)?
                .ok_or_else(|| OrgError::UnitNotFound(id.as_str().into()))?;
            unit.transition_status(next.clone())?;
            Ok(OrgUnit {
                status: next.clone(),
                ..unit
            })
        })();
        let updated = match prepared {
            Ok(u) => u,
            Err(e) => {
                self.record_failure(actor, action, id.as_str(), ctrl, &e);
                return Err(e);
            }
        };
        let ev = event(
            actor,
            action.clone(),
            "OrgUnit",
            id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            json!({ "status": to }),
        );
        let domain = OrgDomainEvent::UnitStatusChanged {
            id: id.clone(),
            new_status: next,
        };
        if let Err(e) = self.repo.update_audited(&updated, &ev, Some(&domain)) {
            self.record_failure(actor, action, id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

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

pub struct OrgPositionService<R, A, E>
where
    R: OrgPositionRepository + OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub repo: R,
    pub audit: A,
    pub events: E,
}

impl<R, A, E> OrgPositionService<R, A, E>
where
    R: OrgPositionRepository + OrgAuditOutbox,
    A: OrgAuditPort,
    E: OrgDomainEventPort,
{
    pub fn new(repo: R, audit: A, events: E) -> Self {
        Self {
            repo,
            audit,
            events,
        }
    }

    fn deliver(&self) {
        let _ = self.repo.drain_audit_outbox(&self.audit);
        let _ = self.repo.drain_domain_outbox(&self.events);
    }

    fn record_failure(
        &self,
        actor: &str,
        action: OrgAuditAction,
        id: &str,
        control_id: &str,
        err: &OrgError,
    ) {
        let ev = event(
            actor,
            action,
            "OrgPosition",
            id,
            OrgEventOutcome::Failure,
            control_id,
            failure_payload(err),
        );
        let _ = self.repo.enqueue_audit(&ev);
        self.deliver();
    }

    pub fn drain(&self) -> Result<(usize, usize), OrgError> {
        let a = self.repo.drain_audit_outbox(&self.audit)?;
        let e = self.repo.drain_domain_outbox(&self.events)?;
        Ok((a, e))
    }

    pub fn create(&self, position: OrgPosition, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_POSITION_CHANGE;
        if let Err(e) = position
            .validate()
            .and_then(|_| self.enforce_substitute_invariants(&position))
        {
            self.record_failure(
                actor,
                OrgAuditAction::Created,
                position.id.as_str(),
                ctrl,
                &e,
            );
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Created,
            "OrgPosition",
            position.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            position_payload(&position),
        );
        let domain = OrgDomainEvent::PositionCreated {
            id: position.id.clone(),
            unit_id: position.unit_id.clone(),
            kind: position.kind.clone(),
            title: position.title.clone(),
        };
        if let Err(e) = self.repo.create_audited(&position, &ev, Some(&domain)) {
            self.record_failure(
                actor,
                OrgAuditAction::Created,
                position.id.as_str(),
                ctrl,
                &e,
            );
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn update(&self, position: OrgPosition, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_POSITION_CHANGE;
        if let Err(e) = position
            .validate()
            .and_then(|_| self.enforce_substitute_invariants(&position))
        {
            self.record_failure(
                actor,
                OrgAuditAction::Updated,
                position.id.as_str(),
                ctrl,
                &e,
            );
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Updated,
            "OrgPosition",
            position.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            position_payload(&position),
        );
        let domain = OrgDomainEvent::PositionUpdated {
            id: position.id.clone(),
        };
        if let Err(e) = self.repo.update_audited(&position, &ev, Some(&domain)) {
            self.record_failure(
                actor,
                OrgAuditAction::Updated,
                position.id.as_str(),
                ctrl,
                &e,
            );
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn deactivate(
        &self,
        id: &OrgPositionId,
        valid_until: NaiveDate,
        actor: &str,
    ) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_POSITION_LIFECYCLE;
        let check = (|| {
            let pos = self
                .repo
                .get(id)?
                .ok_or_else(|| OrgError::PositionNotFound(id.as_str().into()))?;
            pos.transition_status(OrgPositionStatus::Extinct)?;
            Ok(())
        })();
        if let Err(e) = check {
            self.record_failure(actor, OrgAuditAction::Deactivated, id.as_str(), ctrl, &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Deactivated,
            "OrgPosition",
            id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            json!({ "valid_until": valid_until.to_string() }),
        );
        let domain = OrgDomainEvent::PositionDeactivated { id: id.clone() };
        if let Err(e) = self
            .repo
            .deactivate_audited(id, valid_until, &ev, Some(&domain))
        {
            self.record_failure(actor, OrgAuditAction::Deactivated, id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn suspend(&self, id: &OrgPositionId, actor: &str) -> Result<(), OrgError> {
        self.transition(
            id,
            actor,
            OrgPositionStatus::Suspended,
            "active",
            "suspended",
        )
    }

    pub fn reactivate(&self, id: &OrgPositionId, actor: &str) -> Result<(), OrgError> {
        self.transition(id, actor, OrgPositionStatus::Active, "suspended", "active")
    }

    fn transition(
        &self,
        id: &OrgPositionId,
        actor: &str,
        next: OrgPositionStatus,
        from: &str,
        to: &str,
    ) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_POSITION_LIFECYCLE;
        let action = OrgAuditAction::StatusChanged {
            from: from.into(),
            to: to.into(),
        };
        let prepared = (|| {
            let pos = self
                .repo
                .get(id)?
                .ok_or_else(|| OrgError::PositionNotFound(id.as_str().into()))?;
            pos.transition_status(next.clone())?;
            Ok(OrgPosition {
                status: next.clone(),
                ..pos
            })
        })();
        let updated = match prepared {
            Ok(p) => p,
            Err(e) => {
                self.record_failure(actor, action, id.as_str(), ctrl, &e);
                return Err(e);
            }
        };
        let ev = event(
            actor,
            action.clone(),
            "OrgPosition",
            id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            json!({ "status": to }),
        );
        let domain = OrgDomainEvent::PositionStatusChanged {
            id: id.clone(),
            new_status: next,
        };
        if let Err(e) = self.repo.update_audited(&updated, &ev, Some(&domain)) {
            self.record_failure(actor, action, id.as_str(), ctrl, &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

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

// ── CompetencyService ─────────────────────────────────────────────────────────

pub struct CompetencyService<R, A>
where
    R: CompetencyRepository + OrgAuditOutbox,
    A: OrgAuditPort,
{
    pub repo: R,
    pub audit: A,
}

impl<R, A> CompetencyService<R, A>
where
    R: CompetencyRepository + OrgAuditOutbox,
    A: OrgAuditPort,
{
    pub fn new(repo: R, audit: A) -> Self {
        Self { repo, audit }
    }

    fn deliver(&self) {
        let _ = self.repo.drain_audit_outbox(&self.audit);
    }

    fn record_failure(&self, actor: &str, action: OrgAuditAction, id: &str, err: &OrgError) {
        let ev = event(
            actor,
            action,
            "Competency",
            id,
            OrgEventOutcome::Failure,
            controls::CTRL_ORG_COMPETENCY,
            failure_payload(err),
        );
        let _ = self.repo.enqueue_audit(&ev);
        self.deliver();
    }

    pub fn create(&self, competency: Competency, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_COMPETENCY;
        if let Err(e) = competency.validate() {
            self.record_failure(actor, OrgAuditAction::Created, competency.id.as_str(), &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Created,
            "Competency",
            competency.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            competency_payload(&competency),
        );
        if let Err(e) = self.repo.create_audited(&competency, &ev) {
            self.record_failure(actor, OrgAuditAction::Created, competency.id.as_str(), &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn update(&self, competency: Competency, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_COMPETENCY;
        if let Err(e) = competency.validate() {
            self.record_failure(actor, OrgAuditAction::Updated, competency.id.as_str(), &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Updated,
            "Competency",
            competency.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            competency_payload(&competency),
        );
        if let Err(e) = self.repo.update_audited(&competency, &ev) {
            self.record_failure(actor, OrgAuditAction::Updated, competency.id.as_str(), &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn drain_audit(&self) -> Result<usize, OrgError> {
        self.repo.drain_audit_outbox(&self.audit)
    }
}

// ── DelegationService ─────────────────────────────────────────────────────────

pub struct DelegationService<R, A>
where
    R: DelegationRepository + OrgAuditOutbox,
    A: OrgAuditPort,
{
    pub repo: R,
    pub audit: A,
}

impl<R, A> DelegationService<R, A>
where
    R: DelegationRepository + OrgAuditOutbox,
    A: OrgAuditPort,
{
    pub fn new(repo: R, audit: A) -> Self {
        Self { repo, audit }
    }

    fn deliver(&self) {
        let _ = self.repo.drain_audit_outbox(&self.audit);
    }

    fn record_failure(&self, actor: &str, action: OrgAuditAction, id: &str, err: &OrgError) {
        let ev = event(
            actor,
            action,
            "Delegation",
            id,
            OrgEventOutcome::Failure,
            controls::CTRL_ORG_DELEGATION,
            failure_payload(err),
        );
        let _ = self.repo.enqueue_audit(&ev);
        self.deliver();
    }

    /// Cria uma delegação. `from_competencies` são as competências activas de
    /// `from_position` na data — usadas para verificar que o delegante detém a
    /// competência que delega.
    pub fn create(
        &self,
        delegation: Delegation,
        from_competencies: &[&crate::CompetencyId],
        actor: &str,
    ) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_DELEGATION;
        if let Err(e) = delegation
            .validate()
            .and_then(|_| delegation.validate_can_delegate(from_competencies))
        {
            self.record_failure(actor, OrgAuditAction::Created, delegation.id.as_str(), &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Created,
            "Delegation",
            delegation.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            delegation_payload(&delegation),
        );
        if let Err(e) = self.repo.create_audited(&delegation, &ev) {
            self.record_failure(actor, OrgAuditAction::Created, delegation.id.as_str(), &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn update(&self, delegation: Delegation, actor: &str) -> Result<(), OrgError> {
        let ctrl = controls::CTRL_ORG_DELEGATION;
        if let Err(e) = delegation.validate() {
            self.record_failure(actor, OrgAuditAction::Updated, delegation.id.as_str(), &e);
            return Err(e);
        }
        let ev = event(
            actor,
            OrgAuditAction::Updated,
            "Delegation",
            delegation.id.as_str(),
            OrgEventOutcome::Success,
            ctrl,
            delegation_payload(&delegation),
        );
        if let Err(e) = self.repo.update_audited(&delegation, &ev) {
            self.record_failure(actor, OrgAuditAction::Updated, delegation.id.as_str(), &e);
            return Err(e);
        }
        self.deliver();
        Ok(())
    }

    pub fn drain_audit(&self) -> Result<usize, OrgError> {
        self.repo.drain_audit_outbox(&self.audit)
    }
}

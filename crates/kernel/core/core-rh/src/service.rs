//! Serviços de domínio de `core-rh` — ponto de entrada único para escrita governada.
//!
//! Garante os invariantes de domínio antes de persistir e captura evidência COSO
//! atomicamente com o estado (via variantes `_audited` do repositório):
//! - a afetação é válida (campos obrigatórios, intervalo temporal);
//! - não existe sobreposição temporal com outra afetação da mesma pessoa;
//! - o encerramento respeita a ordem temporal (`valid_until > valid_from`);
//! - sucesso **e** falha são evidenciados com `control_id` COSO.

use chrono::NaiveDate;
use serde_json::json;

use crate::{
    assignment::{PersonAssignment, PersonAssignmentId},
    audit::{RhAuditAction, RhAuditEvent, RhAuditPort, RhEventOutcome},
    controls,
    ports::{PersonAssignmentRepository, RhAuditOutbox, UserRepository},
    RhError, UserId, UserIdentity,
};

pub struct PersonAssignmentService<R: PersonAssignmentRepository + RhAuditOutbox> {
    store: R,
}

impl<R: PersonAssignmentRepository + RhAuditOutbox> PersonAssignmentService<R> {
    pub fn new(store: R) -> Self {
        Self { store }
    }

    /// Cria uma nova afetação, evidenciando o resultado.
    ///
    /// `actor` — ID do utilizador que iniciou a operação (para `RhAuditEvent`).
    pub fn assign(&self, assignment: &PersonAssignment, actor: &str) -> Result<(), RhError> {
        if let Err(e) = assignment.validate() {
            self.store.enqueue_audit(&RhAuditEvent::new(
                actor,
                RhAuditAction::Assign,
                "PersonAssignment",
                assignment.id.as_str(),
                RhEventOutcome::Failure,
                Some(controls::ASSIGN_PERSON.into()),
                Some(json!({ "error": e.to_string() })),
            ))?;
            return Err(e);
        }

        let overlap = self.store.has_overlap(
            &assignment.person_id,
            assignment.valid_from,
            assignment.valid_until,
            None,
        )?;
        if overlap {
            self.store.enqueue_audit(&RhAuditEvent::new(
                actor,
                RhAuditAction::Assign,
                "PersonAssignment",
                assignment.id.as_str(),
                RhEventOutcome::Failure,
                Some(controls::ASSIGN_PERSON.into()),
                Some(json!({
                    "error": "sobreposição temporal",
                    "person_id": assignment.person_id.as_str()
                })),
            ))?;
            return Err(RhError::AssignmentOverlap(
                assignment.person_id.as_str().to_owned(),
            ));
        }

        let event = RhAuditEvent::new(
            actor,
            RhAuditAction::Assign,
            "PersonAssignment",
            assignment.id.as_str(),
            RhEventOutcome::Success,
            Some(controls::ASSIGN_PERSON.into()),
            Some(json!({
                "person_id":   assignment.person_id.as_str(),
                "position_id": assignment.position_id,
                "unit_id":     assignment.unit_id,
                "valid_from":  assignment.valid_from.to_string(),
                "basis":       assignment.basis,
            })),
        );

        self.store.upsert_audited(assignment, &event)
    }

    /// Fecha uma afetação existente, evidenciando o resultado.
    pub fn close(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
        actor: &str,
    ) -> Result<(), RhError> {
        let assignment = match self.store.get(id)? {
            Some(a) => a,
            None => {
                self.store.enqueue_audit(&RhAuditEvent::new(
                    actor,
                    RhAuditAction::CloseAssignment,
                    "PersonAssignment",
                    id.as_str(),
                    RhEventOutcome::Failure,
                    Some(controls::CLOSE_ASSIGNMENT.into()),
                    Some(json!({ "error": "afetação não encontrada" })),
                ))?;
                return Err(RhError::AssignmentNotFound(id.as_str().to_owned()));
            }
        };

        if valid_until <= assignment.valid_from {
            self.store.enqueue_audit(&RhAuditEvent::new(
                actor,
                RhAuditAction::CloseAssignment,
                "PersonAssignment",
                id.as_str(),
                RhEventOutcome::Failure,
                Some(controls::CLOSE_ASSIGNMENT.into()),
                Some(json!({ "error": "valid_until inválido" })),
            ))?;
            return Err(RhError::InvalidAssignment(
                "valid_until deve ser posterior a valid_from da afetação".into(),
            ));
        }

        let event = RhAuditEvent::new(
            actor,
            RhAuditAction::CloseAssignment,
            "PersonAssignment",
            id.as_str(),
            RhEventOutcome::Success,
            Some(controls::CLOSE_ASSIGNMENT.into()),
            Some(json!({
                "valid_until": valid_until.to_string(),
                "version":     version,
            })),
        );

        self.store.close_audited(id, valid_until, version, &event)
    }
}

// ── UserService ───────────────────────────────────────────────────────────────

/// Serviço de ciclo de vida de utilizadores — ponto de entrada único para
/// criação, actualização e desactivação governadas.
///
/// Captura evidência COSO atomicamente com o estado em todas as operações:
/// "quem criou este utilizador, quando, com que autoridade" fica sempre registado.
pub struct UserService<R: UserRepository + RhAuditOutbox> {
    store: R,
}

impl<R: UserRepository + RhAuditOutbox> UserService<R> {
    pub fn new(store: R) -> Self {
        Self { store }
    }

    // ── Observabilidade do outbox ──────────────────────────────────────────────

    /// Nº de eventos de auditoria por entregar (saúde / health checks).
    pub fn pending_audit_count(&self) -> Result<u64, RhError> {
        self.store.pending_audit_count()
    }

    /// Nº de eventos em dead-letter (esgotaram tentativas de entrega).
    pub fn dead_letter_audit_count(&self) -> Result<u64, RhError> {
        self.store.dead_letter_audit_count()
    }

    /// Entrega os eventos pendentes ao porto de auditoria real.
    pub fn drain_audit_outbox(&self, audit: &dyn RhAuditPort) -> Result<usize, RhError> {
        self.store.drain_audit_outbox(audit)
    }

    /// Cria ou actualiza um utilizador, evidenciando o resultado.
    ///
    /// `actor` — ID do utilizador que iniciou a operação (administrador ou sistema).
    pub fn create_or_update(&self, user: &UserIdentity, actor: &str) -> Result<(), RhError> {
        if let Err(e) = user.validate() {
            self.store.enqueue_audit(&RhAuditEvent::new(
                actor,
                RhAuditAction::UpsertUser,
                "User",
                &user.user_id,
                RhEventOutcome::Failure,
                Some(controls::UPSERT_USER.into()),
                Some(json!({ "error": e.to_string() })),
            ))?;
            return Err(e);
        }

        let event = RhAuditEvent::new(
            actor,
            RhAuditAction::UpsertUser,
            "User",
            &user.user_id,
            RhEventOutcome::Success,
            Some(controls::UPSERT_USER.into()),
            Some(json!({
                "user_id":      user.user_id,
                "username":     user.username,
                "display_name": user.display_name,
                "role":         user.role.as_str(),
            })),
        );

        self.store.upsert_audited(user, &event)
    }

    /// Desactiva um utilizador, evidenciando o resultado.
    ///
    /// Não apaga o registo — preserva o histórico para rastreabilidade COSO.
    pub fn deactivate(&self, user_id: &UserId, actor: &str) -> Result<(), RhError> {
        if self.store.get_by_id(user_id)?.is_none() {
            self.store.enqueue_audit(&RhAuditEvent::new(
                actor,
                RhAuditAction::DeactivateUser,
                "User",
                user_id.as_str(),
                RhEventOutcome::Failure,
                Some(controls::DEACTIVATE_USER.into()),
                Some(json!({ "error": "utilizador não encontrado" })),
            ))?;
            return Err(RhError::UserNotFound(user_id.as_str().to_owned()));
        }

        let event = RhAuditEvent::new(
            actor,
            RhAuditAction::DeactivateUser,
            "User",
            user_id.as_str(),
            RhEventOutcome::Success,
            Some(controls::DEACTIVATE_USER.into()),
            Some(json!({ "user_id": user_id.as_str() })),
        );

        self.store.deactivate_audited(user_id, &event)
    }
}

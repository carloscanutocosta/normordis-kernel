//! Ports de persistência (hexagonal) para `core-rh`.

use chrono::NaiveDate;

use crate::{
    assignment::{PersonAssignment, PersonAssignmentId},
    audit::{RhAuditEvent, RhAuditPort},
    RhError, UserId, UserIdentity,
};

// ── UserRepository ────────────────────────────────────────────────────────────

/// Port de persistência de utilizadores.
///
/// Os métodos devolvem `RhError` directamente; o adapter SQLite mapeia erros
/// de infra para `RhError::OperationFailed`.
pub trait UserRepository {
    fn get_by_id(&self, user_id: &UserId) -> Result<Option<UserIdentity>, RhError>;
    fn get_by_username(&self, username: &str) -> Result<Option<UserIdentity>, RhError>;
    fn list_active(&self) -> Result<Vec<UserIdentity>, RhError>;

    /// Cria ou actualiza um utilizador sem evidência — usar em importações e testes.
    fn upsert(&self, user: &UserIdentity) -> Result<(), RhError>;

    /// Cria/actualiza o utilizador e enfileira `event` no outbox na mesma transação.
    fn upsert_audited(&self, user: &UserIdentity, event: &RhAuditEvent) -> Result<(), RhError>;

    /// Desactiva o utilizador sem evidência — não apaga, preserva histórico.
    fn deactivate(&self, user_id: &UserId) -> Result<(), RhError>;

    /// Desactiva o utilizador e enfileira `event` no outbox na mesma transação.
    fn deactivate_audited(&self, user_id: &UserId, event: &RhAuditEvent) -> Result<(), RhError>;
}

// ── RhAuditOutbox ─────────────────────────────────────────────────────────────

/// Outbox de evidência de auditoria, co-localizado com o estado (mesma BD).
///
/// Garante que a evidência é capturada atomicamente com a mudança de estado
/// (via métodos `*_audited` das repos) e entregue de forma fiável e idempotente,
/// mesmo que a entrega imediata falhe. Mensagens que esgotam as tentativas
/// são movidas para *dead-letter* (não bloqueiam a fila).
pub trait RhAuditOutbox {
    fn enqueue_audit(&self, event: &RhAuditEvent) -> Result<(), RhError>;

    /// Entrega os eventos pendentes ao `audit`. Idempotente e resiliente a
    /// *poison messages* (dead-letter ao esgotar tentativas). Devolve nº entregue.
    fn drain_audit_outbox(&self, audit: &dyn RhAuditPort) -> Result<usize, RhError>;

    fn pending_audit_count(&self) -> Result<u64, RhError>;
    fn dead_letter_audit_count(&self) -> Result<u64, RhError>;
}

// ── PersonAssignmentRepository ────────────────────────────────────────────────

/// Port de persistência de afetações temporais pessoa ↔ posição.
pub trait PersonAssignmentRepository {
    fn get(&self, id: &PersonAssignmentId) -> Result<Option<PersonAssignment>, RhError>;

    fn find_at(
        &self,
        person_id: &UserId,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError>;

    fn find_holder_at(
        &self,
        position_id: &str,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError>;

    /// Lista afetações activas na data `as_of`.
    fn list_active_for_person(
        &self,
        person_id: &UserId,
        as_of: NaiveDate,
    ) -> Result<Vec<PersonAssignment>, RhError>;

    /// Verifica sobreposição temporal para a mesma pessoa.
    /// `exclude_id` — exclui a afetação com esse ID (actualizações).
    fn has_overlap(
        &self,
        person_id: &UserId,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
        exclude_id: Option<&PersonAssignmentId>,
    ) -> Result<bool, RhError>;

    /// Upsert sem evidência — usar em importações e testes.
    fn upsert(&self, assignment: &PersonAssignment) -> Result<(), RhError>;

    /// Cria/actualiza a afetação e enfileira `event` no outbox na mesma transação.
    fn upsert_audited(
        &self,
        assignment: &PersonAssignment,
        event: &RhAuditEvent,
    ) -> Result<(), RhError>;

    /// Fecha a afetação (OCC) — sem evidência. Usar em importações e testes.
    fn close(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
    ) -> Result<(), RhError>;

    /// Fecha a afetação (OCC) e enfileira `event` no outbox na mesma transação.
    fn close_audited(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
        event: &RhAuditEvent,
    ) -> Result<(), RhError>;
}

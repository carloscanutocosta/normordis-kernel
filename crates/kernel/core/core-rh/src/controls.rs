//! Identificadores de controlos COSO para `core-rh`.
//!
//! Usados nos campos `control_id` dos `RhAuditEvent` emitidos pelo serviço.
//! Convenção: "COSO.RH.<OPERAÇÃO>" — maiúsculas, separado por ponto.

/// Afetação de pessoa a posição orgânica (PersonAssignment::assign).
pub const ASSIGN_PERSON: &str = "COSO.RH.ASSIGN_PERSON";

/// Encerramento de uma afetação (PersonAssignment::close).
pub const CLOSE_ASSIGNMENT: &str = "COSO.RH.CLOSE_ASSIGNMENT";

/// Criação ou actualização de utilizador.
pub const UPSERT_USER: &str = "COSO.RH.UPSERT_USER";

/// Desactivação de utilizador.
pub const DEACTIVATE_USER: &str = "COSO.RH.DEACTIVATE_USER";

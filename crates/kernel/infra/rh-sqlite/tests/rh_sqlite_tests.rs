use adapter_sqlite::SqliteRelationalConfig;
use chrono::NaiveDate;
use core_rh::{
    PersonAssignment, PersonAssignmentId, PersonAssignmentRepository, RhAuditAction, RhAuditEvent,
    RhAuditOutbox, RhAuditPort, RhError, RhEventOutcome, Role, RoleId, RoleRepository, UserId,
    UserIdentity, UserRepository, UserRole, ASSIGN_PERSON, CLOSE_ASSIGNMENT, DEACTIVATE_USER,
    UPSERT_USER,
};
use rh_sqlite::UsersSqliteStore;

fn sample_user() -> UserIdentity {
    UserIdentity {
        user_id: "user-1".to_owned(),
        username: "user1".to_owned(),
        display_name: "User One".to_owned(),
        email: Some("user1@example.test".to_owned()),
        role: UserRole::Utilizador,
    }
}

fn store() -> UsersSqliteStore {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("users.db");
    let store =
        UsersSqliteStore::open(&SqliteRelationalConfig::read_write_create(&db_path)).unwrap();
    std::mem::forget(dir);
    store
}

#[test]
fn upserts_and_gets_user() {
    let store = store();
    let user = sample_user();

    store.upsert_user(&user).unwrap();

    let loaded = store.get_user_by_id("user-1").unwrap().unwrap();
    assert_eq!(loaded, user);
}

#[test]
fn resolves_current_user() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();

    let context = store.set_current_user("user-1").unwrap();
    assert_eq!(context.current_user.user_id, "user-1");
    assert_eq!(
        store.resolve_current_user().unwrap().current_user.username,
        "user1"
    );
}

#[test]
fn updates_role() {
    let store = store();
    let mut user = sample_user();
    store.upsert_user(&user).unwrap();

    user.role = UserRole::Auditor;
    store.upsert_user(&user).unwrap();

    let loaded = store.get_user_by_username("user1").unwrap().unwrap();
    assert_eq!(loaded.role, UserRole::Auditor);
}

#[test]
fn deactivating_current_user_clears_context() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();
    store.set_current_user("user-1").unwrap();

    store.deactivate_user("user-1").unwrap();

    assert!(store.resolve_current_user().is_err());
}

#[test]
fn crate_does_not_depend_on_tauri_or_network() {
    let cargo = include_str!("../Cargo.toml").to_lowercase();
    assert!(!cargo.contains("tauri"));
    for forbidden in ["reqwest", "ureq", "hyper", "ldap", "oauth", "oidc"] {
        assert!(!cargo.contains(forbidden));
    }
}

// ── RoleRepository ──────────────────────────────────────────────────────────

fn rid(s: &str) -> RoleId {
    RoleId::new(s).unwrap()
}

#[test]
fn role_upsert_and_get() {
    let store = store();
    RoleRepository::upsert(
        &store,
        &Role::new(
            "gestor_rh",
            "Gestor de RH",
            Some("Gere pessoal".into()),
            true,
        )
        .unwrap(),
    )
    .unwrap();

    let role = RoleRepository::get(&store, &rid("gestor_rh"))
        .unwrap()
        .unwrap();
    assert_eq!(role.id.as_str(), "gestor_rh");
    assert_eq!(role.name, "Gestor de RH");
    assert_eq!(role.description.as_deref(), Some("Gere pessoal"));
    assert!(role.is_active);
}

#[test]
fn role_get_unknown_returns_none() {
    let store = store();
    assert!(RoleRepository::get(&store, &rid("inexistente"))
        .unwrap()
        .is_none());
}

#[test]
fn role_upsert_is_idempotent_update() {
    let store = store();
    RoleRepository::upsert(&store, &Role::new("admin", "Admin", None, true).unwrap()).unwrap();
    RoleRepository::upsert(
        &store,
        &Role::new("admin", "Administrador", None, true).unwrap(),
    )
    .unwrap();

    let role = RoleRepository::get(&store, &rid("admin")).unwrap().unwrap();
    assert_eq!(role.name, "Administrador");
    assert_eq!(
        RoleRepository::list_active(&store).unwrap().len(),
        1,
        "não deve duplicar"
    );
}

#[test]
fn role_exists_and_active() {
    let store = store();
    RoleRepository::upsert(&store, &Role::new("ativo", "Ativo", None, true).unwrap()).unwrap();
    RoleRepository::upsert(
        &store,
        &Role::new("inativo", "Inativo", None, false).unwrap(),
    )
    .unwrap();

    assert!(RoleRepository::exists_and_active(&store, &rid("ativo")).unwrap());
    assert!(!RoleRepository::exists_and_active(&store, &rid("inativo")).unwrap());
    assert!(!RoleRepository::exists_and_active(&store, &rid("inexistente")).unwrap());
}

#[test]
fn role_list_active_excludes_inactive() {
    let store = store();
    RoleRepository::upsert(&store, &Role::new("a", "Role A", None, true).unwrap()).unwrap();
    RoleRepository::upsert(&store, &Role::new("b", "Role B", None, true).unwrap()).unwrap();
    RoleRepository::upsert(&store, &Role::new("c", "Role C", None, false).unwrap()).unwrap();

    let active = RoleRepository::list_active(&store).unwrap();
    assert_eq!(active.len(), 2);
    assert!(active.iter().all(|r| r.is_active));
}

#[test]
fn role_deactivate_marks_inactive() {
    let store = store();
    RoleRepository::upsert(
        &store,
        &Role::new("temp", "Temporário", None, true).unwrap(),
    )
    .unwrap();

    RoleRepository::deactivate(&store, &rid("temp")).unwrap();

    let role = RoleRepository::get(&store, &rid("temp")).unwrap().unwrap();
    assert!(!role.is_active);
    assert!(!RoleRepository::exists_and_active(&store, &rid("temp")).unwrap());
}

#[test]
fn role_deactivate_unknown_fails() {
    let store = store();
    let err = RoleRepository::deactivate(&store, &rid("nao-existe")).unwrap_err();
    assert!(err.to_string().contains("nao-existe"));
}

// ── Helpers de integração ────────────────────────────────────────────────────

/// Porto de auditoria que conta os eventos recebidos.
struct CountingPort(std::cell::Cell<usize>);

impl CountingPort {
    fn new() -> Self {
        Self(std::cell::Cell::new(0))
    }
    fn count(&self) -> usize {
        self.0.get()
    }
}

impl RhAuditPort for CountingPort {
    fn record(&self, _: &RhAuditEvent) -> Result<(), RhError> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

fn audit_event(action: RhAuditAction, entity_id: &str, control_id: &str) -> RhAuditEvent {
    RhAuditEvent::new(
        "admin",
        action,
        "Test",
        entity_id,
        RhEventOutcome::Success,
        Some(control_id.into()),
        None,
    )
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn sample_assignment(person_id: &str) -> PersonAssignment {
    PersonAssignment {
        id: PersonAssignmentId::new("asgn-001").unwrap(),
        person_id: UserId::new(person_id).unwrap(),
        position_id: "pos-001".into(),
        unit_id: "unit-001".into(),
        basis: "Despacho 1/2025".into(),
        valid_from: date(2025, 1, 1),
        valid_until: None,
        version: 0,
    }
}

// ── UserRepository — operações auditadas ─────────────────────────────────────

#[test]
fn upsert_audited_persiste_estado_e_evidencia() {
    let store = store();
    let user = sample_user();
    let event = audit_event(RhAuditAction::UpsertUser, &user.user_id, UPSERT_USER);

    UserRepository::upsert_audited(&store, &user, &event).unwrap();

    // Estado persistido
    let loaded = store.get_user_by_id("user-1").unwrap().unwrap();
    assert_eq!(loaded.username, "user1");

    // Evidência no outbox — exactamente 1 evento
    assert_eq!(store.pending_audit_count().unwrap(), 1);
}

#[test]
fn upsert_audited_update_acumula_evidencia_separada() {
    let store = store();
    let user = sample_user();
    UserRepository::upsert_audited(
        &store,
        &user,
        &audit_event(RhAuditAction::UpsertUser, &user.user_id, UPSERT_USER),
    )
    .unwrap();

    let mut actualizado = sample_user();
    actualizado.display_name = "Utilizador Actualizado".into();
    UserRepository::upsert_audited(
        &store,
        &actualizado,
        &audit_event(RhAuditAction::UpsertUser, &actualizado.user_id, UPSERT_USER),
    )
    .unwrap();

    // Estado reflecte o segundo upsert
    let loaded = store.get_user_by_id("user-1").unwrap().unwrap();
    assert_eq!(loaded.display_name, "Utilizador Actualizado");

    // Dois eventos separados no outbox
    assert_eq!(store.pending_audit_count().unwrap(), 2);
}

#[test]
fn deactivate_audited_desactiva_e_emite_evidencia() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();

    let uid = UserId::new("user-1").unwrap();
    let event = audit_event(RhAuditAction::DeactivateUser, "user-1", DEACTIVATE_USER);

    UserRepository::deactivate_audited(&store, &uid, &event).unwrap();

    // Utilizador desactivado
    assert!(store.get_user_by_id("user-1").unwrap().is_none());

    // Evidência no outbox
    assert_eq!(store.pending_audit_count().unwrap(), 1);
}

#[test]
fn deactivate_audited_nao_encontrado_nao_deixa_evidencia() {
    let store = store();
    let uid = UserId::new("nao-existe").unwrap();
    let event = audit_event(RhAuditAction::DeactivateUser, "nao-existe", DEACTIVATE_USER);

    let err = UserRepository::deactivate_audited(&store, &uid, &event).unwrap_err();
    assert!(matches!(err, RhError::UserNotFound(_)));

    // Nenhum evento no outbox — transação foi revertida
    assert_eq!(store.pending_audit_count().unwrap(), 0);
}

// ── PersonAssignmentRepository — operações auditadas ─────────────────────────

#[test]
fn assignment_upsert_audited_persiste_estado_e_evidencia() {
    let store = store();
    // FK: a afetação referencia um utilizador existente
    store.upsert_user(&sample_user()).unwrap();

    let assignment = sample_assignment("user-1");
    let event = audit_event(RhAuditAction::Assign, "asgn-001", ASSIGN_PERSON);

    PersonAssignmentRepository::upsert_audited(&store, &assignment, &event).unwrap();

    // Estado persistido
    let loaded = store
        .find_at(&UserId::new("user-1").unwrap(), date(2025, 6, 1))
        .unwrap()
        .unwrap();
    assert_eq!(loaded.position_id, "pos-001");
    assert_eq!(loaded.basis, "Despacho 1/2025");

    // Evidência no outbox
    assert_eq!(store.pending_audit_count().unwrap(), 1);
}

#[test]
fn assignment_close_audited_fecha_e_emite_evidencia() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();

    let assignment = sample_assignment("user-1");
    PersonAssignmentRepository::upsert(&store, &assignment).unwrap();

    let id = PersonAssignmentId::new("asgn-001").unwrap();
    let until = date(2025, 12, 31);
    let event = audit_event(RhAuditAction::CloseAssignment, "asgn-001", CLOSE_ASSIGNMENT);

    PersonAssignmentRepository::close_audited(&store, &id, until, 0, &event).unwrap();

    // Afetação fechada com valid_until correcto
    let closed = PersonAssignmentRepository::get(&store, &id)
        .unwrap()
        .unwrap();
    assert_eq!(closed.valid_until, Some(until));
    assert_eq!(closed.version, 1);

    // Evidência no outbox
    assert_eq!(store.pending_audit_count().unwrap(), 1);
}

#[test]
fn assignment_close_audited_versao_errada_nao_altera_estado_nem_emite_evidencia() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();

    let assignment = sample_assignment("user-1");
    PersonAssignmentRepository::upsert(&store, &assignment).unwrap();

    let id = PersonAssignmentId::new("asgn-001").unwrap();
    let until = date(2025, 12, 31);
    let event = audit_event(RhAuditAction::CloseAssignment, "asgn-001", CLOSE_ASSIGNMENT);

    // Versão errada (1 em vez de 0)
    let err = PersonAssignmentRepository::close_audited(&store, &id, until, 1, &event).unwrap_err();
    assert!(matches!(err, RhError::OperationFailed(_)));

    // Estado inalterado — OCC funcionou
    let unchanged = PersonAssignmentRepository::get(&store, &id)
        .unwrap()
        .unwrap();
    assert!(unchanged.valid_until.is_none());
    assert_eq!(unchanged.version, 0);

    // Nenhuma evidência — transação revertida
    assert_eq!(store.pending_audit_count().unwrap(), 0);
}

// ── drain_audit_outbox ────────────────────────────────────────────────────────

#[test]
fn drain_entrega_todos_os_eventos_pendentes() {
    let store = store();
    store.upsert_user(&sample_user()).unwrap();

    // Enfileira 3 eventos
    UserRepository::upsert_audited(
        &store,
        &sample_user(),
        &audit_event(RhAuditAction::UpsertUser, "user-1", UPSERT_USER),
    )
    .unwrap();
    let uid = UserId::new("user-1").unwrap();
    UserRepository::deactivate_audited(
        &store,
        &uid,
        &audit_event(RhAuditAction::DeactivateUser, "user-1", DEACTIVATE_USER),
    )
    .unwrap();
    // Evento manual adicional
    store
        .enqueue_audit(&audit_event(
            RhAuditAction::UpsertUser,
            "outro",
            UPSERT_USER,
        ))
        .unwrap();

    assert_eq!(store.pending_audit_count().unwrap(), 3);

    let port = CountingPort::new();
    let entregues = store.drain_audit_outbox(&port).unwrap();

    assert_eq!(entregues, 3);
    assert_eq!(port.count(), 3);
    assert_eq!(store.pending_audit_count().unwrap(), 0);
}

#[test]
fn drain_idempotente_segunda_chamada_entrega_zero() {
    let store = store();
    UserRepository::upsert_audited(
        &store,
        &sample_user(),
        &audit_event(RhAuditAction::UpsertUser, "user-1", UPSERT_USER),
    )
    .unwrap();

    store.drain_audit_outbox(&CountingPort::new()).unwrap();
    let segunda = store.drain_audit_outbox(&CountingPort::new()).unwrap();

    assert_eq!(segunda, 0);
    assert_eq!(store.pending_audit_count().unwrap(), 0);
}

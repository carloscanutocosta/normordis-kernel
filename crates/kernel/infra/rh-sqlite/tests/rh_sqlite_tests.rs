use adapter_sqlite::SqliteRelationalConfig;
use core_rh::{Role, RoleId, RoleRepository, UserIdentity, UserRole};
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
    store
        .upsert(
            &Role::new(
                "gestor_rh",
                "Gestor de RH",
                Some("Gere pessoal".into()),
                true,
            )
            .unwrap(),
        )
        .unwrap();

    let role = store.get(&rid("gestor_rh")).unwrap().unwrap();
    assert_eq!(role.id.as_str(), "gestor_rh");
    assert_eq!(role.name, "Gestor de RH");
    assert_eq!(role.description.as_deref(), Some("Gere pessoal"));
    assert!(role.is_active);
}

#[test]
fn role_get_unknown_returns_none() {
    let store = store();
    assert!(store.get(&rid("inexistente")).unwrap().is_none());
}

#[test]
fn role_upsert_is_idempotent_update() {
    let store = store();
    store
        .upsert(&Role::new("admin", "Admin", None, true).unwrap())
        .unwrap();
    store
        .upsert(&Role::new("admin", "Administrador", None, true).unwrap())
        .unwrap();

    let role = store.get(&rid("admin")).unwrap().unwrap();
    assert_eq!(role.name, "Administrador");
    assert_eq!(store.list_active().unwrap().len(), 1, "não deve duplicar");
}

#[test]
fn role_exists_and_active() {
    let store = store();
    store
        .upsert(&Role::new("ativo", "Ativo", None, true).unwrap())
        .unwrap();
    store
        .upsert(&Role::new("inativo", "Inativo", None, false).unwrap())
        .unwrap();

    assert!(store.exists_and_active(&rid("ativo")).unwrap());
    assert!(!store.exists_and_active(&rid("inativo")).unwrap());
    assert!(!store.exists_and_active(&rid("inexistente")).unwrap());
}

#[test]
fn role_list_active_excludes_inactive() {
    let store = store();
    store
        .upsert(&Role::new("a", "Role A", None, true).unwrap())
        .unwrap();
    store
        .upsert(&Role::new("b", "Role B", None, true).unwrap())
        .unwrap();
    store
        .upsert(&Role::new("c", "Role C", None, false).unwrap())
        .unwrap();

    let active = store.list_active().unwrap();
    assert_eq!(active.len(), 2);
    assert!(active.iter().all(|r| r.is_active));
}

#[test]
fn role_deactivate_marks_inactive() {
    let store = store();
    store
        .upsert(&Role::new("temp", "Temporário", None, true).unwrap())
        .unwrap();

    store.deactivate(&rid("temp")).unwrap();

    let role = store.get(&rid("temp")).unwrap().unwrap();
    assert!(!role.is_active);
    assert!(!store.exists_and_active(&rid("temp")).unwrap());
}

#[test]
fn role_deactivate_unknown_fails() {
    let store = store();
    let err = store.deactivate(&rid("nao-existe")).unwrap_err();
    assert!(err.to_string().contains("nao-existe"));
}

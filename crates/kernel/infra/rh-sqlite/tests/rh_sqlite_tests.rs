use adapter_sqlite::SqliteRelationalConfig;
use core_rh::{UserIdentity, UserRole};
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

use chrono::Utc;
use core_rh::{
    audit_actor_from_user, resolve_current_user, CurrentSession, CurrentUser, OrgUnitRef, RhError,
    Role, UserId, UserIdentity, UserProfile, UserRole,
};

fn valid_profile() -> UserProfile {
    UserProfile::new(
        UserId::new("ana.silva").unwrap(),
        "ana.silva",
        "Ana Silva",
        Some("ana.silva@example.test".to_owned()),
        UserRole::Utilizador,
        vec![Role::new("technician", "Tecnico").unwrap()],
        Some(OrgUnitRef::new("org-1", Some("Unidade 1".to_owned())).unwrap()),
    )
    .unwrap()
}

#[test]
fn user_id_valid() {
    let id = UserId::new("user_1-a.b").unwrap();
    assert_eq!(id.as_str(), "user_1-a.b");
}

#[test]
fn user_id_empty_fails() {
    assert_eq!(UserId::new("").unwrap_err(), RhError::InvalidUserId);
}

#[test]
fn user_id_with_spaces_fails() {
    assert_eq!(
        UserId::new("ana silva").unwrap_err(),
        RhError::InvalidUserId
    );
}

#[test]
fn role_valid() {
    let role = Role::new("manager", "Manager").unwrap();
    assert_eq!(role.role_id, "manager");
}

#[test]
fn role_invalid_fails() {
    assert_eq!(
        Role::new("bad role", "Bad Role").unwrap_err(),
        RhError::InvalidRole
    );
}

#[test]
fn org_unit_ref_valid() {
    let org = OrgUnitRef::new("unit-1", Some("Unit 1".to_owned())).unwrap();
    assert_eq!(org.org_unit_id, "unit-1");
}

#[test]
fn user_profile_valid() {
    let profile = valid_profile();
    assert_eq!(profile.user_id.as_str(), "ana.silva");
    assert_eq!(profile.username, "ana.silva");
    assert_eq!(profile.email.as_deref(), Some("ana.silva@example.test"));
    assert_eq!(profile.user_role, UserRole::Utilizador);
}

#[test]
fn display_name_empty_fails() {
    assert_eq!(
        UserProfile::new(
            UserId::new("ana.silva").unwrap(),
            "ana.silva",
            "",
            None,
            UserRole::Utilizador,
            vec![],
            None
        )
        .unwrap_err(),
        RhError::InvalidProfile
    );
}

#[test]
fn username_empty_fails() {
    assert_eq!(
        UserProfile::new(
            UserId::new("ana.silva").unwrap(),
            "",
            "Ana Silva",
            None,
            UserRole::Utilizador,
            vec![],
            None
        )
        .unwrap_err(),
        RhError::InvalidProfile
    );
}

#[test]
fn invalid_email_fails() {
    assert_eq!(
        UserProfile::new(
            UserId::new("ana.silva").unwrap(),
            "ana.silva",
            "Ana Silva",
            Some("bad email".to_owned()),
            UserRole::Utilizador,
            vec![],
            None
        )
        .unwrap_err(),
        RhError::InvalidProfile
    );
}

#[test]
fn user_identity_resolves_current_user() {
    let context = resolve_current_user(UserIdentity {
        user_id: "ana.silva".to_owned(),
        username: "ana.silva".to_owned(),
        display_name: "Ana Silva".to_owned(),
        email: Some("ana.silva@example.test".to_owned()),
        role: UserRole::Utilizador,
    })
    .unwrap();

    assert_eq!(context.current_user.user_id, "ana.silva");
}

#[test]
fn user_identity_rejects_empty_username() {
    let err = resolve_current_user(UserIdentity {
        user_id: "ana.silva".to_owned(),
        username: "".to_owned(),
        display_name: "Ana Silva".to_owned(),
        email: None,
        role: UserRole::Utilizador,
    })
    .unwrap_err();

    assert_eq!(err, RhError::InvalidProfile);
}

#[test]
fn current_user_wraps_profile() {
    let current = CurrentUser::new(valid_profile()).unwrap();
    assert_eq!(current.user_id().as_str(), "ana.silva");
}

#[test]
fn current_session_creates_uuid() {
    let session = CurrentSession::new(valid_profile());
    assert!(!session.session_id.is_nil());
}

#[test]
fn current_session_sets_utc_timestamp() {
    let before = Utc::now();
    let session = CurrentSession::new(valid_profile());
    let after = Utc::now();

    assert!(session.started_at_utc >= before);
    assert!(session.started_at_utc <= after);
}

#[test]
fn audit_actor_from_user_works() {
    let profile = valid_profile();
    let actor = audit_actor_from_user(&profile);

    assert_eq!(actor.actor_id, "ana.silva");
    assert_eq!(actor.actor_name.as_deref(), Some("Ana Silva"));
    assert_eq!(actor.actor_type.as_deref(), Some("user"));
}

#[test]
fn rh_error_converts_to_mini_error() {
    let mini = RhError::InvalidUserId.to_mini_error();
    assert_eq!(mini.code.as_str(), "MINI.RH.INVALID_USER_ID");
    assert_eq!(mini.component.as_str(), "core-rh");
}

#[test]
fn crate_does_not_depend_on_sqlite() {
    let cargo = include_str!("../Cargo.toml").to_lowercase();
    assert!(!cargo.contains("sqlite"));
    assert!(!cargo.contains("rusqlite"));
    assert!(!cargo.contains("adapter-sqlite"));
}

#[test]
fn crate_does_not_depend_on_tauri() {
    let cargo = include_str!("../Cargo.toml").to_lowercase();
    assert!(!cargo.contains("tauri"));
}

#[test]
fn crate_does_not_depend_on_network() {
    let cargo = include_str!("../Cargo.toml").to_lowercase();
    for forbidden in ["reqwest", "ureq", "hyper", "tokio", "ldap", "oauth", "oidc"] {
        assert!(!cargo.contains(forbidden));
    }
}

#[test]
fn crate_does_not_depend_on_core_org() {
    let cargo = include_str!("../Cargo.toml").to_lowercase();
    assert!(!cargo.contains("core-org"));
}

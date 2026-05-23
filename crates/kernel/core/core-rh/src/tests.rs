//! Testes unitários de `core-rh`.
//!
//! Cobrem invariantes de domínio: validação de UserId, papéis funcionais,
//! perfil de utilizador, sessão, referência orgânica e serialização de enums.

use crate::{
    error::RhError,
    identity::{AuthorMetadata, UserContext, UserIdentity},
    org::{OrgPositionRef, OrgUnitRef},
    role::{Role, UserRole},
    session::{CurrentSession, CurrentUser},
    user::{UserId, UserProfile},
    validate::validate_user_id_value,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sample_profile() -> UserProfile {
    UserProfile {
        user_id: UserId::new("user-001").unwrap(),
        username: "joao.silva".into(),
        display_name: "João Silva".into(),
        email: Some("joao@example.com".into()),
        user_role: UserRole::Utilizador,
        roles: vec![],
        org_unit: None,
    }
}

fn sample_identity() -> UserIdentity {
    UserIdentity {
        user_id: "user-001".into(),
        username: "joao.silva".into(),
        display_name: "João Silva".into(),
        email: Some("joao@example.com".into()),
        role: UserRole::Utilizador,
    }
}

// ── UserId ────────────────────────────────────────────────────────────────────

#[test]
fn user_id_valido_aceita() {
    assert!(UserId::new("user-001").is_ok());
    assert!(UserId::new("user.nome_completo").is_ok());
    assert!(UserId::new("abc123").is_ok());
}

#[test]
fn user_id_vazio_rejeita() {
    assert!(matches!(UserId::new(""), Err(RhError::InvalidUserId)));
}

#[test]
fn user_id_com_espaco_rejeita() {
    assert!(matches!(
        UserId::new("user 001"),
        Err(RhError::InvalidUserId)
    ));
}

#[test]
fn user_id_com_char_invalido_rejeita() {
    assert!(matches!(
        UserId::new("user@001"),
        Err(RhError::InvalidUserId)
    ));
    assert!(matches!(
        UserId::new("user/001"),
        Err(RhError::InvalidUserId)
    ));
}

#[test]
fn user_id_demasiado_longo_rejeita() {
    let longo = "a".repeat(129);
    assert!(matches!(
        validate_user_id_value(&longo),
        Err(RhError::InvalidUserId)
    ));
}

#[test]
fn user_id_no_limite_aceita() {
    let limite = "a".repeat(128);
    assert!(validate_user_id_value(&limite).is_ok());
}

// ── UserRole ──────────────────────────────────────────────────────────────────

#[test]
fn user_role_as_str_round_trip() {
    for (role, s) in [
        (UserRole::Utilizador, "utilizador"),
        (UserRole::Auditor, "auditor"),
        (UserRole::Administrator, "administrator"),
    ] {
        assert_eq!(role.as_str(), s);
        assert_eq!(UserRole::from_str(s).unwrap(), role);
    }
}

#[test]
fn user_role_from_str_desconhecido_devolve_none() {
    assert!(UserRole::from_str("admin").is_none());
    assert!(UserRole::from_str("").is_none());
}

#[test]
fn user_role_try_from_desconhecido_devolve_err() {
    assert!(matches!(
        UserRole::try_from("desconhecido"),
        Err(RhError::InvalidRole)
    ));
}

#[test]
fn user_role_parse_aliases() {
    assert_eq!(UserRole::parse("standard").unwrap(), UserRole::Utilizador);
    assert_eq!(UserRole::parse("supervisor").unwrap(), UserRole::Auditor);
    assert_eq!(
        UserRole::parse("ADMINISTRATOR").unwrap(),
        UserRole::Administrator
    );
}

#[test]
fn user_role_parse_desconhecido_rejeita() {
    assert!(matches!(UserRole::parse("root"), Err(RhError::InvalidRole)));
}

// ── Role ──────────────────────────────────────────────────────────────────────

#[test]
fn role_new_valido_aceita() {
    assert!(Role::new("SIGN_L1", "Assinar ofícios nível 1").is_ok());
}

#[test]
fn role_new_role_id_vazio_rejeita() {
    assert!(matches!(
        Role::new("", "Display"),
        Err(RhError::InvalidRole)
    ));
}

#[test]
fn role_new_display_name_vazio_rejeita() {
    assert!(matches!(
        Role::new("SIGN_L1", ""),
        Err(RhError::InvalidRole)
    ));
}

#[test]
fn role_new_role_id_com_espaco_rejeita() {
    assert!(matches!(
        Role::new("SIGN L1", "Display"),
        Err(RhError::InvalidRole)
    ));
}

// ── OrgUnitRef ────────────────────────────────────────────────────────────────

#[test]
fn org_unit_ref_valido_aceita() {
    assert!(OrgUnitRef::new("unit-sf-beja", None).is_ok());
    assert!(OrgUnitRef::new("unit-sf-beja", Some("SF Beja".into())).is_ok());
}

#[test]
fn org_unit_ref_id_vazio_rejeita() {
    assert!(matches!(
        OrgUnitRef::new("", None),
        Err(RhError::InvalidOrgRef)
    ));
}

// ── UserProfile::validate ─────────────────────────────────────────────────────

#[test]
fn profile_valido_aceita() {
    assert!(sample_profile().validate().is_ok());
}

#[test]
fn profile_username_vazio_rejeita() {
    let mut p = sample_profile();
    p.username = "".into();
    assert!(matches!(p.validate(), Err(RhError::InvalidProfile)));
}

#[test]
fn profile_username_com_espaco_rejeita() {
    let mut p = sample_profile();
    p.username = "joao silva".into();
    assert!(matches!(p.validate(), Err(RhError::InvalidProfile)));
}

#[test]
fn profile_display_name_vazio_rejeita() {
    let mut p = sample_profile();
    p.display_name = "".into();
    assert!(matches!(p.validate(), Err(RhError::InvalidProfile)));
}

#[test]
fn profile_email_invalido_rejeita() {
    let mut p = sample_profile();
    p.email = Some("nao-e-email".into());
    assert!(matches!(p.validate(), Err(RhError::InvalidProfile)));
}

#[test]
fn profile_email_none_aceita() {
    let mut p = sample_profile();
    p.email = None;
    assert!(p.validate().is_ok());
}

#[test]
fn profile_email_vazio_aceita() {
    // String vazia é tratada como ausência de email — não é formato inválido.
    let mut p = sample_profile();
    p.email = Some("".into());
    assert!(p.validate().is_ok());
}

// ── UserIdentity ──────────────────────────────────────────────────────────────

#[test]
fn identity_validate_valida_aceita() {
    assert!(sample_identity().validate().is_ok());
}

#[test]
fn identity_validate_user_id_invalido_rejeita() {
    let mut id = sample_identity();
    id.user_id = "user com espaco".into();
    assert!(matches!(id.validate(), Err(RhError::InvalidUserId)));
}

#[test]
fn identity_author_metadata_mapeia_campos() {
    let id = sample_identity();
    let meta: AuthorMetadata = id.author_metadata();
    assert_eq!(meta.actor_id, "user-001");
    assert_eq!(meta.actor_name, "João Silva");
}

#[test]
fn identity_to_profile_round_trip() {
    let id = sample_identity();
    let profile = id.to_profile().unwrap();
    assert_eq!(profile.user_id.as_str(), "user-001");
    assert_eq!(profile.username, "joao.silva");
}

// ── CurrentUser ───────────────────────────────────────────────────────────────

#[test]
fn current_user_new_valido_aceita() {
    assert!(CurrentUser::new(sample_profile()).is_ok());
}

#[test]
fn current_user_id_devolve_correcto() {
    let cu = CurrentUser::new(sample_profile()).unwrap();
    assert_eq!(cu.user_id().as_str(), "user-001");
}

// ── CurrentSession ────────────────────────────────────────────────────────────

#[test]
fn current_session_new_tem_session_id_nao_nulo() {
    let s = CurrentSession::new(sample_profile());
    assert!(!s.session_id.is_nil());
}

#[test]
fn current_session_validate_valida_aceita() {
    let s = CurrentSession::new(sample_profile());
    assert!(s.validate().is_ok());
}

// ── OrgPositionRef ────────────────────────────────────────────────────────────

fn sample_position_ref() -> OrgPositionRef {
    OrgPositionRef::new("pos-001", "unit-001", "comp-001", None).unwrap()
}

#[test]
fn org_position_ref_valido_aceita() {
    assert!(sample_position_ref().validate().is_ok());
}

#[test]
fn org_position_ref_com_delegacao_aceita() {
    let r = OrgPositionRef::new("pos-001", "unit-001", "comp-001", Some("del-001".into()));
    assert!(r.is_ok());
}

#[test]
fn org_position_ref_position_id_vazio_rejeita() {
    let r = OrgPositionRef::new("", "unit-001", "comp-001", None);
    assert!(matches!(r, Err(RhError::InvalidOrgRef)));
}

#[test]
fn org_position_ref_unit_id_vazio_rejeita() {
    let r = OrgPositionRef::new("pos-001", "", "comp-001", None);
    assert!(matches!(r, Err(RhError::InvalidOrgRef)));
}

#[test]
fn org_position_ref_competency_id_vazio_rejeita() {
    let r = OrgPositionRef::new("pos-001", "unit-001", "", None);
    assert!(matches!(r, Err(RhError::InvalidOrgRef)));
}

#[test]
fn org_position_ref_delegation_id_vazio_rejeita() {
    let r = OrgPositionRef::new("pos-001", "unit-001", "comp-001", Some("  ".into()));
    assert!(matches!(r, Err(RhError::InvalidOrgRef)));
}

// ── UserContext::with_position ────────────────────────────────────────────────

fn sample_context() -> UserContext {
    crate::identity::resolve_current_user(sample_identity()).unwrap()
}

#[test]
fn user_context_sem_posicao_tem_org_position_none() {
    let ctx = sample_context();
    assert!(ctx.org_position.is_none());
}

#[test]
fn user_context_with_position_define_posicao() {
    let ctx = sample_context().with_position(sample_position_ref());
    assert!(ctx.org_position.is_some());
    assert_eq!(ctx.org_position.unwrap().position_id, "pos-001");
}

#[test]
fn user_context_with_position_preserva_identidade() {
    let ctx = sample_context().with_position(sample_position_ref());
    assert_eq!(ctx.current_user.user_id, "user-001");
}

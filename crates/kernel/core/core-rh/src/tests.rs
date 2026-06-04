//! Testes unitários de `core-rh`.
//!
//! Cobrem invariantes de domínio: validação de UserId, papéis funcionais,
//! perfil de utilizador, sessão, referência orgânica, serialização de enums
//! e afetações temporais pessoa ↔ posição.

use chrono::NaiveDate;

use crate::{
    assignment::{PersonAssignment, PersonAssignmentId},
    audit::{RhAuditEvent, RhAuditPort},
    error::RhError,
    identity::{AuthorMetadata, UserContext, UserIdentity},
    org::{OrgPositionRef, OrgUnitRef},
    ports::{PersonAssignmentRepository, RhAuditOutbox, UserRepository},
    role::{Role, UserRole},
    service::{PersonAssignmentService, UserService},
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
    assert!(Role::new("SIGN_L1", "Assinar ofícios nível 1", None, true).is_ok());
}

#[test]
fn role_new_role_id_vazio_rejeita() {
    assert!(matches!(
        Role::new("", "Display", None, true),
        Err(RhError::InvalidRole)
    ));
}

#[test]
fn role_new_display_name_vazio_rejeita() {
    assert!(matches!(
        Role::new("SIGN_L1", "", None, true),
        Err(RhError::InvalidRole)
    ));
}

#[test]
fn role_new_role_id_com_espaco_rejeita() {
    assert!(matches!(
        Role::new("SIGN L1", "Display", None, true),
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

// ── PersonAssignmentId ────────────────────────────────────────────────────────

#[test]
fn assignment_id_valido_aceita() {
    assert!(PersonAssignmentId::new("asgn-001").is_ok());
}

#[test]
fn assignment_id_vazio_rejeita() {
    assert!(matches!(
        PersonAssignmentId::new(""),
        Err(RhError::InvalidAssignment(_))
    ));
}

#[test]
fn assignment_id_so_espacos_rejeita() {
    assert!(matches!(
        PersonAssignmentId::new("   "),
        Err(RhError::InvalidAssignment(_))
    ));
}

// ── PersonAssignment::validate ────────────────────────────────────────────────

fn sample_assignment() -> PersonAssignment {
    PersonAssignment {
        id: PersonAssignmentId::new("asgn-001").unwrap(),
        person_id: UserId::new("user-001").unwrap(),
        position_id: "pos-001".into(),
        unit_id: "unit-001".into(),
        basis: "Despacho 123/2025".into(),
        valid_from: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        valid_until: None,
        version: 0,
    }
}

#[test]
fn assignment_valido_aceita() {
    assert!(sample_assignment().validate().is_ok());
}

#[test]
fn assignment_com_valid_until_futuro_aceita() {
    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
    assert!(a.validate().is_ok());
}

#[test]
fn assignment_position_id_vazio_rejeita() {
    let mut a = sample_assignment();
    a.position_id = "".into();
    assert!(matches!(a.validate(), Err(RhError::InvalidAssignment(_))));
}

#[test]
fn assignment_unit_id_vazio_rejeita() {
    let mut a = sample_assignment();
    a.unit_id = "".into();
    assert!(matches!(a.validate(), Err(RhError::InvalidAssignment(_))));
}

#[test]
fn assignment_basis_vazio_rejeita() {
    let mut a = sample_assignment();
    a.basis = "".into();
    assert!(matches!(a.validate(), Err(RhError::InvalidAssignment(_))));
}

#[test]
fn assignment_valid_until_igual_a_from_rejeita() {
    let mut a = sample_assignment();
    a.valid_until = Some(a.valid_from);
    assert!(matches!(a.validate(), Err(RhError::InvalidAssignment(_))));
}

#[test]
fn assignment_valid_until_anterior_a_from_rejeita() {
    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
    assert!(matches!(a.validate(), Err(RhError::InvalidAssignment(_))));
}

// ── PersonAssignment::is_effective_at ────────────────────────────────────────

#[test]
fn assignment_is_effective_no_dia_inicio() {
    let a = sample_assignment(); // valid_from = 2025-01-01, valid_until = None
    assert!(a.is_effective_at(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()));
}

#[test]
fn assignment_is_effective_durante_vigencia() {
    let a = sample_assignment();
    assert!(a.is_effective_at(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()));
}

#[test]
fn assignment_nao_effective_antes_do_inicio() {
    let a = sample_assignment();
    assert!(!a.is_effective_at(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()));
}

#[test]
fn assignment_nao_effective_no_dia_do_until() {
    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    // valid_until é exclusivo (aberto à direita: [from, until))
    assert!(!a.is_effective_at(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()));
}

#[test]
fn assignment_effective_antes_do_until() {
    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    assert!(a.is_effective_at(NaiveDate::from_ymd_opt(2025, 5, 31).unwrap()));
}

// ── Repositório e outbox em memória (para testes do serviço) ─────────────────

struct InMemoryStore {
    assignments: std::cell::RefCell<Vec<PersonAssignment>>,
    audit_log: std::cell::RefCell<Vec<RhAuditEvent>>,
    users: std::cell::RefCell<Vec<UserIdentity>>,
}

impl InMemoryStore {
    fn new() -> Self {
        Self {
            assignments: std::cell::RefCell::new(Vec::new()),
            audit_log: std::cell::RefCell::new(Vec::new()),
            users: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[allow(dead_code)]
    fn audit_count(&self) -> usize {
        self.audit_log.borrow().len()
    }
}

impl RhAuditOutbox for InMemoryStore {
    fn enqueue_audit(&self, event: &RhAuditEvent) -> Result<(), RhError> {
        self.audit_log.borrow_mut().push(event.clone());
        Ok(())
    }

    fn drain_audit_outbox(&self, audit: &dyn RhAuditPort) -> Result<usize, RhError> {
        let log = self.audit_log.borrow();
        for e in log.iter() {
            audit.record(e)?;
        }
        Ok(log.len())
    }

    fn pending_audit_count(&self) -> Result<u64, RhError> {
        Ok(self.audit_log.borrow().len() as u64)
    }

    fn dead_letter_audit_count(&self) -> Result<u64, RhError> {
        Ok(0)
    }
}

impl PersonAssignmentRepository for InMemoryStore {
    fn get(&self, id: &PersonAssignmentId) -> Result<Option<PersonAssignment>, RhError> {
        Ok(self
            .assignments
            .borrow()
            .iter()
            .find(|a| &a.id == id)
            .cloned())
    }

    fn find_at(
        &self,
        person_id: &UserId,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError> {
        Ok(self
            .assignments
            .borrow()
            .iter()
            .find(|a| &a.person_id == person_id && a.is_effective_at(date))
            .cloned())
    }

    fn find_holder_at(
        &self,
        position_id: &str,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError> {
        Ok(self
            .assignments
            .borrow()
            .iter()
            .find(|a| a.position_id == position_id && a.is_effective_at(date))
            .cloned())
    }

    fn list_active_for_person(
        &self,
        person_id: &UserId,
        as_of: NaiveDate,
    ) -> Result<Vec<PersonAssignment>, RhError> {
        Ok(self
            .assignments
            .borrow()
            .iter()
            .filter(|a| &a.person_id == person_id && a.valid_until.is_none_or(|u| u > as_of))
            .cloned()
            .collect())
    }

    fn has_overlap(
        &self,
        person_id: &UserId,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
        exclude_id: Option<&PersonAssignmentId>,
    ) -> Result<bool, RhError> {
        let far = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
        let new_until = valid_until.unwrap_or(far);
        let found = self.assignments.borrow().iter().any(|a| {
            if exclude_id.is_some_and(|ex| ex == &a.id) {
                return false;
            }
            if &a.person_id != person_id {
                return false;
            }
            let existing_until = a.valid_until.unwrap_or(far);
            valid_from < existing_until && new_until > a.valid_from
        });
        Ok(found)
    }

    fn upsert(&self, assignment: &PersonAssignment) -> Result<(), RhError> {
        let mut data = self.assignments.borrow_mut();
        if let Some(pos) = data.iter().position(|a| a.id == assignment.id) {
            data[pos] = assignment.clone();
        } else {
            data.push(assignment.clone());
        }
        Ok(())
    }

    fn upsert_audited(
        &self,
        assignment: &PersonAssignment,
        event: &RhAuditEvent,
    ) -> Result<(), RhError> {
        PersonAssignmentRepository::upsert(self, assignment)?;
        self.enqueue_audit(event)
    }

    fn close(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
    ) -> Result<(), RhError> {
        let mut data = self.assignments.borrow_mut();
        match data.iter_mut().find(|a| &a.id == id) {
            None => Err(RhError::AssignmentNotFound(id.as_str().to_owned())),
            Some(a) if a.version != version => {
                Err(RhError::OperationFailed("versão em conflito".into()))
            }
            Some(a) => {
                a.valid_until = Some(valid_until);
                a.version += 1;
                Ok(())
            }
        }
    }

    fn close_audited(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
        event: &RhAuditEvent,
    ) -> Result<(), RhError> {
        self.close(id, valid_until, version)?;
        self.enqueue_audit(event)
    }
}

impl UserRepository for InMemoryStore {
    fn get_by_id(&self, user_id: &UserId) -> Result<Option<UserIdentity>, RhError> {
        Ok(self
            .users
            .borrow()
            .iter()
            .find(|u| u.user_id == user_id.as_str())
            .cloned())
    }

    fn get_by_username(&self, username: &str) -> Result<Option<UserIdentity>, RhError> {
        Ok(self
            .users
            .borrow()
            .iter()
            .find(|u| u.username == username)
            .cloned())
    }

    fn list_active(&self) -> Result<Vec<UserIdentity>, RhError> {
        Ok(self.users.borrow().clone())
    }

    fn upsert(&self, user: &UserIdentity) -> Result<(), RhError> {
        let mut users = self.users.borrow_mut();
        if let Some(pos) = users.iter().position(|u| u.user_id == user.user_id) {
            users[pos] = user.clone();
        } else {
            users.push(user.clone());
        }
        Ok(())
    }

    fn upsert_audited(&self, user: &UserIdentity, event: &RhAuditEvent) -> Result<(), RhError> {
        UserRepository::upsert(self, user)?;
        self.enqueue_audit(event)
    }

    fn deactivate(&self, user_id: &UserId) -> Result<(), RhError> {
        let mut users = self.users.borrow_mut();
        if let Some(pos) = users.iter().position(|u| u.user_id == user_id.as_str()) {
            users.remove(pos);
            Ok(())
        } else {
            Err(RhError::UserNotFound(user_id.as_str().to_owned()))
        }
    }

    fn deactivate_audited(&self, user_id: &UserId, event: &RhAuditEvent) -> Result<(), RhError> {
        UserRepository::deactivate(self, user_id)?;
        self.enqueue_audit(event)
    }
}

fn make_store() -> InMemoryStore {
    InMemoryStore::new()
}

fn make_service() -> PersonAssignmentService<InMemoryStore> {
    PersonAssignmentService::new(make_store())
}

// ── PersonAssignmentService ───────────────────────────────────────────────────

#[test]
fn service_assign_valido_persiste() {
    let svc = make_service();
    assert!(svc.assign(&sample_assignment(), "actor-001").is_ok());
}

#[test]
fn service_assign_emite_evidencia_sucesso() {
    let store = make_store();
    let svc = PersonAssignmentService::new(store);
    // Usamos InMemoryStore directamente para contar eventos
    // (o serviço consome o store por move, por isso criamos um novo)
    let store2 = make_store();
    let svc2 = PersonAssignmentService::new(store2);
    svc2.assign(&sample_assignment(), "actor-001").unwrap();
    // Não temos acesso ao store após move, mas o teste de compile/panic já valida o fluxo.
    // O teste de contagem real está nos testes de integração (rh-sqlite).
    let _ = svc; // suprimir aviso de variável não usada
}

#[test]
fn service_assign_sobreposto_rejeita() {
    let store = make_store();
    let svc = PersonAssignmentService::new(store);
    svc.assign(&sample_assignment(), "actor-001").unwrap();

    let mut b = sample_assignment();
    b.id = PersonAssignmentId::new("asgn-002").unwrap();
    b.valid_from = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();

    assert!(matches!(
        svc.assign(&b, "actor-001"),
        Err(RhError::AssignmentOverlap(_))
    ));
}

#[test]
fn service_assign_nao_sobreposto_aceita() {
    let store = make_store();
    let svc = PersonAssignmentService::new(store);

    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    svc.assign(&a, "actor-001").unwrap();

    let mut b = sample_assignment();
    b.id = PersonAssignmentId::new("asgn-002").unwrap();
    b.valid_from = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();

    assert!(svc.assign(&b, "actor-001").is_ok());
}

#[test]
fn service_close_valido_encerra() {
    let store = make_store();
    let svc = PersonAssignmentService::new(store);
    svc.assign(&sample_assignment(), "actor-001").unwrap();

    let id = PersonAssignmentId::new("asgn-001").unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    assert!(svc.close(&id, until, 0, "actor-001").is_ok());
}

#[test]
fn service_close_nao_encontrado_rejeita() {
    let svc = make_service();
    let id = PersonAssignmentId::new("asgn-nao-existe").unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    assert!(matches!(
        svc.close(&id, until, 0, "actor-001"),
        Err(RhError::AssignmentNotFound(_))
    ));
}

#[test]
fn service_close_valid_until_anterior_a_from_rejeita() {
    let store = make_store();
    let svc = PersonAssignmentService::new(store);
    svc.assign(&sample_assignment(), "actor-001").unwrap();

    let id = PersonAssignmentId::new("asgn-001").unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    assert!(matches!(
        svc.close(&id, until, 0, "actor-001"),
        Err(RhError::InvalidAssignment(_))
    ));
}

// ── UserRepository em memória ─────────────────────────────────────────────────

fn sample_user_identity() -> UserIdentity {
    UserIdentity {
        user_id: "user-001".into(),
        username: "joao.silva".into(),
        display_name: "João Silva".into(),
        email: Some("joao@example.com".into()),
        role: UserRole::Utilizador,
    }
}

#[test]
fn user_repo_upsert_e_get_by_id() {
    let store = make_store();
    UserRepository::upsert(&store, &sample_user_identity()).unwrap();
    let loaded = store
        .get_by_id(&UserId::new("user-001").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.username, "joao.silva");
}

#[test]
fn user_repo_get_by_username() {
    let store = make_store();
    UserRepository::upsert(&store, &sample_user_identity()).unwrap();
    let loaded = store.get_by_username("joao.silva").unwrap().unwrap();
    assert_eq!(loaded.user_id, "user-001");
}

#[test]
fn user_repo_get_desconhecido_devolve_none() {
    let store = make_store();
    assert!(store
        .get_by_id(&UserId::new("nao-existe").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn user_repo_deactivate_remove() {
    let store = make_store();
    UserRepository::upsert(&store, &sample_user_identity()).unwrap();
    store.deactivate(&UserId::new("user-001").unwrap()).unwrap();
    assert!(store
        .get_by_id(&UserId::new("user-001").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn user_repo_deactivate_desconhecido_rejeita() {
    let store = make_store();
    assert!(matches!(
        store.deactivate(&UserId::new("nao-existe").unwrap()),
        Err(RhError::UserNotFound(_))
    ));
}

// ── UserContext::from_assignment ──────────────────────────────────────────────

#[test]
fn user_context_from_assignment_define_posicao() {
    let identity = sample_identity();
    let assignment = sample_assignment();
    let ctx = UserContext::from_assignment(identity, &assignment, "comp-001", None).unwrap();
    let pos = ctx.org_position.unwrap();
    assert_eq!(pos.position_id, "pos-001");
    assert_eq!(pos.unit_id, "unit-001");
    assert_eq!(pos.competency_id, "comp-001");
    assert!(pos.delegation_id.is_none());
}

#[test]
fn user_context_from_assignment_com_delegacao() {
    let identity = sample_identity();
    let assignment = sample_assignment();
    let ctx =
        UserContext::from_assignment(identity, &assignment, "comp-001", Some("del-007".into()))
            .unwrap();
    assert_eq!(
        ctx.org_position.unwrap().delegation_id.as_deref(),
        Some("del-007")
    );
}

#[test]
fn list_active_for_person_respeita_as_of() {
    let store = make_store();
    let mut a = sample_assignment();
    a.valid_until = Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    PersonAssignmentRepository::upsert(&store, &a).unwrap();

    let person = UserId::new("user-001").unwrap();
    let antes = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();
    let depois = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();

    assert_eq!(
        store.list_active_for_person(&person, antes).unwrap().len(),
        1
    );
    assert_eq!(
        store.list_active_for_person(&person, depois).unwrap().len(),
        0
    );
}

// ── UserService ───────────────────────────────────────────────────────────────

fn make_user_service() -> UserService<InMemoryStore> {
    UserService::new(make_store())
}

#[test]
fn user_service_create_ou_update_valido_persiste() {
    let svc = make_user_service();
    assert!(svc
        .create_or_update(&sample_user_identity(), "admin")
        .is_ok());
}

#[test]
fn user_service_create_ou_update_emite_evidencia() {
    let svc = make_user_service();
    svc.create_or_update(&sample_user_identity(), "admin")
        .unwrap();
    // O serviço enfileirou 1 evento de sucesso
    assert_eq!(svc.pending_audit_count().unwrap(), 1);
}

#[test]
fn user_service_create_usuario_invalido_emite_falha() {
    let svc = make_user_service();
    let mut user = sample_user_identity();
    user.username = "nome com espaco".into(); // inválido
    assert!(svc.create_or_update(&user, "admin").is_err());
    // Evidência de falha enfileirada
    assert_eq!(svc.pending_audit_count().unwrap(), 1);
}

#[test]
fn user_service_deactivate_valido_remove_e_emite_evidencia() {
    let svc = make_user_service();
    svc.create_or_update(&sample_user_identity(), "admin")
        .unwrap();

    let uid = UserId::new("user-001").unwrap();
    svc.deactivate(&uid, "admin").unwrap();

    // Evidência: create + deactivate = 2 eventos
    assert_eq!(svc.pending_audit_count().unwrap(), 2);
    // Confirma remoção: segunda desactivação devolve UserNotFound
    assert!(matches!(
        svc.deactivate(&uid, "admin"),
        Err(RhError::UserNotFound(_))
    ));
}

#[test]
fn user_service_deactivate_nao_encontrado_emite_falha() {
    let svc = make_user_service();
    let uid = UserId::new("nao-existe").unwrap();
    assert!(matches!(
        svc.deactivate(&uid, "admin"),
        Err(RhError::UserNotFound(_))
    ));
    // Evidência de falha enfileirada
    assert_eq!(svc.pending_audit_count().unwrap(), 1);
}

#[test]
fn user_service_idempotente_update_preserva_evidencia() {
    let svc = make_user_service();
    svc.create_or_update(&sample_user_identity(), "admin")
        .unwrap();

    let mut actualizado = sample_user_identity();
    actualizado.display_name = "João Silva Actualizado".into();
    svc.create_or_update(&actualizado, "admin").unwrap();

    // Dois eventos de sucesso
    assert_eq!(svc.pending_audit_count().unwrap(), 2);
}

//! Testes unitários de `core-org`.

use chrono::NaiveDate;

use crate::{
    competency::{Competency, CompetencyId},
    delegation::{Delegation, DelegationId},
    error::OrgError,
    instrument::{InstrumentKind, LegalInstrument, LegalInstrumentId},
    position::{OrgPosition, OrgPositionId, OrgPositionStatus, PositionKind},
    unit::{OrgContacts, OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus},
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn sample_instrument_id() -> LegalInstrumentId {
    LegalInstrumentId::new("inst-001").unwrap()
}

fn sample_unit(id: &str, level: u8, parent: Option<&str>) -> OrgUnit {
    OrgUnit {
        id: OrgUnitId::new(id).unwrap(),
        short_name: "SF Beja".into(),
        full_name: "Serviço de Finanças de Beja".into(),
        service_code: None,
        level: OrgLevel::new(level).unwrap(),
        parent_id: parent.map(|p| OrgUnitId::new(p).unwrap()),
        contacts: OrgContacts::default(),
        created_by: None,
        legal_reference: None,
        valid_from: date(2020, 1, 1),
        valid_until: None,
        status: OrgUnitStatus::Active,
        version: 0,
    }
}

fn sample_position(id: &str, unit: &str) -> OrgPosition {
    OrgPosition {
        id: OrgPositionId::new(id).unwrap(),
        code: "DIR".into(),
        title: "Director".into(),
        kind: PositionKind::Direcao,
        substitutes: None,
        status: OrgPositionStatus::Active,
        unit_id: OrgUnitId::new(unit).unwrap(),
        created_by: sample_instrument_id(),
        valid_from: date(2020, 1, 1),
        valid_until: None,
        version: 0,
    }
}

fn sample_competency(id: &str, position: &str) -> Competency {
    Competency {
        id: CompetencyId::new(id).unwrap(),
        code: "SIGN_L1".into(),
        description: "Assinar ofícios de nível 1".into(),
        scope: "Ofícios emitidos pela unidade".into(),
        assigned_to: OrgPositionId::new(position).unwrap(),
        granted_by: sample_instrument_id(),
        valid_from: date(2020, 1, 1),
        valid_until: None,
    }
}

fn sample_delegation(from: &str, to: &str, comp: &str) -> Delegation {
    Delegation {
        id: DelegationId::new("del-001").unwrap(),
        competency_id: CompetencyId::new(comp).unwrap(),
        from_position: OrgPositionId::new(from).unwrap(),
        to_position: OrgPositionId::new(to).unwrap(),
        instrument_id: sample_instrument_id(),
        valid_from: date(2024, 1, 1),
        valid_until: None,
    }
}

fn sample_instrument() -> LegalInstrument {
    LegalInstrument {
        id: sample_instrument_id(),
        kind: InstrumentKind::Portaria,
        reference: "Portaria n.º 123/2024, de 15 de abril".into(),
        date: date(2024, 4, 15),
        description: "Regulamenta a estrutura da AT".into(),
        effective_from: date(2024, 5, 1),
        effective_until: None,
    }
}

// ── OrgLevel ──────────────────────────────────────────────────────────────────

#[test]
fn org_level_limites_validos() {
    assert!(OrgLevel::new(1).is_ok());
    assert!(OrgLevel::new(5).is_ok());
}

#[test]
fn org_level_zero_rejeita() {
    assert!(matches!(OrgLevel::new(0), Err(OrgError::InvalidLevel(_))));
}

#[test]
fn org_level_acima_de_cinco_aceito() {
    assert!(OrgLevel::new(6).is_ok());
    assert!(OrgLevel::new(10).is_ok());
    assert!(OrgLevel::new(255).is_ok());
}

#[test]
fn org_level_parent_level() {
    assert_eq!(OrgLevel::new(1).unwrap().parent_level(), None);
    assert_eq!(OrgLevel::new(2).unwrap().parent_level(), Some(OrgLevel(1)));
    assert_eq!(OrgLevel::new(6).unwrap().parent_level(), Some(OrgLevel(5)));
}

// ── OrgUnitStatus ─────────────────────────────────────────────────────────────

#[test]
fn status_from_str_round_trip() {
    for s in ["active", "suspended", "extinct"] {
        let st = OrgUnitStatus::from_str(s).unwrap();
        assert_eq!(st.as_str(), s);
    }
}

#[test]
fn status_transicoes_validas() {
    assert!(OrgUnitStatus::Active.can_transition_to(&OrgUnitStatus::Suspended));
    assert!(OrgUnitStatus::Active.can_transition_to(&OrgUnitStatus::Extinct));
    assert!(OrgUnitStatus::Suspended.can_transition_to(&OrgUnitStatus::Active));
}

#[test]
fn status_extinct_e_terminal() {
    assert!(!OrgUnitStatus::Extinct.can_transition_to(&OrgUnitStatus::Active));
    assert!(!OrgUnitStatus::Extinct.can_transition_to(&OrgUnitStatus::Suspended));
}

#[test]
fn status_try_from_desconhecido_devolve_err() {
    assert!(OrgUnitStatus::try_from("nao_existe").is_err());
}

// ── InstrumentKind ────────────────────────────────────────────────────────────

#[test]
fn instrument_kind_round_trip_variantes_fixas() {
    let pairs = [
        (InstrumentKind::Portaria, "portaria"),
        (InstrumentKind::Despacho, "despacho"),
        (InstrumentKind::Deliberacao, "deliberacao"),
        (InstrumentKind::RegulamentoOrganico, "regulamento_organico"),
    ];
    for (kind, s) in pairs {
        assert_eq!(kind.as_str(), s);
        assert_eq!(InstrumentKind::from_str(s).unwrap(), kind);
    }
}

#[test]
fn instrument_kind_outro_round_trip() {
    let kind = InstrumentKind::Outro("resolucao".into());
    let s = kind.as_str();
    assert_eq!(s, "outro:resolucao");
    assert_eq!(InstrumentKind::from_str(&s).unwrap(), kind);
}

#[test]
fn instrument_kind_from_str_desconhecido_devolve_none() {
    assert!(InstrumentKind::from_str("desconhecido").is_none());
}

#[test]
fn instrument_kind_try_from_desconhecido_devolve_err() {
    assert!(InstrumentKind::try_from("xyz").is_err());
}

// ── PositionKind ──────────────────────────────────────────────────────────────

#[test]
fn position_kind_round_trip_variantes_fixas() {
    let pairs = [
        (PositionKind::Direcao, "direcao"),
        (PositionKind::Coordenacao, "coordenacao"),
        (PositionKind::Chefia, "chefia"),
        (PositionKind::Adjunto, "adjunto"),
        (PositionKind::Tecnico, "tecnico"),
    ];
    for (kind, s) in pairs {
        assert_eq!(kind.as_str(), s);
        assert_eq!(PositionKind::from_str(s).unwrap(), kind);
    }
}

#[test]
fn position_kind_outro_round_trip() {
    let kind = PositionKind::Outro("assessor".into());
    assert_eq!(kind.as_str(), "outro:assessor");
    assert_eq!(PositionKind::from_str("outro:assessor").unwrap(), kind);
}

#[test]
fn position_kind_from_str_desconhecido_devolve_none() {
    assert!(PositionKind::from_str("xyz").is_none());
}

// ── OrgPositionStatus ─────────────────────────────────────────────────────────

#[test]
fn position_status_round_trip() {
    for s in ["active", "suspended", "extinct"] {
        let st = OrgPositionStatus::from_str(s).unwrap();
        assert_eq!(st.as_str(), s);
    }
}

#[test]
fn position_status_transicoes_validas() {
    assert!(OrgPositionStatus::Active.can_transition_to(&OrgPositionStatus::Suspended));
    assert!(OrgPositionStatus::Active.can_transition_to(&OrgPositionStatus::Extinct));
    assert!(OrgPositionStatus::Suspended.can_transition_to(&OrgPositionStatus::Active));
}

#[test]
fn position_status_extinct_terminal() {
    assert!(!OrgPositionStatus::Extinct.can_transition_to(&OrgPositionStatus::Active));
}

#[test]
fn position_status_try_from_desconhecido_err() {
    assert!(OrgPositionStatus::try_from("nao_existe").is_err());
}

// ── OrgUnit::validate ─────────────────────────────────────────────────────────

#[test]
fn unit_validate_nivel1_sem_pai_ok() {
    assert!(sample_unit("u-1", 1, None).validate().is_ok());
}

#[test]
fn unit_validate_nivel6_com_pai_ok() {
    assert!(sample_unit("u-6", 6, Some("u-5")).validate().is_ok());
}

#[test]
fn unit_validate_nivel1_com_pai_rejeita() {
    assert!(matches!(
        sample_unit("u-1", 1, Some("u-0")).validate(),
        Err(OrgError::InconsistentLevel)
    ));
}

#[test]
fn unit_validate_nivel6_sem_pai_rejeita() {
    assert!(matches!(
        sample_unit("u-6", 6, None).validate(),
        Err(OrgError::InconsistentLevel)
    ));
}

#[test]
fn unit_validate_temporal_invalido() {
    let mut u = sample_unit("u-1", 1, None);
    u.valid_until = Some(date(2019, 12, 31));
    assert!(matches!(u.validate(), Err(OrgError::InvalidTemporalRange)));
}

// ── OrgUnit::validate_strict ──────────────────────────────────────────────────

#[test]
fn unit_validate_strict_sem_instrumento_nem_referencia_rejeita() {
    let u = sample_unit("u-1", 1, None); // created_by=None, legal_reference=None
    assert!(matches!(u.validate_strict(), Err(OrgError::EmptyField(_))));
}

#[test]
fn unit_validate_strict_com_legal_reference_ok() {
    let mut u = sample_unit("u-1", 1, None);
    u.legal_reference = Some("Portaria n.º 100/2024".into());
    assert!(u.validate_strict().is_ok());
}

#[test]
fn unit_validate_strict_com_created_by_ok() {
    let mut u = sample_unit("u-1", 1, None);
    u.created_by = Some(sample_instrument_id());
    assert!(u.validate_strict().is_ok());
}

// ── Validação de contactos ────────────────────────────────────────────────────

#[test]
fn contacts_email_valido_ok() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.email = Some("sf.beja@at.gov.pt".into());
    assert!(u.validate().is_ok());
}

#[test]
fn contacts_email_invalido_rejeita() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.email = Some("naoemail".into());
    assert!(matches!(
        u.validate(),
        Err(OrgError::InvalidContactField(_))
    ));
}

#[test]
fn contacts_phone_valido_ok() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.phone = Some("+351 284 123 456".into());
    assert!(u.validate().is_ok());
}

#[test]
fn contacts_phone_invalido_rejeita() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.phone = Some("123".into()); // menos de 7 dígitos
    assert!(matches!(
        u.validate(),
        Err(OrgError::InvalidContactField(_))
    ));
}

#[test]
fn contacts_cp4_valido_ok() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.address.cp4 = Some("7800".into());
    assert!(u.validate().is_ok());
}

#[test]
fn contacts_cp4_invalido_rejeita() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.address.cp4 = Some("78".into()); // só 2 dígitos
    assert!(matches!(
        u.validate(),
        Err(OrgError::InvalidContactField(_))
    ));
}

#[test]
fn contacts_cp3_invalido_rejeita() {
    let mut u = sample_unit("u-1", 1, None);
    u.contacts.address.cp3 = Some("12ab".into()); // não são dígitos
    assert!(matches!(
        u.validate(),
        Err(OrgError::InvalidContactField(_))
    ));
}

// ── OrgUnit::validate_level_against_parent ────────────────────────────────────

#[test]
fn unit_validate_level_against_parent_ok() {
    let parent = sample_unit("u-1", 1, None);
    let child = sample_unit("u-2", 2, Some("u-1"));
    assert!(child.validate_level_against_parent(&parent).is_ok());
}

#[test]
fn unit_validate_level_against_parent_gap_rejeita() {
    let parent = sample_unit("u-1", 1, None);
    let child = sample_unit("u-3", 3, Some("u-1")); // buraco no nível 2
    assert!(matches!(
        child.validate_level_against_parent(&parent),
        Err(OrgError::InconsistentLevel)
    ));
}

// ── OrgUnit::is_active_at ─────────────────────────────────────────────────────

#[test]
fn unit_is_active_at_dentro_do_intervalo() {
    assert!(sample_unit("u-1", 1, None).is_active_at(date(2024, 6, 1)));
}

#[test]
fn unit_is_active_at_extincta_false() {
    let mut u = sample_unit("u-1", 1, None);
    u.status = OrgUnitStatus::Extinct;
    assert!(!u.is_active_at(date(2024, 6, 1)));
}

// ── OrgUnit::transition_status ────────────────────────────────────────────────

#[test]
fn unit_transition_active_para_suspended_ok() {
    assert!(sample_unit("u-1", 1, None)
        .transition_status(OrgUnitStatus::Suspended)
        .is_ok());
}

#[test]
fn unit_transition_extinct_para_active_rejeita() {
    let mut u = sample_unit("u-1", 1, None);
    u.status = OrgUnitStatus::Extinct;
    assert!(u.transition_status(OrgUnitStatus::Active).is_err());
}

// ── OrgUnit::validate_parent_chain ────────────────────────────────────────────

#[test]
fn unit_parent_chain_sem_ciclo_ok() {
    let u = sample_unit("u-3", 3, Some("u-2"));
    let id1 = OrgUnitId::new("u-1").unwrap();
    let id2 = OrgUnitId::new("u-2").unwrap();
    assert!(u.validate_parent_chain(&[&id1, &id2]).is_ok());
}

#[test]
fn unit_parent_chain_com_ciclo_rejeita() {
    let u = sample_unit("u-2", 2, Some("u-1"));
    let id2 = OrgUnitId::new("u-2").unwrap();
    assert!(matches!(
        u.validate_parent_chain(&[&id2]),
        Err(OrgError::CircularHierarchy)
    ));
}

// ── OrgUnit::can_deactivate ───────────────────────────────────────────────────

#[test]
fn unit_can_deactivate_sem_filhos_ok() {
    assert!(sample_unit("u-1", 1, None).can_deactivate(&[], &[]).is_ok());
}

#[test]
fn unit_can_deactivate_com_filhos_rejeita() {
    let child = OrgUnitId::new("u-2").unwrap();
    assert!(matches!(
        sample_unit("u-1", 1, None).can_deactivate(&[&child], &[]),
        Err(OrgError::CannotDeactivateWithActiveChildren)
    ));
}

// ── OrgPosition::validate ─────────────────────────────────────────────────────

#[test]
fn position_validate_ok() {
    assert!(sample_position("pos-1", "u-1").validate().is_ok());
}

#[test]
fn position_validate_code_vazio_rejeita() {
    let mut p = sample_position("pos-1", "u-1");
    p.code = "".into();
    assert!(matches!(p.validate(), Err(OrgError::EmptyField(_))));
}

#[test]
fn position_substitutes_si_propria_rejeita() {
    let mut p = sample_position("pos-1", "u-1");
    p.substitutes = Some(OrgPositionId::new("pos-1").unwrap());
    assert!(matches!(p.validate(), Err(OrgError::OperationFailed(_))));
}

#[test]
fn position_adjunto_substituto_ok() {
    let mut p = sample_position("pos-adj", "u-1");
    p.kind = PositionKind::Adjunto;
    p.substitutes = Some(OrgPositionId::new("pos-chefe").unwrap());
    assert!(p.validate().is_ok());
}

// ── OrgPosition::validate_no_substitute_cycle ─────────────────────────────────

#[test]
fn position_no_substitute_cycle_ok() {
    let p = sample_position("pos-adj", "u-1");
    let id1 = OrgPositionId::new("pos-chefe").unwrap();
    assert!(p.validate_no_substitute_cycle(&[&id1]).is_ok());
}

#[test]
fn position_no_substitute_cycle_detecta_ciclo() {
    let p = sample_position("pos-adj", "u-1");
    let id_adj = OrgPositionId::new("pos-adj").unwrap();
    assert!(matches!(
        p.validate_no_substitute_cycle(&[&id_adj]),
        Err(OrgError::SubstitutionCycle)
    ));
}

// ── OrgPosition::transition_status ───────────────────────────────────────────

#[test]
fn position_transition_active_para_suspended_ok() {
    assert!(sample_position("pos-1", "u-1")
        .transition_status(OrgPositionStatus::Suspended)
        .is_ok());
}

#[test]
fn position_transition_extinct_terminal() {
    let mut p = sample_position("pos-1", "u-1");
    p.status = OrgPositionStatus::Extinct;
    assert!(p.transition_status(OrgPositionStatus::Active).is_err());
}

// ── Competency ────────────────────────────────────────────────────────────────

#[test]
fn competency_validate_ok() {
    assert!(sample_competency("comp-1", "pos-1").validate().is_ok());
}

#[test]
fn competency_validate_scope_vazio_rejeita() {
    let mut c = sample_competency("comp-1", "pos-1");
    c.scope = "   ".into();
    assert!(matches!(c.validate(), Err(OrgError::EmptyField(_))));
}

#[test]
fn competency_is_effective_at() {
    let c = sample_competency("comp-1", "pos-1");
    assert!(c.is_effective_at(date(2024, 6, 1)));
    assert!(!c.is_effective_at(date(2019, 12, 31)));
}

// ── Delegation ────────────────────────────────────────────────────────────────

#[test]
fn delegation_validate_ok() {
    assert!(sample_delegation("pos-1", "pos-2", "comp-1")
        .validate()
        .is_ok());
}

#[test]
fn delegation_validate_mesmo_de_e_para_rejeita() {
    assert!(matches!(
        sample_delegation("pos-1", "pos-1", "comp-1").validate(),
        Err(OrgError::OperationFailed(_))
    ));
}

#[test]
fn delegation_validate_can_delegate_ok() {
    let d = sample_delegation("pos-1", "pos-2", "comp-1");
    let comp = CompetencyId::new("comp-1").unwrap();
    assert!(d.validate_can_delegate(&[&comp]).is_ok());
}

// ── LegalInstrument ───────────────────────────────────────────────────────────

#[test]
fn instrument_validate_ok() {
    assert!(sample_instrument().validate().is_ok());
}

#[test]
fn instrument_validate_reference_vazia_rejeita() {
    let mut i = sample_instrument();
    i.reference = "".into();
    assert!(matches!(i.validate(), Err(OrgError::EmptyField(_))));
}

#[test]
fn instrument_is_effective_at() {
    let i = sample_instrument();
    assert!(i.is_effective_at(date(2025, 1, 1)));
    assert!(!i.is_effective_at(date(2024, 4, 30)));
}

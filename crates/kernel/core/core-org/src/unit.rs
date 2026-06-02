//! Unidade orgânica: hierarquia, validade temporal, máquina de estados e validações.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{LegalInstrumentId, OrgError, OrgPositionId};

/// Nível hierárquico de uma unidade orgânica (1 = raiz, sem limite máximo).
/// Nível 1 não tem pai. O pai de nível N tem sempre nível N-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgLevel(pub u8);

impl OrgLevel {
    pub const MIN: u8 = 1;

    pub fn new(n: u8) -> Result<Self, OrgError> {
        if n >= Self::MIN {
            Ok(Self(n))
        } else {
            Err(OrgError::InvalidLevel(n.to_string()))
        }
    }

    pub fn parent_level(self) -> Option<OrgLevel> {
        if self.0 > 1 {
            Some(OrgLevel(self.0 - 1))
        } else {
            None
        }
    }

    pub fn as_u8(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgUnitId(pub String);

impl OrgUnitId {
    pub fn new(id: impl Into<String>) -> Result<Self, OrgError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OrgError::EmptyField("unit_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgUnitStatus {
    Active,
    Suspended,
    Extinct,
}

impl OrgUnitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Extinct => "extinct",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "extinct" => Some(Self::Extinct),
            _ => None,
        }
    }

    /// Active → Suspended|Extinct · Suspended → Active|Extinct · Extinct é terminal.
    pub fn can_transition_to(&self, next: &OrgUnitStatus) -> bool {
        use OrgUnitStatus::*;
        matches!(
            (self, next),
            (Active, Suspended) | (Active, Extinct) | (Suspended, Active) | (Suspended, Extinct)
        )
    }
}

impl TryFrom<&str> for OrgUnitStatus {
    type Error = OrgError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
            .ok_or_else(|| OrgError::OperationFailed(format!("status desconhecido: {s}")))
    }
}

// ── Contactos e morada ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrgAddress {
    pub rua: Option<String>,
    pub numero: Option<String>,
    pub porta: Option<String>,
    pub local: Option<String>,
    /// Quatro dígitos do código postal (ex: "1000")
    pub cp4: Option<String>,
    /// Três dígitos do código postal (ex: "001")
    pub cp3: Option<String>,
    pub localidade: Option<String>,
}

impl OrgAddress {
    pub fn cod_postal(&self) -> Option<String> {
        match (&self.cp4, &self.cp3) {
            (Some(cp4), Some(cp3)) => Some(format!("{cp4}-{cp3}")),
            (Some(cp4), None) => Some(cp4.clone()),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), OrgError> {
        if let Some(ref cp4) = self.cp4 {
            if !is_cp4_valid(cp4) {
                return Err(OrgError::InvalidContactField(format!(
                    "cp4 inválido: '{cp4}' (deve ter exactamente 4 dígitos)"
                )));
            }
        }
        if let Some(ref cp3) = self.cp3 {
            if !is_cp3_valid(cp3) {
                return Err(OrgError::InvalidContactField(format!(
                    "cp3 inválido: '{cp3}' (deve ter exactamente 3 dígitos)"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrgContacts {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub address: OrgAddress,
}

impl OrgContacts {
    pub fn validate(&self) -> Result<(), OrgError> {
        if let Some(ref email) = self.email {
            if !is_email_valid(email) {
                return Err(OrgError::InvalidContactField(format!(
                    "email inválido: '{email}'"
                )));
            }
        }
        if let Some(ref phone) = self.phone {
            if !is_phone_valid(phone) {
                return Err(OrgError::InvalidContactField(format!(
                    "telefone inválido: '{phone}' (mínimo 7 dígitos)"
                )));
            }
        }
        if let Some(ref fax) = self.fax {
            if !is_phone_valid(fax) {
                return Err(OrgError::InvalidContactField(format!(
                    "fax inválido: '{fax}' (mínimo 7 dígitos)"
                )));
            }
        }
        self.address.validate()
    }
}

// ── Agregado OrgUnit ──────────────────────────────────────────────────────────

/// Unidade orgânica com validade temporal e controlo de concorrência optimista.
///
/// `created_by` referencia o instrumento jurídico que criou ou alterou a unidade.
/// `legal_reference` preserva o texto livre da referência legal para unidades sem
/// instrumento formal registado no sistema.
///
/// `version` é incrementado pelo repositório em cada `update()` e serve de guarda
/// contra escritas concorrentes (OCC — Optimistic Concurrency Control).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgUnit {
    pub id: OrgUnitId,
    pub short_name: String,
    pub full_name: String,
    pub service_code: Option<String>,
    pub level: OrgLevel,
    pub parent_id: Option<OrgUnitId>,
    pub contacts: OrgContacts,
    pub created_by: Option<LegalInstrumentId>,
    pub legal_reference: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub status: OrgUnitStatus,
    /// Versão para OCC — deve ser 0 em novas entidades.
    pub version: u32,
}

impl OrgUnit {
    /// Validação base: nomes, temporalidade, consistência hierárquica, contactos.
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.short_name.trim().is_empty() {
            return Err(OrgError::EmptyField("short_name".into()));
        }
        if self.full_name.trim().is_empty() {
            return Err(OrgError::EmptyField("full_name".into()));
        }
        if let Some(until) = self.valid_until {
            if until <= self.valid_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        match (self.level.0, &self.parent_id) {
            (1, Some(_)) => return Err(OrgError::InconsistentLevel),
            (n, None) if n > 1 => return Err(OrgError::InconsistentLevel),
            _ => {}
        }
        self.contacts.validate()?;
        Ok(())
    }

    /// Validação estrita (modo operacional): exige `created_by` ou `legal_reference`.
    /// Usar em `create`/`update` via serviço. `import` usa `validate()` directamente.
    pub fn validate_strict(&self) -> Result<(), OrgError> {
        self.validate()?;
        if self.created_by.is_none() {
            let has_ref = self
                .legal_reference
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !has_ref {
                return Err(OrgError::EmptyField(
                    "created_by ou legal_reference obrigatório em modo operacional".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        matches!(self.status, OrgUnitStatus::Active)
            && date >= self.valid_from
            && self.valid_until.map_or(true, |u| date < u)
    }

    pub fn is_extinct(&self) -> bool {
        matches!(self.status, OrgUnitStatus::Extinct)
    }

    /// Valida a transição de status. Não muta o agregado.
    pub fn transition_status(&self, next: OrgUnitStatus) -> Result<OrgUnitStatus, OrgError> {
        if !self.status.can_transition_to(&next) {
            return Err(OrgError::OperationFailed(format!(
                "transição inválida: {} → {}",
                self.status.as_str(),
                next.as_str()
            )));
        }
        Ok(next)
    }

    /// Verifica que esta unidade não aparece na cadeia de ancestrais (ciclo).
    pub fn validate_parent_chain(&self, ancestors: &[&OrgUnitId]) -> Result<(), OrgError> {
        if ancestors.iter().any(|a| *a == &self.id) {
            return Err(OrgError::CircularHierarchy);
        }
        Ok(())
    }

    /// Verifica que o nível desta unidade é exactamente `parent.level + 1`.
    pub fn validate_level_against_parent(&self, parent: &OrgUnit) -> Result<(), OrgError> {
        match parent.level.0.checked_add(1) {
            Some(expected) if self.level.0 == expected => Ok(()),
            _ => Err(OrgError::InconsistentLevel),
        }
    }

    /// Verifica que a unidade pode ser desactivada.
    pub fn can_deactivate(
        &self,
        active_child_ids: &[&OrgUnitId],
        active_position_ids: &[&OrgPositionId],
    ) -> Result<(), OrgError> {
        if !active_child_ids.is_empty() {
            return Err(OrgError::CannotDeactivateWithActiveChildren);
        }
        if !active_position_ids.is_empty() {
            return Err(OrgError::CannotDeactivateWithActivePositions);
        }
        Ok(())
    }
}

// ── Helpers de validação de contactos ────────────────────────────────────────

fn is_email_valid(email: &str) -> bool {
    let parts: Vec<&str> = email.splitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn is_phone_valid(phone: &str) -> bool {
    let digits = phone.chars().filter(|c| c.is_ascii_digit()).count();
    digits >= 7
}

fn is_cp4_valid(cp4: &str) -> bool {
    cp4.len() == 4 && cp4.chars().all(|c| c.is_ascii_digit())
}

fn is_cp3_valid(cp3: &str) -> bool {
    cp3.len() == 3 && cp3.chars().all(|c| c.is_ascii_digit())
}

//! Unidade orgânica: hierarquia, validade temporal e máquina de estados de status.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{LegalInstrumentId, OrgError, OrgPositionId};

/// Nível hierárquico de uma unidade orgânica (1 = topo, máximo 5).
/// Nível 1 não tem pai. O pai de nível N tem sempre nível N-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgLevel(pub u8);

impl OrgLevel {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 5;

    pub fn new(n: u8) -> Result<Self, OrgError> {
        if (Self::MIN..=Self::MAX).contains(&n) {
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

    pub fn parse_canonical(s: &str) -> Result<Self, OrgError> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "extinct" => Ok(Self::Extinct),
            _ => Err(OrgError::OperationFailed(format!(
                "status desconhecido: {s}"
            ))),
        }
    }

    /// Transições válidas:
    /// - Active → Suspended | Extinct
    /// - Suspended → Active | Extinct
    /// - Extinct é estado terminal
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
        Self::parse_canonical(s)
    }
}

impl std::str::FromStr for OrgUnitStatus {
    type Err = OrgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_canonical(s)
    }
}

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrgContacts {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub address: OrgAddress,
}

/// Unidade orgânica com validade temporal.
///
/// `created_by` referencia o instrumento jurídico que criou ou alterou a unidade.
/// É opcional para permitir importação de dados históricos sem instrumento formal
/// registado; idealmente deve ser preenchido.
///
/// `legal_reference` preserva o texto livre da referência legal (ex: "Portaria n.º 150/2024")
/// para unidades sem instrumento formal registado no sistema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgUnit {
    pub id: OrgUnitId,
    /// Nome abreviado / identificador curto (ex: "SF Beja")
    pub short_name: String,
    /// Nome completo (ex: "Serviço de Finanças de Beja")
    pub full_name: String,
    /// Código de serviço externo (ex: "0312" para SF Beja na AT)
    pub service_code: Option<String>,
    pub level: OrgLevel,
    pub parent_id: Option<OrgUnitId>,
    pub contacts: OrgContacts,
    pub created_by: Option<LegalInstrumentId>,
    pub legal_reference: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub status: OrgUnitStatus,
}

impl OrgUnit {
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
            (2..=5, None) => return Err(OrgError::InconsistentLevel),
            _ => {}
        }
        Ok(())
    }

    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        matches!(self.status, OrgUnitStatus::Active)
            && date >= self.valid_from
            && self.valid_until.is_none_or(|u| date < u)
    }

    pub fn is_extinct(&self) -> bool {
        matches!(self.status, OrgUnitStatus::Extinct)
    }

    /// Valida a transição de status. O novo status não é aplicado ao agregado —
    /// cabe ao port persistir a mudança com o status devolvido.
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

    /// Verifica que esta unidade não aparece na cadeia de ancestrais fornecida
    /// (prevenção de hierarquia circular). A cadeia deve ser obtida pelo chamador
    /// antes de persistir a unidade.
    pub fn validate_parent_chain(&self, ancestors: &[&OrgUnitId]) -> Result<(), OrgError> {
        if ancestors.contains(&&self.id) {
            return Err(OrgError::CircularHierarchy);
        }
        Ok(())
    }

    /// Verifica que a unidade pode ser desactivada.
    /// O chamador deve fornecer os IDs das sub-unidades e posições ainda activas.
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

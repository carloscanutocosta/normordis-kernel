//! Instrumentos jurídicos que fundamentam alterações na estrutura orgânica.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::OrgError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LegalInstrumentId(pub String);

impl LegalInstrumentId {
    pub fn new(id: impl Into<String>) -> Result<Self, OrgError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(OrgError::EmptyField("instrument_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    Portaria,
    Despacho,
    Deliberacao,
    RegulamentoOrganico,
    Outro(String),
}

impl InstrumentKind {
    /// Serializa para a representação canónica de texto.
    /// Para `Outro(s)`, o formato é `"outro:{s}"` — idêntico ao que `org-sqlite` usa.
    pub fn as_str(&self) -> String {
        match self {
            Self::Portaria => "portaria".into(),
            Self::Despacho => "despacho".into(),
            Self::Deliberacao => "deliberacao".into(),
            Self::RegulamentoOrganico => "regulamento_organico".into(),
            Self::Outro(s) => format!("outro:{s}"),
        }
    }

    /// Desserializa da representação canónica. Para `"outro:{valor}"` devolve
    /// `Ok(Outro(valor))`; valores desconhecidos devolvem erro.
    pub fn parse_canonical(s: &str) -> Result<Self, OrgError> {
        match s {
            "portaria" => Ok(Self::Portaria),
            "despacho" => Ok(Self::Despacho),
            "deliberacao" => Ok(Self::Deliberacao),
            "regulamento_organico" => Ok(Self::RegulamentoOrganico),
            other if other.starts_with("outro:") => {
                Ok(Self::Outro(other["outro:".len()..].to_string()))
            }
            _ => Err(OrgError::OperationFailed(format!(
                "tipo de instrumento desconhecido: {s}"
            ))),
        }
    }
}

impl std::str::FromStr for InstrumentKind {
    type Err = OrgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_canonical(s)
    }
}

impl TryFrom<&str> for InstrumentKind {
    type Error = OrgError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse_canonical(s)
    }
}

/// Instrumento jurídico que fundamenta alterações na estrutura orgânica.
/// Toda a entidade com validade temporal em core-org deve referenciar
/// o instrumento que a criou ou modificou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalInstrument {
    pub id: LegalInstrumentId,
    pub kind: InstrumentKind,
    /// Referência oficial (ex: "Portaria n.º 123/2020, de 15 de abril")
    pub reference: String,
    pub date: NaiveDate,
    pub description: String,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
}

impl LegalInstrument {
    pub fn validate(&self) -> Result<(), OrgError> {
        if self.reference.trim().is_empty() {
            return Err(OrgError::EmptyField("reference".into()));
        }
        if self.description.trim().is_empty() {
            return Err(OrgError::EmptyField("description".into()));
        }
        if let Some(until) = self.effective_until {
            if until <= self.effective_from {
                return Err(OrgError::InvalidTemporalRange);
            }
        }
        Ok(())
    }

    pub fn is_effective_at(&self, date: NaiveDate) -> bool {
        date >= self.effective_from && self.effective_until.is_none_or(|u| date < u)
    }
}

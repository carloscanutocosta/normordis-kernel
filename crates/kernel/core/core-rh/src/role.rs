//! Papéis de utilizador: `UserRole` (sistema) e `Role` (aplicacional com id e nome).

use serde::{Deserialize, Serialize};

use crate::{validate_required_display_name, validate_role_id, RhError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    Utilizador,
    Auditor,
    Administrator,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Utilizador => "utilizador",
            Self::Auditor => "auditor",
            Self::Administrator => "administrator",
        }
    }

    /// Desserializa a partir do valor canónico exacto ("utilizador", "auditor",
    /// "administrator"). Para aceitar aliases ou input case-insensitive, usar `parse`.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "utilizador" => Some(Self::Utilizador),
            "auditor" => Some(Self::Auditor),
            "administrator" => Some(Self::Administrator),
            _ => None,
        }
    }

    /// Aceita aliases: "standard" → Utilizador, "supervisor" → Auditor.
    /// A comparação é case-insensitive.
    pub fn parse(value: &str) -> Result<Self, RhError> {
        match value.trim().to_lowercase().as_str() {
            "utilizador" | "standard" => Ok(Self::Utilizador),
            "auditor" | "supervisor" => Ok(Self::Auditor),
            "administrator" => Ok(Self::Administrator),
            _ => Err(RhError::InvalidRole),
        }
    }
}

impl TryFrom<&str> for UserRole {
    type Error = RhError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or(RhError::InvalidRole)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub role_id: String,
    pub display_name: String,
}

impl Role {
    pub fn new(
        role_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Result<Self, RhError> {
        let role = Self {
            role_id: role_id.into(),
            display_name: display_name.into(),
        };
        role.validate()?;
        Ok(role)
    }

    pub fn validate(&self) -> Result<(), RhError> {
        validate_role_id(&self.role_id)?;
        validate_required_display_name(
            "role_display_name",
            &self.display_name,
            RhError::InvalidRole,
        )
    }
}

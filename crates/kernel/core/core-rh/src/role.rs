//! Papéis de utilizador: `UserRole` (sistema) e catálogo gerido de roles funcionais.

use serde::{Deserialize, Serialize};

use crate::{validate_required_display_name, validate_role_id, RhError};

// ─── UserRole (sistema) ───────────────────────────────────────────────────────

/// Papel de sistema — granularidade coarse (acesso ao kernel e às operações base).
/// Não substituir por roles funcionais; os dois coexistem.
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

    /// Desserializa a partir do valor canónico exacto.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "utilizador" => Some(Self::Utilizador),
            "auditor" => Some(Self::Auditor),
            "administrator" => Some(Self::Administrator),
            _ => None,
        }
    }

    /// Aceita aliases: "standard" → Utilizador, "supervisor" → Auditor.
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

// ─── RoleId ───────────────────────────────────────────────────────────────────

/// Identificador único de um role funcional no catálogo institucional.
/// Não pode conter espaços em branco. Exemplos: "gestor_rh", "chefe_divisao".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(String);

impl RoleId {
    pub fn new(s: impl Into<String>) -> Result<Self, RhError> {
        let s = s.into();
        validate_role_id(&s)?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ─── Role (catálogo gerido) ───────────────────────────────────────────────────

/// Role funcional no catálogo institucional de roles.
///
/// Roles funcionais são geridos administrativamente e definem a que apps
/// um utilizador tem acesso no workspace. Distintos de `UserRole` (sistema).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

impl Role {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: Option<String>,
        is_active: bool,
    ) -> Result<Self, RhError> {
        let role = Self {
            id: RoleId::new(id)?,
            name: name.into(),
            description,
            is_active,
        };
        role.validate()?;
        Ok(role)
    }

    pub fn validate(&self) -> Result<(), RhError> {
        validate_required_display_name("role_name", &self.name, RhError::InvalidRole)
    }
}

// ─── RoleRepository (port) ────────────────────────────────────────────────────

/// Port do catálogo de roles funcionais.
/// Implementado por `rh-sqlite`; injectado onde seja necessário validar roles.
pub trait RoleRepository {
    /// O tipo de erro deve implementar `std::error::Error` (logo, `Display` + `Debug`),
    /// garantindo que os consumidores podem reportar falhas sem bounds adicionais.
    type Error: std::error::Error;

    /// Devolve o role, ou `None` se não existir.
    fn get(&self, id: &RoleId) -> Result<Option<Role>, Self::Error>;

    /// Lista todos os roles activos.
    fn list_active(&self) -> Result<Vec<Role>, Self::Error>;

    /// Verifica se o role existe E está activo.
    fn exists_and_active(&self, id: &RoleId) -> Result<bool, Self::Error>;

    /// Insere ou actualiza um role (idempotente).
    fn upsert(&self, role: &Role) -> Result<(), Self::Error>;

    /// Desactiva um role (não apaga — preserva histórico).
    fn deactivate(&self, id: &RoleId) -> Result<(), Self::Error>;
}

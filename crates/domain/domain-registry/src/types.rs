use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use core_rh::RoleId;

use crate::RegistryError;

/// Identificador único de uma app registada no workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppId(String);

impl AppId {
    pub fn new(s: impl Into<String>) -> Result<Self, RegistryError> {
        let s = s.into().trim().to_string();
        if s.is_empty() {
            return Err(RegistryError::EmptyField("app_id"));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Estado do ciclo de vida de uma app no catálogo institucional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    /// Em definição, não disponível para utilizadores.
    Draft,
    /// Disponível em ambiente controlado, em validação.
    Experimental,
    /// Em produção, disponível para uso geral.
    Active,
    /// Temporariamente indisponível.
    Suspended,
    /// Em fase de substituição, não recomendada para novos usos.
    Deprecated,
    /// Retirada de serviço definitivamente.
    Retired,
}

impl AppState {
    pub fn as_str(&self) -> &str {
        match self {
            AppState::Draft => "Draft",
            AppState::Experimental => "Experimental",
            AppState::Active => "Active",
            AppState::Suspended => "Suspended",
            AppState::Deprecated => "Deprecated",
            AppState::Retired => "Retired",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, RegistryError> {
        match s {
            "Draft" => Ok(AppState::Draft),
            "Experimental" => Ok(AppState::Experimental),
            "Active" => Ok(AppState::Active),
            "Suspended" => Ok(AppState::Suspended),
            "Deprecated" => Ok(AppState::Deprecated),
            "Retired" => Ok(AppState::Retired),
            other => Err(RegistryError::Storage(format!(
                "estado desconhecido: {other}"
            ))),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, AppState::Retired)
    }

    pub fn valid_transitions(&self) -> &[AppState] {
        match self {
            AppState::Draft => &[AppState::Experimental, AppState::Active, AppState::Retired],
            AppState::Experimental => &[AppState::Active, AppState::Suspended, AppState::Retired],
            AppState::Active => &[AppState::Suspended, AppState::Deprecated, AppState::Retired],
            AppState::Suspended => &[AppState::Active, AppState::Deprecated, AppState::Retired],
            AppState::Deprecated => &[AppState::Retired],
            AppState::Retired => &[],
        }
    }

    pub fn can_transition_to(&self, target: &AppState) -> bool {
        self.valid_transitions().contains(target)
    }
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Registo datado e imutável de uma transição de estado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateTransition {
    pub state: AppState,
    pub transitioned_at: DateTime<Utc>,
    pub transitioned_by: String,
    pub reason: Option<String>,
}

/// Visibilidade organizacional de uma app no workspace.
///
/// `Public` e `Internal` são classificações organizacionais.
/// O controlo de acesso operacional é feito por `allowed_roles`:
/// se não-vazio, apenas utilizadores com pelo menos um dos roles listados
/// vêem a app, independentemente da `AppVisibility`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppVisibility {
    /// Disponível a todos os utilizadores autenticados.
    Public,
    /// Disponível apenas a utilizadores internos.
    Internal,
}

impl AppVisibility {
    pub fn as_str(&self) -> &str {
        match self {
            AppVisibility::Public => "Public",
            AppVisibility::Internal => "Internal",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, RegistryError> {
        match s {
            "Public" => Ok(AppVisibility::Public),
            "Internal" => Ok(AppVisibility::Internal),
            other => Err(RegistryError::Storage(format!(
                "visibilidade desconhecida: {other}"
            ))),
        }
    }
}

/// Registo completo de uma app no catálogo institucional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRegistration {
    pub id: AppId,
    pub name: String,
    pub version: String,
    pub owner: String,
    pub domain: String,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub visibility: AppVisibility,
    /// Roles que têm acesso a esta app.
    /// Vazio = todos os utilizadores (sujeito a `AppVisibility`).
    /// Não-vazio = só utilizadores com pelo menos um dos roles listados.
    pub allowed_roles: Vec<RoleId>,
    pub registered_at: DateTime<Utc>,
    pub registered_by: String,
    /// Histórico de estados ordenado por `transitioned_at` ASC.
    pub state_history: Vec<AppStateTransition>,
}

impl AppRegistration {
    pub fn current_state(&self) -> Option<&AppState> {
        self.state_history.last().map(|t| &t.state)
    }

    pub fn is_active(&self) -> bool {
        matches!(self.current_state(), Some(AppState::Active))
    }

    /// Uma app é acessível a este utilizador se:
    /// - `allowed_roles` estiver vazio (acesso livre), OU
    /// - o utilizador tiver pelo menos um dos roles listados.
    pub fn is_accessible_to(&self, user_roles: &[RoleId]) -> bool {
        if self.allowed_roles.is_empty() {
            return true;
        }
        user_roles.iter().any(|r| self.allowed_roles.contains(r))
    }
}

/// Pedido de registo de uma nova app no catálogo.
#[derive(Debug, Clone)]
pub struct RegisterAppRequest {
    pub id: AppId,
    pub name: String,
    pub version: String,
    pub owner: String,
    pub domain: String,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub visibility: AppVisibility,
    /// Roles com acesso a esta app. Vazio = acesso livre.
    pub allowed_roles: Vec<RoleId>,
    pub registered_by: String,
}

impl RegisterAppRequest {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.name.trim().is_empty() {
            return Err(RegistryError::EmptyField("name"));
        }
        if self.version.trim().is_empty() {
            return Err(RegistryError::EmptyField("version"));
        }
        if self.owner.trim().is_empty() {
            return Err(RegistryError::EmptyField("owner"));
        }
        if self.domain.trim().is_empty() {
            return Err(RegistryError::EmptyField("domain"));
        }
        if self.registered_by.trim().is_empty() {
            return Err(RegistryError::EmptyField("registered_by"));
        }
        Ok(())
    }
}

/// Pedido de transição de estado de uma app registada.
#[derive(Debug, Clone)]
pub struct TransitionStateRequest {
    pub app_id: AppId,
    pub to_state: AppState,
    pub transitioned_by: String,
    pub reason: Option<String>,
}

impl TransitionStateRequest {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.transitioned_by.trim().is_empty() {
            return Err(RegistryError::EmptyField("transitioned_by"));
        }
        Ok(())
    }
}

/// Pedido de actualização de metadados de uma app registada.
/// Apenas os campos `Some(...)` são actualizados; `None` = sem alteração.
#[derive(Debug, Clone)]
pub struct UpdateAppMetadataRequest {
    pub app_id: AppId,
    pub version: Option<String>,
    /// `Some(None)` limpa o campo; `Some(Some(s))` actualiza.
    pub description: Option<Option<String>>,
    pub capabilities: Option<Vec<String>>,
    pub visibility: Option<AppVisibility>,
    pub owner: Option<String>,
    pub updated_by: String,
}

impl UpdateAppMetadataRequest {
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.updated_by.trim().is_empty() {
            return Err(RegistryError::EmptyField("updated_by"));
        }
        if let Some(v) = &self.version {
            if v.trim().is_empty() {
                return Err(RegistryError::EmptyField("version"));
            }
        }
        if let Some(o) = &self.owner {
            if o.trim().is_empty() {
                return Err(RegistryError::EmptyField("owner"));
            }
        }
        Ok(())
    }
}

/// Filtro para listagem de apps registadas. Campos `None` não aplicam filtro.
#[derive(Debug, Clone, Default)]
pub struct AppRegistryFilter {
    pub state: Option<AppState>,
    pub domain: Option<String>,
    pub owner: Option<String>,
    pub visibility: Option<AppVisibility>,
    /// Filtra por substring do nome (case-insensitive).
    pub name_contains: Option<String>,
}

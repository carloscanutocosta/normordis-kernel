use chrono::{DateTime, Utc};

use crate::{
    AppId, AppRegistration, AppRegistryFilter, AppStateTransition, RegisterAppRequest,
    RegistryError, RoleId, TransitionStateRequest, UpdateAppMetadataRequest,
};

pub trait AppRegistryRepository {
    type Error: From<RegistryError>;

    /// Regista uma nova app no catálogo (atomicamente com o estado Draft inicial).
    fn register(
        &self,
        request: &RegisterAppRequest,
        registered_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;

    /// Persiste uma transição de estado.
    fn transition(
        &self,
        request: &TransitionStateRequest,
        transitioned_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;

    /// Actualiza os metadados de uma app. Apenas campos `Some(...)` são alterados.
    /// Cada campo efectivamente alterado é registado no audit trail de metadados.
    fn update_metadata(
        &self,
        request: &UpdateAppMetadataRequest,
        updated_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;

    /// Substitui a lista de roles com acesso à app (atomicamente).
    /// Uma lista vazia remove todas as restrições de role.
    fn set_allowed_roles(
        &self,
        app_id: &AppId,
        roles: &[RoleId],
        set_by: &str,
        set_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;

    /// Apps visíveis para um utilizador com estes roles.
    /// Inclui sempre apps sem restrição de role (allowed_roles vazio).
    /// Se `roles` estiver vazio, devolve apenas apps sem restrição.
    fn list_for_roles(
        &self,
        roles: &[RoleId],
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error>;

    /// Devolve o registo completo com histórico de estados, ou `None` se não existir.
    fn get(&self, id: &AppId) -> Result<Option<AppRegistration>, Self::Error>;

    /// Lista apps que satisfazem o filtro.
    fn list(
        &self,
        filter: &AppRegistryFilter,
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error>;

    /// Devolve o histórico de estados, ordenado por `transitioned_at` ASC.
    fn state_history(&self, id: &AppId) -> Result<Vec<AppStateTransition>, Self::Error>;

    /// Verifica se uma app com o dado `id` já existe no catálogo.
    fn exists(&self, id: &AppId) -> Result<bool, Self::Error>;
}

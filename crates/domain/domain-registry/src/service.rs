use chrono::{DateTime, Utc};

use core_rh::RoleRepository;

use crate::{
    AppId, AppRegistration, AppRegistryFilter, AppRegistryRepository,
    AppState, AppStateTransition, RegisterAppRequest, RegistryError,
    RoleId, TransitionStateRequest, UpdateAppMetadataRequest,
};

/// Ponto de entrada único para operações sobre o catálogo institucional de apps.
///
/// Genérico sobre `R: AppRegistryRepository` (catálogo) e `L: RoleRepository`
/// (catálogo de roles, usado para validação). O repositório não re-valida.
pub struct AppRegistryService<R, L>
where
    R: AppRegistryRepository,
    L: RoleRepository,
{
    repo:  R,
    roles: L,
}

impl<R, L> AppRegistryService<R, L>
where
    R: AppRegistryRepository,
    L: RoleRepository,
{
    pub fn new(repo: R, roles: L) -> Self {
        Self { repo, roles }
    }

    /// Valida que todos os `RoleId` existem no catálogo e estão activos.
    /// Distingue role inexistente (`RoleNotFound`) de role inactivo (`RoleInactive`).
    fn validate_roles(&self, role_ids: &[RoleId]) -> Result<(), R::Error> {
        for id in role_ids {
            let role = self.roles.get(id).map_err(|e| {
                R::Error::from(RegistryError::Storage(format!(
                    "erro ao validar role '{}': {e}",
                    id.as_str()
                )))
            })?;
            match role {
                None => {
                    return Err(RegistryError::RoleNotFound(id.as_str().to_owned()).into());
                }
                Some(r) if !r.is_active => {
                    return Err(RegistryError::RoleInactive(id.as_str().to_owned()).into());
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Regista uma nova app. O estado inicial é sempre `Draft`.
    pub fn register(
        &self,
        request: RegisterAppRequest,
        now: DateTime<Utc>,
    ) -> Result<(), R::Error> {
        request.validate()?;
        self.validate_roles(&request.allowed_roles)?;
        if self.repo.exists(&request.id)? {
            return Err(
                RegistryError::AppAlreadyRegistered(request.id.as_str().to_owned()).into(),
            );
        }
        self.repo.register(&request, now)
    }

    /// Transita para um novo estado, validando a máquina de estados.
    pub fn transition(
        &self,
        request: TransitionStateRequest,
        now: DateTime<Utc>,
    ) -> Result<(), R::Error> {
        request.validate()?;

        let app = self.repo.get(&request.app_id)?.ok_or_else(|| {
            R::Error::from(RegistryError::AppNotFound(request.app_id.as_str().to_owned()))
        })?;

        let current = app.current_state().ok_or_else(|| {
            R::Error::from(RegistryError::Storage("app sem estado inicial".into()))
        })?;

        if current.is_terminal() {
            return Err(RegistryError::TerminalState(current.as_str().to_owned()).into());
        }
        if !current.can_transition_to(&request.to_state) {
            return Err(RegistryError::InvalidStateTransition {
                from: current.as_str().to_owned(),
                to:   request.to_state.as_str().to_owned(),
            }
            .into());
        }

        self.repo.transition(&request, now)
    }

    /// Actualiza os metadados de uma app registada.
    /// Cada campo efectivamente alterado é registado no audit trail de metadados.
    pub fn update_metadata(
        &self,
        request: UpdateAppMetadataRequest,
        now: DateTime<Utc>,
    ) -> Result<(), R::Error> {
        request.validate()?;
        if !self.repo.exists(&request.app_id)? {
            return Err(
                RegistryError::AppNotFound(request.app_id.as_str().to_owned()).into(),
            );
        }
        self.repo.update_metadata(&request, now)
    }

    /// Define os roles com acesso à app, substituindo a lista existente.
    /// Todos os roles são validados contra o catálogo antes de persistir.
    pub fn set_allowed_roles(
        &self,
        app_id: AppId,
        roles: Vec<RoleId>,
        set_by: &str,
        now: DateTime<Utc>,
    ) -> Result<(), R::Error> {
        if set_by.trim().is_empty() {
            return Err(RegistryError::EmptyField("set_by").into());
        }
        if !self.repo.exists(&app_id)? {
            return Err(RegistryError::AppNotFound(app_id.as_str().to_owned()).into());
        }
        self.validate_roles(&roles)?;
        self.repo.set_allowed_roles(&app_id, &roles, set_by, now)
    }

    /// Apps visíveis para um utilizador com estes roles.
    pub fn list_for_roles(
        &self,
        roles: &[RoleId],
        limit: usize,
    ) -> Result<Vec<AppRegistration>, R::Error> {
        self.repo.list_for_roles(roles, limit)
    }

    pub fn get(&self, id: &AppId) -> Result<Option<AppRegistration>, R::Error> {
        self.repo.get(id)
    }

    pub fn list_active(&self, limit: usize) -> Result<Vec<AppRegistration>, R::Error> {
        self.repo.list(
            &AppRegistryFilter {
                state: Some(AppState::Active),
                ..Default::default()
            },
            limit,
        )
    }

    pub fn list(
        &self,
        filter: &AppRegistryFilter,
        limit: usize,
    ) -> Result<Vec<AppRegistration>, R::Error> {
        self.repo.list(filter, limit)
    }

    pub fn history(
        &self,
        id: &AppId,
    ) -> Result<Vec<AppStateTransition>, R::Error> {
        self.repo.state_history(id)
    }
}

//! Identidade operacional do utilizador autenticado e contexto de sessão corrente.

use serde::{Deserialize, Serialize};

use crate::{
    audit_actor_from_user, validate_optional_email, validate_required_display_name,
    validate_username, OrgPositionRef, PersonAssignment, RhError, UserId, UserProfile, UserRole,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserIdentity {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: UserRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserContext {
    pub current_user: UserIdentity,
    /// Posição orgânica activa na sessão. `None` em contextos sem posição
    /// (ex: utilizador administrativo sem cargo, sessão de leitura).
    /// Obrigatório para construir `AuthorityContext` e finalizar documentos.
    pub org_position: Option<OrgPositionRef>,
}

impl UserContext {
    /// Associa uma posição orgânica ao contexto. Builder fluente — permite
    /// `resolve_current_user(identity)?.with_position(pos)`.
    pub fn with_position(mut self, pos: OrgPositionRef) -> Self {
        self.org_position = Some(pos);
        self
    }

    /// Constrói um `UserContext` com a posição orgânica derivada de um
    /// `PersonAssignment` institucional — grunda a sessão no registo COSO.
    ///
    /// `competency_id` e `delegation_id` são fornecidos pelo chamador porque
    /// não estão na afetação (essa informação está em `core-org`).
    pub fn from_assignment(
        identity: UserIdentity,
        assignment: &PersonAssignment,
        competency_id: impl Into<String>,
        delegation_id: Option<String>,
    ) -> Result<Self, RhError> {
        identity.validate()?;
        let org_position = OrgPositionRef::new(
            &assignment.position_id,
            &assignment.unit_id,
            competency_id,
            delegation_id,
        )?;
        Ok(Self {
            current_user: identity,
            org_position: Some(org_position),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorMetadata {
    pub actor_id: String,
    pub actor_name: String,
}

impl UserIdentity {
    pub fn validate(&self) -> Result<(), RhError> {
        UserId::new(self.user_id.clone())?;
        validate_username(&self.username)?;
        validate_required_display_name(
            "display_name",
            &self.display_name,
            RhError::InvalidProfile,
        )?;
        validate_optional_email(self.email.as_deref())?;
        Ok(())
    }

    pub fn author_metadata(&self) -> AuthorMetadata {
        AuthorMetadata {
            actor_id: self.user_id.clone(),
            actor_name: self.display_name.clone(),
        }
    }

    pub fn to_profile(&self) -> Result<UserProfile, RhError> {
        UserProfile::new(
            UserId::new(self.user_id.clone())?,
            self.username.clone(),
            self.display_name.clone(),
            self.email.clone(),
            self.role,
            Vec::new(),
            None,
        )
    }

    pub fn audit_actor(&self) -> Result<core_audit::AuditActor, RhError> {
        Ok(audit_actor_from_user(&self.to_profile()?))
    }
}

impl TryFrom<UserProfile> for UserIdentity {
    type Error = RhError;

    fn try_from(value: UserProfile) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self {
            user_id: value.user_id.as_str().to_owned(),
            username: value.username,
            display_name: value.display_name,
            email: value.email,
            role: value.user_role,
        })
    }
}

pub fn resolve_current_user(user: UserIdentity) -> Result<UserContext, RhError> {
    user.validate()?;
    Ok(UserContext {
        current_user: user,
        org_position: None,
    })
}

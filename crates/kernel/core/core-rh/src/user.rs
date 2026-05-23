//! Identificador de utilizador (`UserId`) e perfil completo (`UserProfile`).

use serde::{Deserialize, Serialize};

use crate::{
    validate_optional_email, validate_required_display_name, validate_user_id_value,
    validate_username, OrgUnitRef, RhError, Role, UserRole,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(String);

impl UserId {
    pub fn new(value: impl Into<String>) -> Result<Self, RhError> {
        let value = value.into();
        validate_user_id_value(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: UserId,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub user_role: UserRole,
    pub roles: Vec<Role>,
    pub org_unit: Option<OrgUnitRef>,
}

impl UserProfile {
    pub fn new(
        user_id: UserId,
        username: impl Into<String>,
        display_name: impl Into<String>,
        email: Option<String>,
        user_role: UserRole,
        roles: Vec<Role>,
        org_unit: Option<OrgUnitRef>,
    ) -> Result<Self, RhError> {
        let profile = Self {
            user_id,
            username: username.into(),
            display_name: display_name.into(),
            email,
            user_role,
            roles,
            org_unit,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), RhError> {
        validate_username(&self.username)?;
        validate_required_display_name(
            "display_name",
            &self.display_name,
            RhError::InvalidProfile,
        )?;
        validate_optional_email(self.email.as_deref())?;

        for role in &self.roles {
            role.validate().map_err(|_| RhError::InvalidProfile)?;
        }

        if let Some(org_unit) = &self.org_unit {
            org_unit.validate().map_err(|_| RhError::InvalidProfile)?;
        }

        Ok(())
    }
}

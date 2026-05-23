//! Sessão corrente do utilizador autenticado e wrapper `CurrentUser`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{RhError, UserId, UserProfile};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub profile: UserProfile,
}

impl CurrentUser {
    pub fn new(profile: UserProfile) -> Result<Self, RhError> {
        profile.validate()?;
        Ok(Self { profile })
    }

    pub fn user_id(&self) -> &UserId {
        &self.profile.user_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentSession {
    pub session_id: Uuid,
    pub started_at_utc: DateTime<Utc>,
    pub user: UserProfile,
}

impl CurrentSession {
    pub fn new(user: UserProfile) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            started_at_utc: Utc::now(),
            user,
        }
    }

    pub fn validate(&self) -> Result<(), RhError> {
        if self.session_id.is_nil() {
            return Err(RhError::InvalidSession);
        }

        self.user.validate().map_err(|_| RhError::InvalidSession)
    }
}

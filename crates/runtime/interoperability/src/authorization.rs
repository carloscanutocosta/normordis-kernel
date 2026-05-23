use serde::{Deserialize, Serialize};

use core_exports::ExportMaterializationRequest;

use crate::{InteroperabilityError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportAuthorizationContext {
    pub actor: String,
    pub purpose: String,
    pub correlation_id: String,
}

impl ExportAuthorizationContext {
    pub fn validate(&self) -> Result<()> {
        if self.actor.trim().is_empty() {
            return Err(InteroperabilityError::EmptyField("actor"));
        }
        if self.purpose.trim().is_empty() {
            return Err(InteroperabilityError::EmptyField("purpose"));
        }
        if self.correlation_id.trim().is_empty() {
            return Err(InteroperabilityError::EmptyField("correlation_id"));
        }
        Ok(())
    }
}

pub trait ExportAuthorizationPolicy {
    fn authorize(
        &self,
        ctx: &ExportAuthorizationContext,
        request: &ExportMaterializationRequest,
    ) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAllExportAuthorization;

impl ExportAuthorizationPolicy for AllowAllExportAuthorization {
    fn authorize(
        &self,
        _ctx: &ExportAuthorizationContext,
        _request: &ExportMaterializationRequest,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAllExportAuthorization;

impl ExportAuthorizationPolicy for DenyAllExportAuthorization {
    fn authorize(
        &self,
        _ctx: &ExportAuthorizationContext,
        _request: &ExportMaterializationRequest,
    ) -> Result<()> {
        Err(InteroperabilityError::Unauthorized(
            "policy deny-all".into(),
        ))
    }
}

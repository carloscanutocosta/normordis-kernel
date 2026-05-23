//! Referências leves a entidades orgânicas — sem dependência directa de `core-org`.
//!
//! Usam `String` IDs internamente para preservar a independência de `core-org`.
//! A conversão para os newtypes fortes de `core-org` (ex: `OrgPositionId`,
//! `CompetencyId`) é responsabilidade de `core-documental` ou do service layer.

use serde::{Deserialize, Serialize};

use crate::{validate_competency_id, validate_org_unit_id, validate_position_id, RhError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgUnitRef {
    pub org_unit_id: String,
    pub display_name: Option<String>,
}

impl OrgUnitRef {
    pub fn new(
        org_unit_id: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<Self, RhError> {
        let org_ref = Self {
            org_unit_id: org_unit_id.into(),
            display_name,
        };
        org_ref.validate()?;
        Ok(org_ref)
    }

    pub fn validate(&self) -> Result<(), RhError> {
        validate_org_unit_id(&self.org_unit_id)
    }
}

/// Referência leve à posição orgânica activa do utilizador na sessão corrente.
///
/// Captura os IDs necessários para construir um `AuthorityContext` no momento
/// de finalização de documentos. Usa `String` para não depender de `core-org`;
/// a conversão para newtypes fortes é feita por `AuthorityContext::from_user_context`
/// em `core-documental`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgPositionRef {
    /// ID da posição orgânica (cargo) em que o utilizador está a actuar.
    pub position_id: String,
    /// ID da unidade orgânica a que esta posição pertence.
    pub unit_id: String,
    /// ID da competência primária exercida nesta sessão.
    pub competency_id: String,
    /// ID da delegação, se o utilizador estiver a actuar por delegação.
    pub delegation_id: Option<String>,
}

impl OrgPositionRef {
    pub fn new(
        position_id: impl Into<String>,
        unit_id: impl Into<String>,
        competency_id: impl Into<String>,
        delegation_id: Option<String>,
    ) -> Result<Self, RhError> {
        let r = Self {
            position_id: position_id.into(),
            unit_id: unit_id.into(),
            competency_id: competency_id.into(),
            delegation_id,
        };
        r.validate()?;
        Ok(r)
    }

    pub fn validate(&self) -> Result<(), RhError> {
        validate_position_id(&self.position_id)?;
        validate_org_unit_id(&self.unit_id)?;
        validate_competency_id(&self.competency_id)?;
        if let Some(del) = &self.delegation_id {
            if del.trim().is_empty() {
                return Err(RhError::InvalidOrgRef);
            }
        }
        Ok(())
    }
}

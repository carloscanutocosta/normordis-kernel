//! Snapshot jurídico de autoridade — capturado imutavelmente no momento de
//! finalização do documento para reconstituição histórica.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use core_org::{CompetencyId, DelegationId, OrgPositionId, OrgUnitId};
use core_rh::{UserContext, UserId};

use crate::DocumentalError;

/// Snapshot jurídico imutável capturado no momento de finalização do documento.
///
/// Congela quem (utilizador/pessoa), em que posição (cargo), na unidade orgânica,
/// com que competência e ao abrigo de que instrumento (por delegação ou não).
/// Este snapshot preserva a autoridade jurídica para reconstituição histórica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityContext {
    pub user_id: UserId,
    pub position_id: OrgPositionId,
    pub unit_id: OrgUnitId,
    pub competency_id: CompetencyId,
    pub delegation_id: Option<DelegationId>,
    pub captured_at: DateTime<Utc>,
}

impl AuthorityContext {
    /// Constrói um snapshot de autoridade a partir do `UserContext` corrente.
    ///
    /// Requer que `ctx.org_position` esteja preenchido — retorna
    /// `MissingAuthorityContext` se o utilizador não tiver posição activa na sessão.
    /// `captured_at` deve ser o instante do acto de finalização (normalmente `Utc::now()`).
    pub fn from_user_context(
        ctx: &UserContext,
        captured_at: DateTime<Utc>,
    ) -> Result<Self, DocumentalError> {
        let pos = ctx
            .org_position
            .as_ref()
            .ok_or(DocumentalError::MissingAuthorityContext)?;

        let err = |field: &str| {
            DocumentalError::OperationFailed(format!("{field} inválido no UserContext"))
        };

        let user_id = UserId::new(ctx.current_user.user_id.clone()).map_err(|_| err("user_id"))?;
        let position_id =
            OrgPositionId::new(pos.position_id.clone()).map_err(|_| err("position_id"))?;
        let unit_id = OrgUnitId::new(pos.unit_id.clone()).map_err(|_| err("unit_id"))?;
        let competency_id =
            CompetencyId::new(pos.competency_id.clone()).map_err(|_| err("competency_id"))?;
        let delegation_id = pos
            .delegation_id
            .as_deref()
            .map(|id| DelegationId::new(id).map_err(|_| err("delegation_id")))
            .transpose()?;

        Ok(Self {
            user_id,
            position_id,
            unit_id,
            competency_id,
            delegation_id,
            captured_at,
        })
    }
}

//! Delegações temporárias de permissão e revogação de políticas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SecurityError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DelegationId(pub String);

impl DelegationId {
    pub fn new(id: impl Into<String>) -> Result<Self, SecurityError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SecurityError::MissingField("delegation_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationRequest {
    pub policy_id: String,
    pub revoked_by: String,
    pub reason: Option<String>,
}

impl RevocationRequest {
    pub fn validate(&self) -> Result<(), SecurityError> {
        if self.policy_id.trim().is_empty() {
            return Err(SecurityError::MissingField("policy_id".into()));
        }
        if self.revoked_by.trim().is_empty() {
            return Err(SecurityError::MissingField("revoked_by".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub granted_by: String,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    /// Condições adicionais opcionais (JSON, CEL, etc.).
    /// **Reservado para uso futuro — não avaliado pelo motor de autorização.**
    /// Qualquer valor aqui é armazenado e devolvido mas não tem efeito na decisão de acesso.
    pub conditions: Option<String>,
    /// Delegação-pai de que este pedido deriva, para rastreabilidade de cadeia
    /// e revogação em cascata. `None` = delegação raiz (concedida directamente
    /// por um principal do sistema ou em bootstrap).
    ///
    /// Quando criado via `SecurityService::grant_delegation()`, este campo é
    /// preenchido automaticamente pelo serviço com base na delegação do granter.
    pub granted_via: Option<DelegationId>,
}

impl DelegationRequest {
    pub fn validate(&self) -> Result<(), SecurityError> {
        if self.principal.trim().is_empty() {
            return Err(SecurityError::MissingField("principal".into()));
        }
        if self.operation.trim().is_empty() {
            return Err(SecurityError::MissingField("operation".into()));
        }
        if self.granted_by.trim().is_empty() {
            return Err(SecurityError::MissingField("granted_by".into()));
        }
        if self.valid_to <= self.valid_from {
            return Err(SecurityError::InvalidDelegation(
                "valid_to não pode ser anterior ou igual a valid_from".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub delegation_id: DelegationId,
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    /// Condições adicionais opcionais (JSON, CEL, etc.).
    /// **Reservado para uso futuro — não avaliado pelo motor de autorização.**
    pub conditions: Option<String>,
    pub revoked: bool,
    /// Delegação-pai da qual esta deriva. `None` = delegação raiz.
    /// Usado para revogação em cascata: revogar o pai revoga automaticamente
    /// todos os descendentes.
    pub granted_via: Option<DelegationId>,
}

impl Delegation {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        !self.revoked && now >= self.valid_from && now < self.valid_to
    }
}

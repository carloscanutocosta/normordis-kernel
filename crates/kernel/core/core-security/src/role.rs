//! Roles de segurança e port de pertença a roles.
//!
//! ## Separação de responsabilidades
//!
//! `core-rh` gere a estrutura organizacional (cargos, lotações, contratos).
//! `core-security` gere a autorização — quem pode fazer o quê.
//!
//! A ligação entre os dois domínios é feita pelo adapter `rh-security-bridge`:
//! - Mantém uma tabela `security_role_members` gerida pelo administrador de segurança
//! - Implementa `RoleMembershipRepository`, devolvendo os roles de cada principal
//! - É substituível — amanhã pode consultar LDAP, AD ou o próprio core-rh
//!
//! ## Convenção de nomenclatura de RoleId
//!
//! Recomendado: `"role:<nome>"` (ex: `"role:editor"`, `"role:auditor"`).
//! Não é imposto pelo tipo — o domínio decide a convenção.
//! Os role IDs são armazenados como `principal` nas delegações existentes,
//! o que significa que uma delegação `principal="role:editor"` aplica-se a
//! todos os utilizadores com esse role no momento da verificação.

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SecurityError;

/// Identificador de um role de segurança.
///
/// Convenção recomendada: `"role:<nome>"`. O tipo não valida o prefixo —
/// use `new()` apenas para garantir que a string não está vazia.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn new(id: impl Into<String>) -> Result<Self, SecurityError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SecurityError::MissingField("role_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Port para consulta de pertença a roles de segurança.
///
/// Implementações:
/// - `NoopRoleMembership` — devolve sempre lista vazia (sem overhead)
/// - `InMemoryRoleMembership` — útil em testes
/// - `RhSecurityBridgeStore` (crate `rh-security-bridge`) — persiste em SQLite
#[allow(async_fn_in_trait)]
pub trait RoleMembershipRepository {
    /// Devolve os roles activos do `principal_id` no momento `now`.
    async fn get_roles_for_principal(
        &self,
        principal_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<RoleId>, SecurityError>;
}

/// Implementação nula — nenhum principal tem roles.
///
/// Usar quando roles não são necessários (ex: mini-apps simples).
pub struct NoopRoleMembership;

impl RoleMembershipRepository for NoopRoleMembership {
    async fn get_roles_for_principal(
        &self,
        _principal_id: &str,
        _now: DateTime<Utc>,
    ) -> Result<Vec<RoleId>, SecurityError> {
        Ok(vec![])
    }
}

/// Implementação em memória — útil em testes de integração.
///
/// Atribuições não têm data de validade — valem enquanto estiverem registadas.
/// Para validade temporal usar `RhSecurityBridgeStore`.
pub struct InMemoryRoleMembership {
    memberships: RwLock<HashMap<String, Vec<RoleId>>>,
}

impl InMemoryRoleMembership {
    pub fn new() -> Self {
        Self {
            memberships: RwLock::new(HashMap::new()),
        }
    }

    /// Atribui `role` ao `principal`. Idempotente se o role já existir.
    pub fn assign(&self, principal: &str, role: RoleId) {
        let mut guard = self.memberships.write().unwrap();
        let roles = guard.entry(principal.to_string()).or_default();
        if !roles.contains(&role) {
            roles.push(role);
        }
    }

    /// Remove `role` do `principal`.
    pub fn revoke(&self, principal: &str, role: &RoleId) {
        let mut guard = self.memberships.write().unwrap();
        if let Some(roles) = guard.get_mut(principal) {
            roles.retain(|r| r != role);
        }
    }
}

impl Default for InMemoryRoleMembership {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleMembershipRepository for InMemoryRoleMembership {
    async fn get_roles_for_principal(
        &self,
        principal_id: &str,
        _now: DateTime<Utc>,
    ) -> Result<Vec<RoleId>, SecurityError> {
        let guard = self.memberships.read().unwrap();
        Ok(guard.get(principal_id).cloned().unwrap_or_default())
    }
}

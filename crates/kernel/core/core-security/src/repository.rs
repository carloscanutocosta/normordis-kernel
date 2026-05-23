//! Port de persistência (hexagonal) para `core-security`.
//!
//! Todos os métodos são `async fn` — o crate não impõe um runtime; a escolha
//! (tokio, smol, async-std) é do adapter concreto e do caller. Em contextos síncronos
//! usar `futures::executor::block_on` ou o `spawn_blocking` do runtime adequado.

use chrono::{DateTime, Utc};

use crate::{
    Delegation, DelegationId, DelegationRequest, ListOptions, Policy, RevocationRequest,
    SecurityError,
};

#[allow(async_fn_in_trait)]
pub trait SecurityPolicyRepository {
    /// Persiste uma nova política. Idempotente se `policy_id` + `version` já existirem.
    /// Falha com `AlreadyExists` se `policy_id` existir com `version` diferente.
    async fn save_policy(&self, policy: &Policy, now: DateTime<Utc>) -> Result<(), SecurityError>;

    async fn get_policy(&self, policy_id: &str) -> Result<Option<Policy>, SecurityError>;

    /// Retorna políticas não revogadas. `opts = None` → sem limite (retorna todas).
    async fn list_active_policies(
        &self,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Policy>, SecurityError>;

    /// Revogação append-only — a política é marcada como inactiva, não removida.
    async fn revoke_policy(
        &self,
        req: &RevocationRequest,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError>;

    /// Persiste uma delegação. Usar via `SecurityService::grant_delegation()` para verificação
    /// de autoridade; chamar directamente apenas em bootstrap ou operações administrativas.
    async fn delegate_permission(
        &self,
        req: &DelegationRequest,
        now: DateTime<Utc>,
    ) -> Result<Delegation, SecurityError>;

    /// Retorna delegações activas para `principal` no momento `now`.
    /// `opts = None` → sem limite (retorna todas).
    async fn list_delegations(
        &self,
        principal: &str,
        now: DateTime<Utc>,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Delegation>, SecurityError>;

    /// Revoga uma delegação e todos os seus descendentes (revogação em cascata).
    ///
    /// Usar quando um utilizador perde um cargo, sai da organização, ou a delegação
    /// precisa de ser retirada antes de `valid_to`. Falha com `DelegationNotFound`
    /// se a delegação não existir ou já estiver revogada.
    ///
    /// A cascata garante que sub-delegações concedidas via esta delegação ficam
    /// igualmente inactivas — fechando a superfície de delegações órfãs.
    async fn revoke_delegation(
        &self,
        delegation_id: &DelegationId,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError>;
}

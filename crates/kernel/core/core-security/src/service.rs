//! Motor de autorização de `core-security`.
//!
//! ## Lógica de decisão em `authorize()`
//!
//! ```text
//! Gate 1: validate_write_invariant()           ← campos não vazios, sempre
//! Gate 2a: delegação directa (principal, op, resource)?
//!           └─ Sim → AuthorizationToken(Delegation)
//! Gate 2b: roles do principal → delegação de role (role, op, resource)?
//!           └─ Sim → AuthorizationToken(RoleDelegation)
//! Gate 3: políticas activas?
//!          ├─ Nenhuma                           → AuthorizationToken(Bootstrap)
//!          ├─ Rule disabled para esta op        → AuthorizationToken(ExemptedByRule)
//!          ├─ Strict OU rule enabled para op    → Err(InvariantViolated)
//!          └─ Baseline, sem rule                → AuthorizationToken(BaselinePolicy)
//! ```
//!
//! Toda a decisão é registada no `SecurityAuditLog`.
//!
//! ## Genéricos
//!
//! `SecurityService<R, A, M>` é parametrizado por três tipos:
//! - `R: SecurityPolicyRepository` — repositório de políticas e delegações
//! - `A: SecurityAuditLog` — log de decisões; por omissão `NoopSecurityAuditLog`
//! - `M: RoleMembershipRepository` — pertença a roles; por omissão `NoopRoleMembership`
//!
//! ## Construtores
//!
//! ```ignore
//! // Simples (sem audit, sem roles):
//! let svc = SecurityService::new(repo);
//!
//! // Com audit:
//! let svc = SecurityService::with_audit(repo, audit_store);
//!
//! // Completo:
//! let svc = SecurityService::with_all(repo, audit_store, role_store);
//! ```

use chrono::{DateTime, Utc};

use crate::{
    audit_log::{AuditDecision, NoopSecurityAuditLog, SecurityAuditLog, SecurityAuthDecision},
    role::{NoopRoleMembership, RoleId, RoleMembershipRepository},
    validate_write_invariant, Delegation, DelegationId, DelegationRequest, Policy, PolicyMode,
    Rule, SecurityError, SecurityPolicyRepository, WriteInvariantContext,
};

/// Prova selada de que `SecurityService::authorize()` foi chamado com sucesso.
///
/// O campo privado `_sealed` impede a construção fora deste módulo (sela a prova
/// à passagem por `authorize`). É deliberadamente mais forte que `#[non_exhaustive]`,
/// que não impediria a construção dentro do próprio crate.
#[allow(clippy::manual_non_exhaustive)]
#[derive(Debug, Clone)]
pub struct AuthorizationToken {
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub granted_by: GrantedBy,
    pub at: DateTime<Utc>,
    _sealed: (),
}

/// Razão pela qual a autorização foi concedida.
#[derive(Debug, Clone)]
pub enum GrantedBy {
    /// Delegação explícita directa para (principal, operation, resource).
    Delegation(DelegationId),
    /// Delegação atribuída a um role do qual o principal é membro.
    RoleDelegation(RoleId, DelegationId),
    /// Modo baseline sem rule governando esta operação.
    BaselinePolicy,
    /// Operação explicitamente isenta por rule `enabled = false`.
    ExemptedByRule,
    /// Sem políticas activas — sistema em arranque.
    Bootstrap,
}

/// Motor de autorização de `core-security`.
pub struct SecurityService<R, A = NoopSecurityAuditLog, M = NoopRoleMembership> {
    repo: R,
    audit: A,
    roles: M,
}

// ── Construtores ──────────────────────────────────────────────────────────────

impl<R: SecurityPolicyRepository> SecurityService<R, NoopSecurityAuditLog, NoopRoleMembership> {
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            audit: NoopSecurityAuditLog,
            roles: NoopRoleMembership,
        }
    }
}

impl<R: SecurityPolicyRepository, A: SecurityAuditLog> SecurityService<R, A, NoopRoleMembership> {
    pub fn with_audit(repo: R, audit: A) -> Self {
        Self {
            repo,
            audit,
            roles: NoopRoleMembership,
        }
    }
}

impl<R: SecurityPolicyRepository, A: SecurityAuditLog, M: RoleMembershipRepository>
    SecurityService<R, A, M>
{
    pub fn with_all(repo: R, audit: A, roles: M) -> Self {
        Self { repo, audit, roles }
    }

    pub fn repo(&self) -> &R {
        &self.repo
    }

    pub fn audit_log(&self) -> &A {
        &self.audit
    }

    pub fn role_membership(&self) -> &M {
        &self.roles
    }

    // ── Autorização ───────────────────────────────────────────────────────────

    pub async fn authorize(
        &self,
        ctx: &WriteInvariantContext,
        resource: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<AuthorizationToken, SecurityError> {
        validate_write_invariant(ctx)?;

        let principal_id = ctx.principal.id();
        let result = self.do_authorize(ctx, resource, now, principal_id).await;

        let entry = match &result {
            Ok(token) => SecurityAuthDecision {
                logged_at: now,
                principal: principal_id.into(),
                operation: ctx.operation.clone(),
                resource: resource.map(String::from),
                correlation_id: ctx.correlation_id.clone(),
                decision: AuditDecision::Granted,
                granted_by_kind: Some(granted_by_kind(&token.granted_by)),
                deny_reason: None,
            },
            Err(err) => SecurityAuthDecision {
                logged_at: now,
                principal: principal_id.into(),
                operation: ctx.operation.clone(),
                resource: resource.map(String::from),
                correlation_id: ctx.correlation_id.clone(),
                decision: AuditDecision::Denied,
                granted_by_kind: None,
                deny_reason: Some(err.to_string()),
            },
        };
        if let Err(e) = self.audit.record_decision(&entry).await {
            eprintln!("[core-security] audit log failed: {e}");
        }

        result
    }

    async fn do_authorize(
        &self,
        ctx: &WriteInvariantContext,
        resource: Option<&str>,
        now: DateTime<Utc>,
        principal_id: &str,
    ) -> Result<AuthorizationToken, SecurityError> {
        // Gate 2a: delegação directa do principal
        let direct = self.repo.list_delegations(principal_id, now, None).await?;
        if let Some(d) = direct
            .iter()
            .find(|d| matches_delegation(d, &ctx.operation, resource))
        {
            return Ok(self.token(
                principal_id,
                &ctx.operation,
                resource,
                GrantedBy::Delegation(d.delegation_id.clone()),
                now,
            ));
        }

        // Gate 2b: delegação via role
        let user_roles = self
            .roles
            .get_roles_for_principal(principal_id, now)
            .await?;
        for role in &user_roles {
            let role_delegations = self.repo.list_delegations(role.as_str(), now, None).await?;
            if let Some(d) = role_delegations
                .iter()
                .find(|d| matches_delegation(d, &ctx.operation, resource))
            {
                return Ok(self.token(
                    principal_id,
                    &ctx.operation,
                    resource,
                    GrantedBy::RoleDelegation(role.clone(), d.delegation_id.clone()),
                    now,
                ));
            }
        }

        // Gate 3: políticas activas
        let policies = self.repo.list_active_policies(None).await?;

        if policies.is_empty() {
            return Ok(self.token(
                principal_id,
                &ctx.operation,
                resource,
                GrantedBy::Bootstrap,
                now,
            ));
        }

        let op_rule = find_operation_rule(&policies, &ctx.operation);

        if matches!(op_rule, Some(r) if !r.enabled) {
            return Ok(self.token(
                principal_id,
                &ctx.operation,
                resource,
                GrantedBy::ExemptedByRule,
                now,
            ));
        }

        let is_strict = policies.iter().any(|p| p.mode == PolicyMode::Strict);
        let op_is_governed = matches!(op_rule, Some(r) if r.enabled);

        if is_strict || op_is_governed {
            let reason = if is_strict {
                "modo strict activo"
            } else {
                "operação governada por regra de política"
            };
            let resource_info = resource.map(|r| format!(" em '{r}'")).unwrap_or_default();
            return Err(SecurityError::InvariantViolated(format!(
                "principal '{}' não tem delegação activa para '{}'{resource_info} ({reason})",
                principal_id, ctx.operation,
            )));
        }

        Ok(self.token(
            principal_id,
            &ctx.operation,
            resource,
            GrantedBy::BaselinePolicy,
            now,
        ))
    }

    // ── Delegação ─────────────────────────────────────────────────────────────

    pub async fn grant_delegation(
        &self,
        req: &DelegationRequest,
        granter: &WriteInvariantContext,
        now: DateTime<Utc>,
    ) -> Result<Delegation, SecurityError> {
        req.validate()?;
        validate_write_invariant(granter)?;

        let policies = self.repo.list_active_policies(None).await?;

        // Para principais humanos, verificar autoridade e recolher a delegação-pai
        // para propagar automaticamente em `granted_via`.
        let parent_id = if !policies.is_empty() && granter.principal.is_human() {
            match self
                .find_granting_delegation(
                    granter.principal.id(),
                    &req.operation,
                    req.resource.as_deref(),
                    now,
                )
                .await?
            {
                None => {
                    return Err(SecurityError::InvariantViolated(format!(
                        "principal '{}' não tem autoridade para delegar '{}' — não possui delegação activa para esta operação",
                        granter.principal.id(),
                        req.operation
                    )));
                }
                Some(id) => Some(id),
            }
        } else {
            None // principal sistema ou bootstrap — delegação raiz
        };

        // Forçar granted_by com a identidade real do granter (rastreabilidade)
        // e propagar granted_via se o caller não o definiu.
        let effective_req = DelegationRequest {
            granted_by: granter.principal.id().to_string(),
            granted_via: req.granted_via.clone().or(parent_id),
            ..req.clone()
        };

        self.repo.delegate_permission(&effective_req, now).await
    }

    pub async fn revoke_delegation(
        &self,
        delegation_id: &DelegationId,
        revoker: &WriteInvariantContext,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        validate_write_invariant(revoker)?;
        self.repo.revoke_delegation(delegation_id, now).await
    }

    /// Devolve a `DelegationId` da primeira delegação activa (directa ou via role)
    /// que cobre `(operation, resource)` para `principal_id`, ou `None` se não existir.
    ///
    /// Usado tanto para verificar autoridade em `grant_delegation()` como para
    /// propagar `granted_via` automaticamente na sub-delegação.
    async fn find_granting_delegation(
        &self,
        principal_id: &str,
        operation: &str,
        resource: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<Option<DelegationId>, SecurityError> {
        let direct = self.repo.list_delegations(principal_id, now, None).await?;
        if let Some(d) = direct
            .iter()
            .find(|d| matches_delegation(d, operation, resource))
        {
            return Ok(Some(d.delegation_id.clone()));
        }

        let user_roles = self
            .roles
            .get_roles_for_principal(principal_id, now)
            .await?;
        for role in &user_roles {
            let role_delegations = self.repo.list_delegations(role.as_str(), now, None).await?;
            if let Some(d) = role_delegations
                .iter()
                .find(|d| matches_delegation(d, operation, resource))
            {
                return Ok(Some(d.delegation_id.clone()));
            }
        }
        Ok(None)
    }

    fn token(
        &self,
        principal: &str,
        operation: &str,
        resource: Option<&str>,
        granted_by: GrantedBy,
        at: DateTime<Utc>,
    ) -> AuthorizationToken {
        AuthorizationToken {
            principal: principal.into(),
            operation: operation.into(),
            resource: resource.map(String::from),
            granted_by,
            at,
            _sealed: (),
        }
    }
}

// ── Funções auxiliares ────────────────────────────────────────────────────────

fn granted_by_kind(g: &GrantedBy) -> String {
    match g {
        GrantedBy::Delegation(id) => format!("delegation:{}", id.as_str()),
        GrantedBy::RoleDelegation(role, id) => {
            format!("role-delegation:{}:{}", role.as_str(), id.as_str())
        }
        GrantedBy::BaselinePolicy => "baseline".into(),
        GrantedBy::ExemptedByRule => "exempted".into(),
        GrantedBy::Bootstrap => "bootstrap".into(),
    }
}

fn matches_delegation(d: &Delegation, operation: &str, resource: Option<&str>) -> bool {
    if d.operation != operation {
        return false;
    }
    match (&d.resource, resource) {
        (None, _) => true,
        (Some(dr), Some(rr)) => dr == rr,
        (Some(_), None) => false,
    }
}

fn find_operation_rule<'a>(policies: &'a [Policy], operation: &str) -> Option<&'a Rule> {
    let mut found: Option<&'a Rule> = None;
    for p in policies {
        for r in &p.rules {
            if r.code == operation {
                match found {
                    None => found = Some(r),
                    Some(existing) if r.enabled && !existing.enabled => found = Some(r),
                    _ => {}
                }
            }
        }
    }
    found
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    use crate::{
        role::InMemoryRoleMembership, AuditDecision, DelegationRequest,
        InMemorySecurityPolicyRepository, Policy, PolicyMode, RevocationRequest, Rule,
        VerifiedPrincipal,
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap()
    }

    fn human_ctx(principal: &str, op: &str) -> WriteInvariantContext {
        WriteInvariantContext {
            operation: op.into(),
            correlation_id: "corr-test".into(),
            principal: VerifiedPrincipal::human(principal),
        }
    }

    fn system_ctx(name: &str, op: &str) -> WriteInvariantContext {
        WriteInvariantContext {
            operation: op.into(),
            correlation_id: "corr-sys".into(),
            principal: VerifiedPrincipal::system(name),
        }
    }

    fn service() -> SecurityService<InMemorySecurityPolicyRepository> {
        SecurityService::new(InMemorySecurityPolicyRepository::new())
    }

    fn service_with_roles() -> SecurityService<
        InMemorySecurityPolicyRepository,
        NoopSecurityAuditLog,
        InMemoryRoleMembership,
    > {
        SecurityService::with_all(
            InMemorySecurityPolicyRepository::new(),
            NoopSecurityAuditLog,
            InMemoryRoleMembership::new(),
        )
    }

    fn strict_policy(id: &str) -> Policy {
        Policy {
            policy_id: id.into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Strict,
            rules: vec![Rule {
                code: "AUTH".into(),
                enabled: true,
                description: None,
            }],
        }
    }

    fn baseline_policy(id: &str) -> Policy {
        Policy {
            policy_id: id.into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Baseline,
            rules: vec![Rule {
                code: "AUTH".into(),
                enabled: true,
                description: None,
            }],
        }
    }

    fn grant_req(principal: &str, op: &str) -> DelegationRequest {
        DelegationRequest {
            principal: principal.into(),
            operation: op.into(),
            resource: None,
            granted_by: "admin".into(),
            valid_from: now(),
            valid_to: later(),
            conditions: None,
            granted_via: None,
        }
    }

    // ── VerifiedPrincipal ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn human_principal_aceite() {
        assert!(service()
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn system_principal_aceite() {
        assert!(service()
            .authorize(&system_ctx("daemon:apid", "bootstrap.init"), None, now())
            .await
            .is_ok());
    }

    // ── AuthorizationToken selado ─────────────────────────────────────────────

    #[tokio::test]
    async fn token_so_criado_pelo_service() {
        let token = service()
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .unwrap();
        assert_eq!(token.principal, "user:alice");
        assert!(matches!(token.granted_by, GrantedBy::Bootstrap));
    }

    // ── Revogação de delegação ────────────────────────────────────────────────

    #[tokio::test]
    async fn revoke_delegation_remove_acesso() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let deleg = svc
            .repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());

        svc.revoke_delegation(
            &deleg.delegation_id,
            &system_ctx("daemon:admin", "delegation.revoke"),
            now(),
        )
        .await
        .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn revoke_delegation_not_found() {
        let err = service()
            .revoke_delegation(
                &DelegationId("nao-existe".into()),
                &system_ctx("daemon:admin", "delegation.revoke"),
                now(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, SecurityError::DelegationNotFound(_)));
    }

    // ── Cascata ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn revoke_delegation_cascata_filhos() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let root = svc
            .repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();

        let child_req = DelegationRequest {
            principal: "user:bob".into(),
            operation: "doc.sign".into(),
            resource: None,
            granted_by: "user:alice".into(),
            valid_from: now(),
            valid_to: later(),
            conditions: None,
            granted_via: Some(root.delegation_id.clone()),
        };
        svc.repo()
            .delegate_permission(&child_req, now())
            .await
            .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_ok());

        svc.revoke_delegation(
            &root.delegation_id,
            &system_ctx("daemon:admin", "delegation.revoke"),
            now(),
        )
        .await
        .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_err());
    }

    // ── Roles ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn role_delegation_concede_acesso_a_membro() {
        let svc = service_with_roles();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Delegar a operação ao role "role:editor"
        svc.repo()
            .delegate_permission(&grant_req("role:editor", "doc.sign"), now())
            .await
            .unwrap();

        // Alice ainda não tem acesso (não é membro do role)
        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());

        // Atribuir alice ao role "role:editor"
        svc.role_membership()
            .assign("user:alice", RoleId("role:editor".into()));

        // Alice tem acesso via role
        let token = svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .unwrap();
        assert!(matches!(token.granted_by, GrantedBy::RoleDelegation(_, _)));
        if let GrantedBy::RoleDelegation(role, _) = &token.granted_by {
            assert_eq!(role.as_str(), "role:editor");
        }
    }

    #[tokio::test]
    async fn delegacao_directa_prevalece_sobre_role() {
        let svc = service_with_roles();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Alice tem delegação directa E é membro de role com delegação
        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("role:editor", "doc.sign"), now())
            .await
            .unwrap();
        svc.role_membership()
            .assign("user:alice", RoleId("role:editor".into()));

        let token = svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .unwrap();
        // Delegação directa tem prioridade
        assert!(matches!(token.granted_by, GrantedBy::Delegation(_)));
    }

    #[tokio::test]
    async fn revogar_role_remove_acesso_via_role() {
        let svc = service_with_roles();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("role:editor", "doc.sign"), now())
            .await
            .unwrap();

        let editor = RoleId("role:editor".into());
        svc.role_membership().assign("user:alice", editor.clone());

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());

        svc.role_membership().revoke("user:alice", &editor);

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn grant_delegation_com_autoridade_via_role() {
        let svc = service_with_roles();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Delegar ao role
        svc.repo()
            .delegate_permission(&grant_req("role:editor", "doc.sign"), now())
            .await
            .unwrap();

        // Alice é membro do role → tem autoridade para sub-delegar
        svc.role_membership()
            .assign("user:alice", RoleId("role:editor".into()));

        let granter = human_ctx("user:alice", "delegation.grant");
        let result = svc
            .grant_delegation(&grant_req("user:bob", "doc.sign"), &granter, now())
            .await;
        assert!(result.is_ok());

        // Bob tem acesso via delegação directa
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_ok());
    }

    // ── Rules ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn rule_enabled_escalates_baseline_para_delegation_required() {
        let svc = service();
        svc.repo()
            .save_policy(
                &Policy {
                    policy_id: "pol-b".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Baseline,
                    rules: vec![Rule {
                        code: "doc.sign".into(),
                        enabled: true,
                        description: None,
                    }],
                },
                now(),
            )
            .await
            .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());

        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();
        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn rule_disabled_isenta_em_strict() {
        let svc = service();
        svc.repo()
            .save_policy(
                &Policy {
                    policy_id: "pol-s".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![
                        Rule {
                            code: "AUTH".into(),
                            enabled: true,
                            description: None,
                        },
                        Rule {
                            code: "audit.read".into(),
                            enabled: false,
                            description: None,
                        },
                    ],
                },
                now(),
            )
            .await
            .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
        let token = svc
            .authorize(&human_ctx("user:alice", "audit.read"), None, now())
            .await
            .unwrap();
        assert!(matches!(token.granted_by, GrantedBy::ExemptedByRule));
    }

    #[tokio::test]
    async fn rule_enabled_prevalece_sobre_disabled_em_conflito() {
        let svc = service();
        for (id, enabled) in [("pol-a", false), ("pol-b", true)] {
            svc.repo()
                .save_policy(
                    &Policy {
                        policy_id: id.into(),
                        version: "1.0.0".into(),
                        mode: PolicyMode::Baseline,
                        rules: vec![Rule {
                            code: "doc.sign".into(),
                            enabled,
                            description: None,
                        }],
                    },
                    now(),
                )
                .await
                .unwrap();
        }
        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
    }

    // ── grant_delegation() ────────────────────────────────────────────────────

    #[tokio::test]
    async fn grant_delegation_system_bypassa_authority() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        assert!(svc
            .grant_delegation(
                &grant_req("user:alice", "doc.sign"),
                &system_ctx("daemon:admin", "delegation.grant"),
                now(),
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn grant_delegation_human_sem_autoridade_falha() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let err = svc
            .grant_delegation(
                &grant_req("user:carol", "doc.sign"),
                &human_ctx("user:bob", "delegation.grant"),
                now(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, SecurityError::InvariantViolated(_)));
    }

    #[tokio::test]
    async fn grant_delegation_human_com_autoridade_passa() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        svc.grant_delegation(
            &grant_req("user:alice", "doc.sign"),
            &system_ctx("daemon:admin", "bootstrap"),
            now(),
        )
        .await
        .unwrap();

        assert!(svc
            .grant_delegation(
                &grant_req("user:bob", "doc.sign"),
                &human_ctx("user:alice", "delegation.grant"),
                now(),
            )
            .await
            .is_ok());
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn grant_delegation_auto_propaga_granted_via_e_cascata() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Sistema dá delegação raiz a alice
        let root = svc
            .grant_delegation(
                &grant_req("user:alice", "doc.sign"),
                &system_ctx("daemon:admin", "bootstrap"),
                now(),
            )
            .await
            .unwrap();

        // Alice sub-delega para bob — sem definir granted_via no pedido
        let bob_deleg = svc
            .grant_delegation(
                &grant_req("user:bob", "doc.sign"),
                &human_ctx("user:alice", "delegation.grant"),
                now(),
            )
            .await
            .unwrap();

        // O serviço preencheu granted_via automaticamente
        assert_eq!(bob_deleg.granted_via, Some(root.delegation_id.clone()));

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_ok());
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_ok());

        // Revogar root → cascata apanha bob automaticamente
        svc.revoke_delegation(
            &root.delegation_id,
            &system_ctx("daemon:admin", "delegation.revoke"),
            now(),
        )
        .await
        .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .is_err());
        assert!(svc
            .authorize(&human_ctx("user:bob", "doc.sign"), None, now())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn grant_delegation_impoe_granted_by_do_granter() {
        // granted_by é sempre sobrescrito com a identidade real do granter,
        // independentemente do que o caller colocar no pedido
        let svc = service();
        let deleg = svc
            .grant_delegation(
                &DelegationRequest {
                    granted_by: "impostor".into(),
                    ..grant_req("user:bob", "doc.sign")
                },
                &system_ctx("daemon:admin", "bootstrap"),
                now(),
            )
            .await
            .unwrap();
        assert_eq!(deleg.granted_by, "daemon:admin");
    }

    #[tokio::test]
    async fn grant_delegation_respeita_granted_via_definido_pelo_caller() {
        // Se o caller já definiu granted_via, o serviço não o sobrescreve
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let root = svc
            .grant_delegation(
                &grant_req("user:alice", "doc.sign"),
                &system_ctx("daemon:admin", "bootstrap"),
                now(),
            )
            .await
            .unwrap();

        let custom_parent = root.delegation_id.clone();
        let req_com_parent = DelegationRequest {
            granted_via: Some(custom_parent.clone()),
            ..grant_req("user:bob", "doc.sign")
        };

        let bob_deleg = svc
            .grant_delegation(
                &req_com_parent,
                &human_ctx("user:alice", "delegation.grant"),
                now(),
            )
            .await
            .unwrap();

        assert_eq!(bob_deleg.granted_via, Some(custom_parent));
    }

    #[tokio::test]
    async fn grant_delegation_em_bootstrap_qualquer_pode_delegar() {
        let svc = service();
        assert!(svc
            .grant_delegation(
                &grant_req("user:bob", "doc.sign"),
                &human_ctx("user:alice", "bootstrap.grant"),
                now(),
            )
            .await
            .is_ok());
    }

    // ── Audit log ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn audit_log_regista_concessao() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone());
        let _ = svc
            .authorize(&human_ctx("user:alice", "any.op"), None, now())
            .await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit.entries()[0].decision, AuditDecision::Granted);
    }

    #[tokio::test]
    async fn audit_log_regista_recusa() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone());
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        let _ = svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit.entries()[0].decision, AuditDecision::Denied);
    }

    // ── Cenários combinados ───────────────────────────────────────────────────

    #[tokio::test]
    async fn bootstrap_sem_politicas_permite() {
        let token = service()
            .authorize(&human_ctx("user:alice", "any.op"), None, now())
            .await
            .unwrap();
        assert!(matches!(token.granted_by, GrantedBy::Bootstrap));
    }

    #[tokio::test]
    async fn strict_passa_a_baseline_apos_revogar_unica_strict() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .save_policy(&baseline_policy("pol-b"), now())
            .await
            .unwrap();

        assert!(svc
            .authorize(&human_ctx("user:alice", "op"), None, now())
            .await
            .is_err());

        svc.repo()
            .revoke_policy(
                &RevocationRequest {
                    policy_id: "pol-s".into(),
                    revoked_by: "admin".into(),
                    reason: None,
                },
                now(),
            )
            .await
            .unwrap();

        let token = svc
            .authorize(&human_ctx("user:alice", "op"), None, now())
            .await
            .unwrap();
        assert!(matches!(token.granted_by, GrantedBy::BaselinePolicy));
    }

    #[tokio::test]
    async fn invariante_falha_antes_do_repo() {
        let bad = WriteInvariantContext {
            operation: "doc.sign".into(),
            correlation_id: "".into(),
            principal: VerifiedPrincipal::human("user:alice"),
        };
        assert!(matches!(
            service().authorize(&bad, None, now()).await.unwrap_err(),
            SecurityError::InvariantViolated(_)
        ));
    }

    // ── Paginação ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_active_policies_com_paginacao() {
        use crate::ListOptions;
        let svc = service();
        for i in 0..5 {
            svc.repo()
                .save_policy(
                    &Policy {
                        policy_id: format!("pol-{i:02}"),
                        version: "1.0.0".into(),
                        mode: PolicyMode::Baseline,
                        rules: vec![Rule {
                            code: "auth".into(),
                            enabled: true,
                            description: None,
                        }],
                    },
                    now(),
                )
                .await
                .unwrap();
        }
        let p1 = svc
            .repo()
            .list_active_policies(Some(ListOptions::page(1, 2)))
            .await
            .unwrap();
        let p2 = svc
            .repo()
            .list_active_policies(Some(ListOptions::page(2, 2)))
            .await
            .unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p2.len(), 2);
        assert_ne!(p1[0].policy_id, p2[0].policy_id);
    }
}

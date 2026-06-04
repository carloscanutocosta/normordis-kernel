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
//!          ├─ Nenhuma + bootstrap opt-in        → AuthorizationToken(Bootstrap)
//!          ├─ Nenhuma em produção               → Err(InvariantViolated)
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
    authz::{AuthzDecision, AuthzOutcome, AuthzRequest, EvidenceLevel},
    context::SecurityContext,
    delegation::DelegationCondition,
    event::{NoopSecurityEventPublisher, SecurityEvent, SecurityEventKind, SecurityEventPublisher},
    ports::SodHistoryProvider,
    role::{NoopRoleMembership, RoleId, RoleMembershipRepository},
    sod::{check_sod, SodRule, SodViolation},
    validate_write_invariant, Delegation, DelegationId, DelegationRequest, Policy, PolicyMode,
    ResourceAttributes, ResourceClassification, Rule, SecurityError, SecurityPolicyRepository,
    VerifiedPrincipal, WriteInvariantContext,
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

/// Comportamento quando ainda não existem políticas activas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapAuthorization {
    /// Deny-by-default: sem política activa, não há autorização.
    Deny,
    /// Permite operações durante bootstrap controlado.
    Allow,
}

/// Política perante falhas de componentes de evidência/observabilidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityFailureMode {
    /// A falha bloqueia a autorização, adequado a produção institucional.
    FailClosed,
    /// A falha é reportada para stderr mas não bloqueia a decisão.
    BestEffort,
}

/// Política operacional do motor de autorização.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityRuntimePolicy {
    pub bootstrap_authorization: BootstrapAuthorization,
    pub audit_failure: SecurityFailureMode,
    pub event_failure: SecurityFailureMode,
    pub sod_history_failure: SecurityFailureMode,
}

impl SecurityRuntimePolicy {
    /// Política recomendada para produção: deny-by-default e evidência obrigatória.
    pub const fn production() -> Self {
        Self {
            bootstrap_authorization: BootstrapAuthorization::Deny,
            audit_failure: SecurityFailureMode::FailClosed,
            event_failure: SecurityFailureMode::FailClosed,
            sod_history_failure: SecurityFailureMode::FailClosed,
        }
    }

    /// Política para bootstrap/testes: mantém o comportamento permissivo legado.
    pub const fn bootstrap_permissive() -> Self {
        Self {
            bootstrap_authorization: BootstrapAuthorization::Allow,
            audit_failure: SecurityFailureMode::BestEffort,
            event_failure: SecurityFailureMode::BestEffort,
            sod_history_failure: SecurityFailureMode::BestEffort,
        }
    }

    pub const fn with_bootstrap_authorization(mut self, mode: BootstrapAuthorization) -> Self {
        self.bootstrap_authorization = mode;
        self
    }

    pub const fn with_audit_failure(mut self, mode: SecurityFailureMode) -> Self {
        self.audit_failure = mode;
        self
    }

    pub const fn with_event_failure(mut self, mode: SecurityFailureMode) -> Self {
        self.event_failure = mode;
        self
    }

    pub const fn with_sod_history_failure(mut self, mode: SecurityFailureMode) -> Self {
        self.sod_history_failure = mode;
        self
    }
}

impl Default for SecurityRuntimePolicy {
    fn default() -> Self {
        Self::production()
    }
}

/// Motor de autorização de `core-security`.
///
/// Quatro genéricos com defaults:
/// - `R: SecurityPolicyRepository` — repositório de políticas e delegações
/// - `A: SecurityAuditLog` — log de decisões (default: noop)
/// - `M: RoleMembershipRepository` — pertença a roles (default: noop)
/// - `P: SecurityEventPublisher` — publicação de eventos de segurança (default: noop)
pub struct SecurityService<
    R,
    A = NoopSecurityAuditLog,
    M = NoopRoleMembership,
    P = NoopSecurityEventPublisher,
> {
    repo: R,
    audit: A,
    roles: M,
    events: P,
    runtime_policy: SecurityRuntimePolicy,
}

// ── Construtores ──────────────────────────────────────────────────────────────

impl<R: SecurityPolicyRepository>
    SecurityService<R, NoopSecurityAuditLog, NoopRoleMembership, NoopSecurityEventPublisher>
{
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            audit: NoopSecurityAuditLog,
            roles: NoopRoleMembership,
            events: NoopSecurityEventPublisher,
            runtime_policy: SecurityRuntimePolicy::production(),
        }
    }
}

impl<R: SecurityPolicyRepository, A: SecurityAuditLog>
    SecurityService<R, A, NoopRoleMembership, NoopSecurityEventPublisher>
{
    pub fn with_audit(repo: R, audit: A) -> Self {
        Self {
            repo,
            audit,
            roles: NoopRoleMembership,
            events: NoopSecurityEventPublisher,
            runtime_policy: SecurityRuntimePolicy::production(),
        }
    }
}

impl<R, A, M> SecurityService<R, A, M, NoopSecurityEventPublisher>
where
    R: SecurityPolicyRepository,
    A: SecurityAuditLog,
    M: RoleMembershipRepository,
{
    pub fn with_all(repo: R, audit: A, roles: M) -> Self {
        Self {
            repo,
            audit,
            roles,
            events: NoopSecurityEventPublisher,
            runtime_policy: SecurityRuntimePolicy::production(),
        }
    }
}

impl<R, A, M, P> SecurityService<R, A, M, P>
where
    R: SecurityPolicyRepository,
    A: SecurityAuditLog,
    M: RoleMembershipRepository,
    P: SecurityEventPublisher,
{
    /// Constrói o serviço com todos os componentes, incluindo publisher de eventos.
    pub fn with_publisher(repo: R, audit: A, roles: M, events: P) -> Self {
        Self {
            repo,
            audit,
            roles,
            events,
            runtime_policy: SecurityRuntimePolicy::production(),
        }
    }

    pub fn with_runtime_policy(mut self, runtime_policy: SecurityRuntimePolicy) -> Self {
        self.runtime_policy = runtime_policy;
        self
    }

    pub fn runtime_policy(&self) -> SecurityRuntimePolicy {
        self.runtime_policy
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

    pub fn event_publisher(&self) -> &P {
        &self.events
    }

    // ── Autorização ───────────────────────────────────────────────────────────

    /// Autoriza uma operação com contexto mínimo (operação + principal + resource string).
    ///
    /// ## Limitação: condições de delegação
    ///
    /// Este método **não avalia** condições de delegação (`DelegationCondition`).
    /// Se uma delegação tiver `conditions` definido (estado, classificação, âmbito orgânico),
    /// a delegação não será considerada activa e a operação será negada com uma mensagem
    /// explicativa a sugerir o uso de `authorize_contextual()`.
    ///
    /// Use [`authorize_contextual`](Self::authorize_contextual) quando:
    /// - O recurso tem atributos relevantes (estado, classificação, âmbito orgânico)
    /// - As delegações podem ter condições declaradas
    /// - É necessário verificar o nível de autenticação
    pub async fn authorize(
        &self,
        ctx: &WriteInvariantContext,
        resource: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<AuthorizationToken, SecurityError> {
        validate_write_invariant(ctx)?;

        let principal_id = ctx.principal.id();
        let result = self
            .do_authorize(ctx, resource, None, now, principal_id)
            .await;

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
                evidence_level: token_evidence_level(&token.granted_by),
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
                evidence_level: EvidenceLevel::None,
            },
        };
        self.handle_audit_result(self.audit.record_decision(&entry).await)?;
        // Emitir evento de segurança em caso de recusa
        if result.is_err() {
            let evt = SecurityEvent::new(
                SecurityEventKind::AuthorizationDenied,
                &ctx.correlation_id,
                now,
            )
            .with_principal(principal_id)
            .with_operation(&ctx.operation);
            self.handle_event_result(self.events.publish(&evt).await)?;
        }

        result
    }

    async fn do_authorize(
        &self,
        ctx: &WriteInvariantContext,
        resource: Option<&str>,
        resource_attrs: Option<&ResourceAttributes>,
        now: DateTime<Utc>,
        principal_id: &str,
    ) -> Result<AuthorizationToken, SecurityError> {
        // Gate 2a: delegação directa do principal
        let direct = self.repo.list_delegations(principal_id, now, None).await?;
        let mut conditions_blocked = false;

        for d in &direct {
            match delegation_status(d, &ctx.operation, resource, resource_attrs) {
                DelegationStatus::Granted => {
                    return Ok(self.token(
                        principal_id,
                        &ctx.operation,
                        resource,
                        GrantedBy::Delegation(d.delegation_id.clone()),
                        now,
                    ));
                }
                DelegationStatus::ConditionsFailed => conditions_blocked = true,
                DelegationStatus::NotApplicable => {}
            }
        }

        // Gate 2b: delegação via role
        let user_roles = self
            .roles
            .get_roles_for_principal(principal_id, now)
            .await?;
        for role in &user_roles {
            let role_delegations = self.repo.list_delegations(role.as_str(), now, None).await?;
            for d in &role_delegations {
                match delegation_status(d, &ctx.operation, resource, resource_attrs) {
                    DelegationStatus::Granted => {
                        return Ok(self.token(
                            principal_id,
                            &ctx.operation,
                            resource,
                            GrantedBy::RoleDelegation(role.clone(), d.delegation_id.clone()),
                            now,
                        ));
                    }
                    DelegationStatus::ConditionsFailed => conditions_blocked = true,
                    DelegationStatus::NotApplicable => {}
                }
            }
        }

        // Gate 3: políticas activas e temporalmente válidas
        let all_policies = self.repo.list_active_policies(None).await?;
        let policies: Vec<Policy> = all_policies
            .into_iter()
            .filter(|p| p.is_active_at(now))
            .collect();

        // Emitir PolicyEvaluated quando há políticas carregadas (gate 3 activo)
        if !policies.is_empty() {
            let evt =
                SecurityEvent::new(SecurityEventKind::PolicyEvaluated, &ctx.correlation_id, now)
                    .with_principal(principal_id)
                    .with_operation(&ctx.operation);
            self.handle_event_result(self.events.publish(&evt).await)?;
        }

        if policies.is_empty() {
            if self.runtime_policy.bootstrap_authorization == BootstrapAuthorization::Deny {
                return Err(SecurityError::InvariantViolated(
                    "sem políticas activas; bootstrap permissivo não está habilitado".into(),
                ));
            }
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
            let resource_info = resource.map(|r| format!(" em '{r}'")).unwrap_or_default();
            let reason = if conditions_blocked {
                if resource_attrs.is_none() {
                    format!(
                        "principal '{}' tem delegação activa para '{}'{resource_info} mas com \
                         condições que não puderam ser avaliadas sem ResourceAttributes — \
                         use authorize_contextual() para passar atributos do recurso",
                        principal_id, ctx.operation
                    )
                } else {
                    format!(
                        "principal '{}' tem delegação activa para '{}'{resource_info} mas as \
                         condições não estão satisfeitas (estado, classificação ou âmbito orgânico \
                         não correspondem aos atributos do recurso)",
                        principal_id, ctx.operation
                    )
                }
            } else {
                let mode_reason = if is_strict {
                    "modo strict activo"
                } else {
                    "operação governada por regra de política"
                };
                format!(
                    "principal '{}' não tem delegação activa para '{}'{resource_info} ({mode_reason})",
                    principal_id, ctx.operation
                )
            };
            return Err(SecurityError::InvariantViolated(reason));
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
        let parent_id = if policies.is_empty()
            && granter.principal.is_human()
            && self.runtime_policy.bootstrap_authorization == BootstrapAuthorization::Deny
        {
            return Err(SecurityError::InvariantViolated(
                "sem políticas activas; concessão humana em bootstrap não está habilitada".into(),
            ));
        } else if !policies.is_empty() && granter.principal.is_human() {
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
        // Na verificação de autoridade para sub-delegação não avaliamos conditions
        // (o granter possui a delegação; as conditions são contextuais à operação)
        let direct = self.repo.list_delegations(principal_id, now, None).await?;
        if let Some(d) = direct
            .iter()
            .find(|d| matches_delegation(d, operation, resource, None))
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
                .find(|d| matches_delegation(d, operation, resource, None))
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

    // ── Autorização contextual (ABAC) ─────────────────────────────────────────

    /// Autorização contextual — ABAC com nível de autenticação e atributos de recurso.
    ///
    /// Complementa `authorize()` com suporte a:
    /// - Verificação de nível de autenticação (`required_auth_level`)
    /// - Atributos de recurso (tipo, classificação, estado, âmbito orgânico)
    /// - Decisão explicável com razão, código e nível de evidência
    ///
    /// ## Deny-by-default
    ///
    /// Qualquer falha (campo vazio, nível insuficiente, sem delegação em modo strict)
    /// resulta em `AuthzDecision::deny(...)`. Ausência de política só permite
    /// acesso quando `SecurityRuntimePolicy` habilita bootstrap permissivo.
    pub async fn authorize_contextual(
        &self,
        subject: &VerifiedPrincipal,
        req: &AuthzRequest,
        ctx: &SecurityContext,
    ) -> AuthzDecision {
        // Gate 0: campos obrigatórios
        if req.action.trim().is_empty() {
            return AuthzDecision::deny("action obrigatória", "SEC.AUTHZ.MISSING_ACTION");
        }
        if req.correlation_id.trim().is_empty() {
            return AuthzDecision::deny(
                "correlation_id obrigatório",
                "SEC.AUTHZ.MISSING_CORRELATION",
            );
        }

        // Gate 0.5: consistência de âmbito orgânico
        //
        // Se o SecurityContext indica o âmbito orgânico activo do sujeito
        // E o recurso tem unidade orgânica definida → devem coincidir.
        // Se um dos dois estiver ausente, este gate não actua (não bloqueia).
        if let Some(ctx_scope) = &ctx.org_scope {
            if let Some(res_ou) = req.resource.as_ref().and_then(|r| r.org_unit.as_ref()) {
                if ctx_scope != res_ou {
                    let decision = AuthzDecision::deny(
                        format!(
                            "âmbito orgânico incompatível: sujeito em '{ctx_scope}', recurso em '{res_ou}'"
                        ),
                        "SEC.AUTHZ.ORG_SCOPE_MISMATCH",
                    );
                    if let Err(e) = self.log_contextual_decision(subject, req, &decision).await {
                        return decision_from_operational_error(e);
                    }
                    let evt = SecurityEvent::new(
                        SecurityEventKind::AuthorizationDenied,
                        &req.correlation_id,
                        req.at,
                    )
                    .with_principal(subject.id())
                    .with_operation(&req.action)
                    .with_details(format!("org_scope_mismatch: {ctx_scope} ≠ {res_ou}"));
                    if let Err(e) = self.handle_event_result(self.events.publish(&evt).await) {
                        return decision_from_operational_error(e);
                    }
                    return decision;
                }
            }
        }

        // Gate 1: nível de autenticação
        if let Some(required) = req.required_auth_level {
            if ctx.auth_level < required {
                let decision = AuthzDecision::deny(
                    format!(
                        "nível de autenticação insuficiente: requerido {required}, actual {}",
                        ctx.auth_level
                    ),
                    "SEC.AUTHZ.INSUFFICIENT_AUTH_LEVEL",
                );
                if let Err(e) = self.log_contextual_decision(subject, req, &decision).await {
                    return decision_from_operational_error(e);
                }
                // Emitir evento: tentativa de operar sem nível suficiente
                let evt = SecurityEvent::new(
                    SecurityEventKind::AuthorizationDenied,
                    &req.correlation_id,
                    req.at,
                )
                .with_principal(subject.id())
                .with_operation(&req.action)
                .with_details(format!("auth_level insuficiente: {}", decision.reason));
                if let Err(e) = self.handle_event_result(self.events.publish(&evt).await) {
                    return decision_from_operational_error(e);
                }
                return decision;
            }
        }

        // Gate 2: política e delegação (reutiliza motor existente)
        let inv_ctx = WriteInvariantContext {
            operation: req.action.clone(),
            correlation_id: req.correlation_id.clone(),
            principal: subject.clone(),
        };
        let resource_str = req.resource.as_ref().map(|r| r.resource_type.clone());
        let resource_attrs = req.resource.as_ref();

        let decision = match self
            .do_authorize(
                &inv_ctx,
                resource_str.as_deref(),
                resource_attrs,
                req.at,
                subject.id(),
            )
            .await
        {
            Ok(token) => {
                let (reason, code) = granted_by_to_reason(&token.granted_by);
                let evidence = evidence_for(&token.granted_by, req);
                AuthzDecision {
                    outcome: AuthzOutcome::Allow,
                    policy_id: ctx.policy_id.clone(),
                    reason,
                    evidence_level: evidence,
                    code,
                }
            }
            Err(SecurityError::InvariantViolated(msg)) => {
                AuthzDecision::deny(msg, "SEC.AUTHZ.INVARIANT_VIOLATED")
            }
            Err(err) => AuthzDecision::deny(err.to_string(), "SEC.AUTHZ.DENIED"),
        };

        if let Err(e) = self.log_contextual_decision(subject, req, &decision).await {
            return decision_from_operational_error(e);
        }

        // Emitir eventos de segurança
        let event_kind = if decision.is_denied() {
            SecurityEventKind::AuthorizationDenied
        } else if decision.evidence_level == EvidenceLevel::Enhanced {
            SecurityEventKind::AuthorizationAllowedSensitive
        } else {
            // Concessão normal sem evento especial
            return decision;
        };
        let evt = SecurityEvent::new(event_kind, &req.correlation_id, req.at)
            .with_principal(subject.id())
            .with_operation(&req.action);
        if let Err(e) = self.handle_event_result(self.events.publish(&evt).await) {
            return decision_from_operational_error(e);
        }

        decision
    }

    async fn log_contextual_decision(
        &self,
        subject: &VerifiedPrincipal,
        req: &AuthzRequest,
        decision: &AuthzDecision,
    ) -> Result<(), SecurityError> {
        let entry = SecurityAuthDecision {
            logged_at: req.at,
            principal: subject.id().into(),
            operation: req.action.clone(),
            resource: req.resource.as_ref().map(|r| r.resource_type.clone()),
            correlation_id: req.correlation_id.clone(),
            decision: if decision.is_allowed() {
                AuditDecision::Granted
            } else {
                AuditDecision::Denied
            },
            granted_by_kind: if decision.is_allowed() {
                Some(decision.code.clone())
            } else {
                None
            },
            deny_reason: if decision.is_denied() {
                Some(decision.reason.clone())
            } else {
                None
            },
            evidence_level: decision.evidence_level,
        };
        self.handle_audit_result(self.audit.record_decision(&entry).await)
    }

    // ── Segregação de funções ─────────────────────────────────────────────────

    /// Verifica violações de SoD (Separation of Duties) consultando o histórico.
    ///
    /// ## Quando usar
    ///
    /// Chamar antes de `authorize()` ou `authorize_contextual()` em operações
    /// sensíveis (e.g., aprovação de documentos). Se devolver `Some(SodViolation)`:
    /// - `override_allowed = false` → bloquear a operação
    /// - `override_allowed = true` → registar a excepção com evidência reforçada
    ///
    /// ## Falha do histórico
    ///
    /// Se `history.previous_actions()` falhar, o gate devolve `None` (fail-open)
    /// e regista em stderr. A decisão de autorização principal (delegação + política)
    /// continua a ser o gate primário de segurança.
    #[allow(clippy::too_many_arguments)] // gate SoD exige todos os parâmetros; agrupar seria API confusa
    pub async fn check_sod_gate<H: SodHistoryProvider>(
        &self,
        rules: &[SodRule],
        principal_id: &str,
        action: &str,
        resource_id: &str,
        history: &H,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Option<SodViolation> {
        let previous = match history
            .previous_actions(principal_id, resource_id, now)
            .await
        {
            Ok(p) => p,
            Err(e) => match self.runtime_policy.sod_history_failure {
                SecurityFailureMode::BestEffort => {
                    eprintln!("[core-security] SoD history fetch failed: {e}");
                    return None;
                }
                SecurityFailureMode::FailClosed => {
                    return Some(SodViolation {
                        rule_id: "SOD.HISTORY.UNAVAILABLE".into(),
                        description: format!("histórico SoD indisponível: {e}"),
                        override_allowed: false,
                    });
                }
            },
        };

        let violation = check_sod(rules, action, &previous);

        if let Some(ref v) = violation {
            let evt =
                SecurityEvent::new(SecurityEventKind::SodViolationDetected, correlation_id, now)
                    .with_principal(principal_id)
                    .with_operation(action)
                    .with_resource(resource_id)
                    .with_details(v.to_string());
            if let Err(e) = self.handle_event_result(self.events.publish(&evt).await) {
                return Some(SodViolation {
                    rule_id: "SOD.EVENT.UNAVAILABLE".into(),
                    description: e.to_string(),
                    override_allowed: false,
                });
            }
        }

        violation
    }

    fn handle_audit_result(&self, result: Result<(), SecurityError>) -> Result<(), SecurityError> {
        match (result, self.runtime_policy.audit_failure) {
            (Ok(()), _) => Ok(()),
            (Err(e), SecurityFailureMode::BestEffort) => {
                eprintln!("[core-security] audit log failed: {e}");
                Ok(())
            }
            (Err(e), SecurityFailureMode::FailClosed) => {
                Err(SecurityError::AuditUnavailable(e.to_string()))
            }
        }
    }

    fn handle_event_result(&self, result: Result<(), SecurityError>) -> Result<(), SecurityError> {
        match (result, self.runtime_policy.event_failure) {
            (Ok(()), _) => Ok(()),
            (Err(e), SecurityFailureMode::BestEffort) => {
                eprintln!("[core-security] event publish failed: {e}");
                Ok(())
            }
            (Err(e), SecurityFailureMode::FailClosed) => {
                Err(SecurityError::EventPublishFailed(e.to_string()))
            }
        }
    }
}

// ── Funções auxiliares ────────────────────────────────────────────────────────

/// Nível de evidência para o path simples `authorize()` (sem ResourceAttributes).
///
/// Delegações explícitas → Normal; bootstrap/baseline/isenção → None.
fn token_evidence_level(g: &GrantedBy) -> EvidenceLevel {
    match g {
        GrantedBy::Delegation(_) | GrantedBy::RoleDelegation(_, _) => EvidenceLevel::Normal,
        _ => EvidenceLevel::None,
    }
}

/// Converte a razão de concessão num par (mensagem legível, código estruturado).
fn granted_by_to_reason(g: &GrantedBy) -> (String, String) {
    match g {
        GrantedBy::Delegation(id) => (
            format!("delegação directa: {}", id.as_str()),
            "SEC.AUTHZ.ALLOW.DELEGATION".into(),
        ),
        GrantedBy::RoleDelegation(role, id) => (
            format!("delegação via role {} ({})", role.as_str(), id.as_str()),
            "SEC.AUTHZ.ALLOW.ROLE_DELEGATION".into(),
        ),
        GrantedBy::BaselinePolicy => (
            "modo baseline sem regra governante".into(),
            "SEC.AUTHZ.ALLOW.BASELINE".into(),
        ),
        GrantedBy::ExemptedByRule => (
            "operação isenta por regra de política".into(),
            "SEC.AUTHZ.ALLOW.EXEMPTED".into(),
        ),
        GrantedBy::Bootstrap => (
            "sistema em arranque — sem políticas activas".into(),
            "SEC.AUTHZ.ALLOW.BOOTSTRAP".into(),
        ),
    }
}

/// Determina o nível de evidência com base na razão de concessão e nos atributos do recurso.
///
/// Recursos Restricted/Secret ou delegações explícitas exigem evidência reforçada.
fn evidence_for(g: &GrantedBy, req: &AuthzRequest) -> EvidenceLevel {
    let high_class = req.resource.as_ref().is_some_and(|r| {
        matches!(
            r.classification,
            Some(ResourceClassification::Restricted) | Some(ResourceClassification::Secret)
        )
    });
    if high_class {
        return EvidenceLevel::Enhanced;
    }
    match g {
        GrantedBy::Bootstrap | GrantedBy::BaselinePolicy | GrantedBy::ExemptedByRule => {
            EvidenceLevel::None
        }
        GrantedBy::Delegation(_) | GrantedBy::RoleDelegation(_, _) => EvidenceLevel::Normal,
    }
}

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

fn decision_from_operational_error(err: SecurityError) -> AuthzDecision {
    let code = match err {
        SecurityError::AuditUnavailable(_) => "SEC.AUTHZ.AUDIT_UNAVAILABLE",
        SecurityError::EventPublishFailed(_) => "SEC.AUTHZ.EVENT_UNAVAILABLE",
        SecurityError::SodHistoryUnavailable(_) => "SEC.AUTHZ.SOD_HISTORY_UNAVAILABLE",
        _ => "SEC.AUTHZ.OPERATIONAL_FAILURE",
    };
    AuthzDecision::deny(err.to_string(), code)
}

/// Resultado detalhado da verificação de uma delegação.
///
/// Distingue "não se aplica" de "aplica-se mas conditions não satisfeitas",
/// permitindo ao chamador produzir mensagens de erro informativas.
#[derive(Debug, PartialEq, Eq)]
enum DelegationStatus {
    /// Operação, recurso e conditions (se existirem) satisfeitos — conceder.
    Granted,
    /// Operação e recurso coincidem, mas conditions não estão satisfeitas.
    ConditionsFailed,
    /// Operação ou recurso não coincidem — delegação irrelevante.
    NotApplicable,
}

/// Avalia o estado de uma delegação contra `(operation, resource, resource_attrs)`.
fn delegation_status(
    d: &Delegation,
    operation: &str,
    resource: Option<&str>,
    resource_attrs: Option<&ResourceAttributes>,
) -> DelegationStatus {
    if d.operation != operation {
        return DelegationStatus::NotApplicable;
    }
    let resource_ok = match (&d.resource, resource) {
        (None, _) => true,
        (Some(dr), Some(rr)) => dr == rr,
        (Some(_), None) => false,
    };
    if !resource_ok {
        return DelegationStatus::NotApplicable;
    }
    if let Some(cond_str) = &d.conditions {
        match DelegationCondition::parse(cond_str) {
            Some(cond) if cond.evaluate(resource_attrs) => DelegationStatus::Granted,
            _ => DelegationStatus::ConditionsFailed, // JSON inválido OU conditions não satisfeitas
        }
    } else {
        DelegationStatus::Granted
    }
}

/// Compatibilidade com chamadas internas que só precisam de bool.
fn matches_delegation(
    d: &Delegation,
    operation: &str,
    resource: Option<&str>,
    resource_attrs: Option<&ResourceAttributes>,
) -> bool {
    delegation_status(d, operation, resource, resource_attrs) == DelegationStatus::Granted
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
            .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive())
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
        .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive())
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
            valid_from: None,
            valid_to: None,
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
            valid_from: None,
            valid_to: None,
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
                    valid_from: None,
                    valid_to: None,
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
                    valid_from: None,
                    valid_to: None,
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
                        valid_from: None,
                        valid_to: None,
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
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
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
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
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
    async fn producao_sem_politicas_nega_por_defeito() {
        let svc = SecurityService::new(InMemorySecurityPolicyRepository::new());
        let err = svc
            .authorize(&human_ctx("user:alice", "any.op"), None, now())
            .await
            .unwrap_err();
        assert!(matches!(err, SecurityError::InvariantViolated(_)));
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
                        valid_from: None,
                        valid_to: None,
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

    // ── authorize_contextual ──────────────────────────────────────────────────

    use crate::{
        auth_level::AuthLevel,
        authz::{AuthzRequest, EvidenceLevel, ResourceAttributes},
        classification::ResourceClassification,
        context::{OrgScope, SecurityContext},
    };

    fn authz_req(action: &str) -> AuthzRequest {
        AuthzRequest::new(action, "corr-test", now())
    }

    fn normal_ctx(subject: &str) -> SecurityContext {
        SecurityContext::minimal(subject, AuthLevel::Normal)
    }

    #[tokio::test]
    async fn contextual_bootstrap_permite() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let decision = svc
            .authorize_contextual(&subject, &authz_req("doc.sign"), &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_allowed());
        assert_eq!(decision.code, "SEC.AUTHZ.ALLOW.BOOTSTRAP");
    }

    #[tokio::test]
    async fn contextual_producao_sem_politicas_nega_por_defeito() {
        let svc = SecurityService::new(InMemorySecurityPolicyRepository::new());
        let subject = VerifiedPrincipal::human("user:alice");
        let decision = svc
            .authorize_contextual(&subject, &authz_req("doc.sign"), &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_denied());
        assert_eq!(decision.code, "SEC.AUTHZ.INVARIANT_VIOLATED");
    }

    #[tokio::test]
    async fn contextual_strict_sem_delegacao_nega() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        let subject = VerifiedPrincipal::human("user:alice");
        let decision = svc
            .authorize_contextual(&subject, &authz_req("doc.sign"), &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_denied());
        assert_eq!(decision.code, "SEC.AUTHZ.INVARIANT_VIOLATED");
    }

    #[tokio::test]
    async fn contextual_delegacao_directa_permite() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();
        let subject = VerifiedPrincipal::human("user:alice");
        let decision = svc
            .authorize_contextual(&subject, &authz_req("doc.sign"), &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_allowed());
        assert_eq!(decision.code, "SEC.AUTHZ.ALLOW.DELEGATION");
        assert_eq!(decision.evidence_level, EvidenceLevel::Normal);
    }

    #[tokio::test]
    async fn contextual_auth_level_insuficiente_nega() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = authz_req("doc.sign_official").with_required_auth_level(AuthLevel::Strong);
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(decision.is_denied());
        assert_eq!(decision.code, "SEC.AUTHZ.INSUFFICIENT_AUTH_LEVEL");
        assert!(decision.reason.contains("strong"));
    }

    #[tokio::test]
    async fn contextual_auth_level_suficiente_prossegue() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = authz_req("doc.sign_official").with_required_auth_level(AuthLevel::Reinforced);
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Strong);
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        // Sem políticas → Bootstrap
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn contextual_recurso_restrito_evidence_enhanced() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.export"), now())
            .await
            .unwrap();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.export", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("document_instance")
                .with_classification(ResourceClassification::Restricted),
        );
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(decision.is_allowed());
        assert_eq!(decision.evidence_level, EvidenceLevel::Enhanced);
    }

    #[tokio::test]
    async fn contextual_action_vazia_nega() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("", "corr-1", now());
        let decision = svc
            .authorize_contextual(&subject, &req, &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_denied());
        assert_eq!(decision.code, "SEC.AUTHZ.MISSING_ACTION");
    }

    #[tokio::test]
    async fn contextual_correlation_id_vazio_nega() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.sign", "", now());
        let decision = svc
            .authorize_contextual(&subject, &req, &normal_ctx("user:alice"))
            .await;
        assert!(decision.is_denied());
        assert_eq!(decision.code, "SEC.AUTHZ.MISSING_CORRELATION");
    }

    #[tokio::test]
    async fn contextual_regista_no_audit_log() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
        let subject = VerifiedPrincipal::human("user:alice");
        svc.authorize_contextual(&subject, &authz_req("doc.read"), &normal_ctx("user:alice"))
            .await;
        assert_eq!(audit.len(), 1);
        assert_eq!(audit.entries()[0].decision, AuditDecision::Granted);
        assert_eq!(audit.entries()[0].operation, "doc.read");
    }

    #[tokio::test]
    async fn contextual_org_scope_propagado_para_policy_id() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal)
            .with_org_scope(OrgScope::new("SF-1234"))
            .with_decision(Some("SEC.DOC.SUBMIT".into()), Some("AUTHZ-789".into()));
        let decision = svc
            .authorize_contextual(&subject, &authz_req("doc.submit"), &ctx)
            .await;
        assert!(decision.is_allowed());
        assert_eq!(decision.policy_id, Some("SEC.DOC.SUBMIT".into()));
    }

    // ── Erro informativo de conditions em authorize() ─────────────────────────

    #[tokio::test]
    async fn authorize_com_conditions_e_sem_attrs_da_erro_informativo() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Delegação com condição de estado — authorize() não tem ResourceAttributes
        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            ..Default::default()
        };
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.sign".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some(cond.to_json().unwrap()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        let err = svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .unwrap_err();

        // A mensagem deve mencionar conditions e sugerir authorize_contextual
        let msg = err.to_string();
        assert!(
            msg.contains("condições") && msg.contains("authorize_contextual"),
            "mensagem deve explicar o problema e a solução: {msg}"
        );
    }

    #[tokio::test]
    async fn authorize_com_conditions_satisfeitas_em_contextual_funciona() {
        // Confirma que a mesma delegação com conditions funciona em authorize_contextual
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            ..Default::default()
        };
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.sign".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some(cond.to_json().unwrap()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.sign", "corr-1", now())
            .with_resource(ResourceAttributes::of_type("doc").with_state("draft"));
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);

        assert!(
            svc.authorize_contextual(&subject, &req, &ctx)
                .await
                .is_allowed(),
            "authorize_contextual com state=draft deve funcionar"
        );
    }

    // ── Gate 0.5: org_scope ───────────────────────────────────────────────────

    #[tokio::test]
    async fn contextual_org_scope_match_permite() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.read", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-1234")),
        );
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal)
            .with_org_scope(OrgScope::new("SF-1234"));
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(decision.is_allowed(), "mesmo org_scope → deve autorizar");
    }

    #[tokio::test]
    async fn contextual_org_scope_mismatch_nega() {
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.read", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-9999")),
        );
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal)
            .with_org_scope(OrgScope::new("SF-1234"));
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(decision.is_denied(), "org_scope diferente → deve negar");
        assert_eq!(decision.code, "SEC.AUTHZ.ORG_SCOPE_MISMATCH");
        assert!(decision.reason.contains("SF-1234"));
        assert!(decision.reason.contains("SF-9999"));
    }

    #[tokio::test]
    async fn contextual_org_scope_ausente_no_contexto_nao_bloqueia() {
        // Sem ctx.org_scope → gate 0.5 não actua
        let svc = service();
        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.read", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-1234")),
        );
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal); // sem org_scope
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(
            decision.is_allowed(),
            "sem ctx.org_scope → gate 0.5 inactivo"
        );
    }

    // ── DelegationCondition: testes E2E no path authorize_contextual ──────────

    use crate::delegation::DelegationCondition;

    #[tokio::test]
    async fn conditions_required_state_satisfeito_autoriza() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            ..Default::default()
        };
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.submit".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some(cond.to_json().unwrap()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        let subject = VerifiedPrincipal::human("user:alice");
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);

        let req_draft = AuthzRequest::new("doc.submit", "corr-1", now())
            .with_resource(ResourceAttributes::of_type("doc").with_state("draft"));
        assert!(
            svc.authorize_contextual(&subject, &req_draft, &ctx)
                .await
                .is_allowed(),
            "state=draft satisfaz a condição"
        );

        let req_approved = AuthzRequest::new("doc.submit", "corr-2", now())
            .with_resource(ResourceAttributes::of_type("doc").with_state("approved"));
        let denied = svc
            .authorize_contextual(&subject, &req_approved, &ctx)
            .await;
        assert!(denied.is_denied(), "state=approved não satisfaz a condição");
    }

    #[tokio::test]
    async fn conditions_sem_resource_attrs_nega() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            ..Default::default()
        };
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.submit".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some(cond.to_json().unwrap()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        // Sem resource_attrs → a condição não pode ser verificada → nega
        let subject = VerifiedPrincipal::human("user:alice");
        let req = authz_req("doc.submit"); // sem .with_resource(...)
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);
        let decision = svc.authorize_contextual(&subject, &req, &ctx).await;
        assert!(decision.is_denied(), "conditions sem resource_attrs → nega");
    }

    #[tokio::test]
    async fn conditions_json_invalido_nega() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // Condição com JSON malformado → deny-by-default
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.sign".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some("not-valid-json".into()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        let subject = VerifiedPrincipal::human("user:alice");
        let req = authz_req("doc.sign");
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);
        assert!(
            svc.authorize_contextual(&subject, &req, &ctx)
                .await
                .is_denied(),
            "JSON inválido → nega por deny-by-default"
        );
    }

    #[tokio::test]
    async fn conditions_required_org_unit_match_autoriza() {
        let svc = service();
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        let cond = DelegationCondition {
            required_org_unit: Some(OrgScope::new("SF-1234")),
            ..Default::default()
        };
        svc.repo()
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.approve".into(),
                    resource: None,
                    granted_by: "daemon:admin".into(),
                    valid_from: now(),
                    valid_to: later(),
                    conditions: Some(cond.to_json().unwrap()),
                    granted_via: None,
                },
                now(),
            )
            .await
            .unwrap();

        let subject = VerifiedPrincipal::human("user:alice");
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);

        let in_scope = AuthzRequest::new("doc.approve", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-1234")),
        );
        assert!(svc
            .authorize_contextual(&subject, &in_scope, &ctx)
            .await
            .is_allowed());

        let out_scope = AuthzRequest::new("doc.approve", "corr-2", now()).with_resource(
            ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-9999")),
        );
        assert!(svc
            .authorize_contextual(&subject, &out_scope, &ctx)
            .await
            .is_denied());
    }

    // ── check_sod_gate ────────────────────────────────────────────────────────

    use crate::{sod::SodRule, NoopSodHistoryProvider};

    struct MockSodHistory(Vec<String>);

    impl crate::SodHistoryProvider for MockSodHistory {
        async fn previous_actions(
            &self,
            _: &str,
            _: &str,
            _: DateTime<Utc>,
        ) -> Result<Vec<String>, SecurityError> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn sod_gate_sem_historico_nao_viola() {
        let svc = service();
        let rules = vec![SodRule::no_self_approval("SOD-001")];
        let result = svc
            .check_sod_gate(
                &rules,
                "user:alice",
                "document.approve",
                "doc:123",
                &NoopSodHistoryProvider,
                "corr-1",
                now(),
            )
            .await;
        assert!(result.is_none(), "sem histórico → sem violação");
    }

    #[tokio::test]
    async fn sod_gate_com_conflito_viola_e_emite_evento() {
        use crate::InMemorySecurityEventPublisher;
        let publisher = InMemorySecurityEventPublisher::new();
        let svc = SecurityService::with_publisher(
            InMemorySecurityPolicyRepository::new(),
            NoopSecurityAuditLog,
            NoopRoleMembership,
            publisher.clone(),
        );

        let rules = vec![SodRule::no_self_approval("SOD-001")];
        let history = MockSodHistory(vec!["document.create".into()]);

        let violation = svc
            .check_sod_gate(
                &rules,
                "user:alice",
                "document.approve",
                "doc:123",
                &history,
                "corr-1",
                now(),
            )
            .await;

        assert!(violation.is_some(), "criou E quer aprovar → violação");
        assert_eq!(violation.unwrap().rule_id, "SOD-001");
        assert_eq!(
            publisher.count_kind(&SecurityEventKind::SodViolationDetected),
            1,
            "evento SodViolationDetected deve ser emitido"
        );
    }

    // ── EvidenceLevel no audit log ────────────────────────────────────────────

    #[tokio::test]
    async fn evidence_level_normal_para_delegacao_em_authorize() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.sign"), now())
            .await
            .unwrap();

        svc.authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await
            .unwrap();

        assert_eq!(audit.entries()[0].evidence_level, EvidenceLevel::Normal);
    }

    #[tokio::test]
    async fn evidence_level_enhanced_para_recurso_restrito_em_contextual() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();
        svc.repo()
            .delegate_permission(&grant_req("user:alice", "doc.export"), now())
            .await
            .unwrap();

        let subject = VerifiedPrincipal::human("user:alice");
        let req = AuthzRequest::new("doc.export", "corr-1", now()).with_resource(
            ResourceAttributes::of_type("doc").with_classification(ResourceClassification::Secret),
        );
        let ctx = SecurityContext::minimal("user:alice", AuthLevel::Normal);
        svc.authorize_contextual(&subject, &req, &ctx).await;

        assert_eq!(audit.entries()[0].evidence_level, EvidenceLevel::Enhanced);
    }

    #[tokio::test]
    async fn evidence_level_none_para_bootstrap() {
        use crate::InMemoryAuditLog;
        let audit = InMemoryAuditLog::new();
        let svc =
            SecurityService::with_audit(InMemorySecurityPolicyRepository::new(), audit.clone())
                .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());

        // Sem políticas → Bootstrap → EvidenceLevel::None
        svc.authorize(&human_ctx("user:alice", "any.op"), None, now())
            .await
            .unwrap();

        assert_eq!(audit.entries()[0].evidence_level, EvidenceLevel::None);
    }

    #[tokio::test]
    async fn policy_evaluated_evento_emitido_no_gate3() {
        use crate::InMemorySecurityEventPublisher;
        let publisher = InMemorySecurityEventPublisher::new();
        let svc = SecurityService::with_publisher(
            InMemorySecurityPolicyRepository::new(),
            NoopSecurityAuditLog,
            NoopRoleMembership,
            publisher.clone(),
        );
        svc.repo()
            .save_policy(&strict_policy("pol-s"), now())
            .await
            .unwrap();

        // A tentativa de autorização sem delegação → chega ao gate 3 → PolicyEvaluated
        let _ = svc
            .authorize(&human_ctx("user:alice", "doc.sign"), None, now())
            .await;
        assert!(
            publisher.count_kind(&SecurityEventKind::PolicyEvaluated) >= 1,
            "PolicyEvaluated deve ser emitido quando há políticas activas"
        );
    }
}

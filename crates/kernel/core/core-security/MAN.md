# MAN — core-security

## Objectivo

Autorização zero-trust com políticas soberanas. Define os tipos e serviços que governam quem pode fazer o quê sobre que recurso, com suporte a delegações em cadeia e auditoria imutável de todas as decisões.

---

## Contrato público

### Erros

```rust
pub struct SecurityError { pub code: ErrorCode, pub component: Component, ... }
pub const COMPONENT: Component; // "core-security"
// Códigos: Denied, NotFound, InvalidPolicy, InvalidPrincipal,
//           DelegationNotFound, DelegationExpired, InsufficientAuthority, ...
```

### Principal

```rust
pub struct VerifiedPrincipal {
    pub id: String,
    pub kind: PrincipalKind,
}
pub enum PrincipalKind { Human, System }

impl VerifiedPrincipal {
    pub fn human(id: impl Into<String>) -> Self;
    pub fn system(name: impl Into<String>) -> Self;
}
```

### Política

```rust
pub struct Policy {
    pub mode: PolicyMode,
    pub rules: Vec<Rule>,
}
pub enum PolicyMode { Baseline, Strict }
// Baseline: permite tudo o que não está explicitamente negado.
// Strict:   nega tudo o que não está explicitamente permitido.

pub struct Rule {
    pub code: String,   // ex: "document.read", "document.*"
    pub enabled: bool,
    pub description: Option<String>,
}
pub fn validate_policy(policy: &Policy) -> Result<(), SecurityError>;
```

### Delegações

```rust
pub struct Delegation {
    pub id: DelegationId,
    pub from_principal: String,
    pub to_principal: String,
    pub action_code: String,
    pub resource_pattern: Option<String>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub granted_via: Option<DelegationId>, // cadeia de delegação
}
pub struct DelegationId(pub String);

pub struct DelegationRequest {
    pub from_principal: String,
    pub to_principal: String,
    pub action_code: String,
    pub resource_pattern: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct RevocationRequest {
    pub delegation_id: DelegationId,
    pub revoked_by: String,
    pub reason: Option<String>,
}
```

### Auditoria

```rust
pub trait SecurityAuditLog: Send + Sync {
    fn record(&self, decision: AuditDecision) -> Result<(), SecurityError>;
    fn list(&self, options: ListOptions) -> Result<Vec<AuditDecision>, SecurityError>;
}
pub struct AuditDecision {
    pub principal: String,
    pub action: String,
    pub resource: Option<String>,
    pub decision: SecurityAuthDecision,
    pub decided_at: DateTime<Utc>,
    pub granted_by: Option<GrantedBy>,
}
pub enum SecurityAuthDecision { Granted, Denied }
pub enum GrantedBy { Policy, Delegation(DelegationId) }

pub struct InMemoryAuditLog { ... }    // para testes
pub struct NoopSecurityAuditLog;       // sem registo
```

### Roles

```rust
pub struct RoleId(pub String);
pub trait RoleMembershipRepository: Send + Sync {
    fn assign(&self, principal_id: &str, role_id: &RoleId) -> Result<(), SecurityError>;
    fn revoke(&self, principal_id: &str, role_id: &RoleId) -> Result<(), SecurityError>;
    fn roles_for(&self, principal_id: &str) -> Result<Vec<RoleId>, SecurityError>;
    fn members_of(&self, role_id: &RoleId) -> Result<Vec<String>, SecurityError>;
}
pub struct InMemoryRoleMembership { ... }
pub struct NoopRoleMembership;
```

### Repository port

```rust
pub trait SecurityPolicyRepository: Send + Sync {
    fn get_policy(&self, principal_id: &str) -> Result<Option<Policy>, SecurityError>;
    fn set_policy(&self, principal_id: &str, policy: Policy) -> Result<(), SecurityError>;
    fn get_delegation(&self, id: &DelegationId) -> Result<Option<Delegation>, SecurityError>;
    fn list_delegations_from(&self, principal_id: &str) -> Result<Vec<Delegation>, SecurityError>;
    fn list_delegations_to(&self, principal_id: &str) -> Result<Vec<Delegation>, SecurityError>;
    fn save_delegation(&self, delegation: Delegation) -> Result<(), SecurityError>;
    fn revoke_delegation(&self, req: RevocationRequest) -> Result<(), SecurityError>;
}
pub struct InMemorySecurityPolicyRepository { ... }
```

### Serviço de autorização

```rust
pub struct SecurityService<R, A, M>
where
    R: SecurityPolicyRepository,
    A: SecurityAuditLog,
    M: RoleMembershipRepository,
{ ... }

impl<R, A, M> SecurityService<R, A, M> {
    pub fn new(repo: R, audit: A, roles: M) -> Self;

    /// Avalia os 4 gates em sequência e devolve token de autorização ou erro.
    pub fn authorize(
        &self,
        principal: &VerifiedPrincipal,
        action: &str,
        resource: &str,
    ) -> Result<AuthorizationToken, SecurityError>;

    /// Concede delegação; verifica que o `from` tem autoridade para delegar.
    pub fn grant_delegation(&self, req: DelegationRequest) -> Result<DelegationId, SecurityError>;

    /// Revoga delegação; cascata sobre delegações derivadas (granted_via).
    pub fn revoke_delegation(&self, req: RevocationRequest) -> Result<(), SecurityError>;

    /// Encontra a delegação activa que permite ao principal a acção.
    pub fn find_granting_delegation(
        &self,
        principal_id: &str,
        action: &str,
        resource: &str,
    ) -> Result<Option<Delegation>, SecurityError>;
}

pub struct AuthorizationToken {
    pub principal: String,
    pub action: String,
    pub resource: String,
    pub granted_by: GrantedBy,
    pub granted_at: DateTime<Utc>,
}
```

### Invariante de escrita

```rust
pub struct WriteInvariantContext {
    pub principal: &VerifiedPrincipal,
    pub action: &str,
    pub resource: &str,
}
pub fn validate_write_invariant(ctx: &WriteInvariantContext) -> Result<(), SecurityError>;
```

---

## Como usar

### Autorização simples

```rust
let svc = SecurityService::new(repo, audit, roles);
let principal = VerifiedPrincipal::human("user-1");

match svc.authorize(&principal, "document.sign", "oficio-99") {
    Ok(token) => println!("Autorizado via {:?}", token.granted_by),
    Err(e) => eprintln!("Negado: {e}"),
}
```

### Delegação com expiração

```rust
svc.grant_delegation(DelegationRequest {
    from_principal: "director-1".into(),
    to_principal: "substituto-1".into(),
    action_code: "document.sign".into(),
    resource_pattern: Some("oficio/*".into()),
    expires_at: Some(Utc::now() + Duration::days(5)),
})?;
```

### Política Strict (deny-by-default)

```rust
let policy = Policy {
    mode: PolicyMode::Strict,
    rules: vec![
        Rule { code: "document.read".into(), enabled: true, description: None },
    ],
};
repo.set_policy("user-1", policy)?;
// user-1 só pode "document.read" — tudo o resto é negado
```

---

## Invariantes

- Os 4 gates de `authorize()` são avaliados em sequência: (1) principal válido, (2) política da entidade, (3) delegações directas, (4) roles e políticas de role.
- `revoke_delegation` faz cascata: todas as delegações com `granted_via` igual ao id revogado são também revogadas.
- `grant_delegation` só é permitido se o `from` tiver autoridade para a acção (via política ou delegação própria).
- `AuthorizationToken` é imutável e deve ser validado pelo receptor antes de executar a operação.

---

## Limites actuais

- Sem suporte a wildcards em `action_code` (ex: `document.*`) na avaliação de delegações.
- Sem TTL automático de delegações expiradas — o caller deve verificar `expires_at`.
- Sem suporte multi-tenant (todos os principals partilham o mesmo repositório).

---

## ToDo

- [ ] Wildcards em action codes.
- [ ] Expiração automática de delegações (background job ou lazy check).
- [ ] Exportação de auditoria em formato JSON para SIEM externo.

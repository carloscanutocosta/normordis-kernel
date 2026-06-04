# MAN — core-security

## Objectivo

`core-security` modela autorização institucional reutilizável: políticas soberanas,
delegações temporais, roles, autorização contextual, segregação de funções, auditoria
de decisões e eventos de segurança. É domínio puro: não persiste dados, não autentica
identidades e não depende de SQLite, IAM, UI ou runtime concreto.

## Posição arquitectural

- `core-security` define semântica, tipos, serviços e portos.
- `security-sqlite` implementa persistência de políticas, delegações e decisões.
- `rh-security-bridge` implementa memberships de roles e validação de âmbito RH.
- SIEM, core-audit, AD/LDAP/OAuth e bootstrap final pertencem a adapters/runtime.

## Contrato público principal

### Serviço

```rust
pub struct SecurityService<R, A = NoopSecurityAuditLog, M = NoopRoleMembership, P = NoopSecurityEventPublisher>
where
    R: SecurityPolicyRepository,
    A: SecurityAuditLog,
    M: RoleMembershipRepository,
    P: SecurityEventPublisher;

impl<R: SecurityPolicyRepository> SecurityService<R> {
    pub fn new(repo: R) -> Self;
}

impl<R, A, M, P> SecurityService<R, A, M, P> {
    pub fn with_runtime_policy(self, policy: SecurityRuntimePolicy) -> Self;
    pub async fn authorize(
        &self,
        ctx: &WriteInvariantContext,
        resource: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<AuthorizationToken, SecurityError>;
    pub async fn authorize_contextual(
        &self,
        subject: &VerifiedPrincipal,
        req: &AuthzRequest,
        ctx: &SecurityContext,
    ) -> AuthzDecision;
}
```

Construtores:

- `new(repo)`: produção por omissão, sem audit/roles/eventos concretos.
- `with_audit(repo, audit)`: adiciona auditoria.
- `with_all(repo, audit, roles)`: adiciona memberships de roles.
- `with_publisher(repo, audit, roles, events)`: adiciona eventos de segurança.

### Política operacional

```rust
pub struct SecurityRuntimePolicy {
    pub bootstrap_authorization: BootstrapAuthorization,
    pub audit_failure: SecurityFailureMode,
    pub event_failure: SecurityFailureMode,
    pub sod_history_failure: SecurityFailureMode,
}

pub enum BootstrapAuthorization { Deny, Allow }
pub enum SecurityFailureMode { FailClosed, BestEffort }
```

`SecurityRuntimePolicy::production()` é o default:

- sem políticas activas, a autorização é negada;
- falha de auditoria bloqueia autorização;
- falha de publicação de evento relevante bloqueia autorização;
- falha de histórico SoD bloqueia a operação sensível.

`SecurityRuntimePolicy::bootstrap_permissive()` existe para bootstrap controlado,
testes e migrações assistidas. Não deve ser usado em produção sem janela operacional
explícita, owner e evidência externa.

### Políticas

```rust
pub struct Policy {
    pub policy_id: String,
    pub version: String,
    pub mode: PolicyMode,
    pub rules: Vec<Rule>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
}

pub enum PolicyMode {
    Baseline,
    Strict,
}
```

As políticas são soberanas e globais ao repositório injectado. `Strict` faz
deny-by-default para operações sem delegação. `Baseline` permite operações não
governadas por rule explícita, mas uma rule `enabled = true` exige delegação.
Uma rule `enabled = false` funciona como isenção explícita.

### Delegações

Delegações são temporais, revogáveis em cascata e podem apontar para uma delegação
pai via `granted_via`. `resource = None` significa delegação para qualquer recurso
da operação. `resource = Some(x)` exige correspondência exacta.

Condições opcionais em JSON são avaliadas por `authorize_contextual()`:

- `required_state`;
- `required_classification`;
- `required_org_unit`.

O método simples `authorize()` não recebe atributos de recurso; se encontrar uma
delegação condicional aplicável, nega e indica o uso de `authorize_contextual()`.

### Autorização contextual

`AuthzRequest` adiciona atributos ABAC, canal, correlação e nível mínimo de
autenticação. `SecurityContext` transporta principal autenticado, `AuthLevel`,
âmbito orgânico, sessão e decisão institucional correlacionada.

`AuthzDecision` é sempre explicável:

- `outcome`: allow/deny;
- `code`: código estruturado;
- `reason`: razão legível;
- `policy_id`: decisão/política correlacionada quando aplicável;
- `evidence_level`: none/normal/enhanced.

### Portos

- `SecurityPolicyRepository`: políticas e delegações.
- `SecurityAuditLog`: decisões de autorização.
- `RoleMembershipRepository`: roles activos de um principal.
- `SecurityEventPublisher`: eventos técnicos de segurança.
- `OrgScopeValidator`: validação de âmbito orgânico em adapters.
- `SodHistoryProvider`: histórico necessário para SoD.

## Invariantes

- `AuthorizationToken` só é construído pelo serviço.
- Produção é deny-by-default sem políticas activas.
- Bootstrap permissivo é opt-in por `SecurityRuntimePolicy`.
- Falhas de evidência/observabilidade são fail-closed em produção.
- Delegações expiradas ou revogadas não autorizam.
- Revogação de delegação propaga-se a descendentes.
- Condições malformadas negam.
- High classification eleva `EvidenceLevel` para `Enhanced`.
- Erros públicos são tipados e não devem expor segredos, tokens ou backends.

## Exemplo mínimo de produção

```rust
let repo = SecuritySqliteStore::open(&config)?;
let audit = SecuritySqliteStore::open(&config)?;
let roles = RhSecurityBridgeStore::open(&config)?;

let svc = SecurityService::with_all(repo, audit, roles);

let token = svc
    .authorize(&ctx, Some("doc-42"), Utc::now())
    .await?;
```

## Exemplo de bootstrap controlado

```rust
let svc = SecurityService::new(repo)
    .with_runtime_policy(SecurityRuntimePolicy::bootstrap_permissive());
```

Usar apenas durante inicialização controlada. A app host deve registar a janela,
o operador, a razão e a transição para `SecurityRuntimePolicy::production()`.

## Limites actuais

- Não autentica identidades; recebe `VerifiedPrincipal` já verificado.
- Não implementa multi-tenant por si; isolar por repositório/configuração.
- Não implementa wildcards em operações ou recursos.
- Não executa jobs de expiração; filtra por tempo no momento da decisão.
- Auditoria probatória forte depende de adapter/core-audit com hash chain/WORM.

## Validação

```sh
cargo test -p core-security
cargo clippy -p core-security --all-targets -- -D warnings
```

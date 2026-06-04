# core-security

Autorização zero-trust com políticas soberanas, delegações e auditoria de decisões.

## Objectivo

Define o contrato de domínio para controlo de acesso baseado em políticas (PBAC/ABAC): avaliação de regras soberanas, delegações temporais, roles, segregação de funções, auditoria de decisões e eventos técnicos de segurança.

## Posição arquitectural

`crates/kernel/core` — domínio puro. Não depende de SQLite nem de qualquer infraestrutura concreta. Os adapters concretos estão em `crates/kernel/infra/security-sqlite` e `crates/kernel/infra/rh-security-bridge`.

## Responsabilidade

- Tipos `VerifiedPrincipal`, `PrincipalKind` (Human/System).
- Port `SecurityPolicyRepository` (leitura/escrita de políticas e delegações).
- Port `SecurityAuditLog` (registo de decisões).
- Port `RoleMembershipRepository` (atribuição de roles a principals).
- `SecurityService` — serviço de domínio async com autorização simples e contextual.
- `SecurityRuntimePolicy` — modo operacional de produção ou bootstrap controlado.
- `SecurityEventPublisher`, `OrgScopeValidator` e `SodHistoryProvider` — portos hexagonais para SIEM, RH/organização e auditoria.
- Implementações em-memória para testes: `InMemorySecurityPolicyRepository`, `InMemoryRoleMembership`, `InMemoryAuditLog`.
- Modos de política: `Baseline` (permissivo por omissão) e `Strict` (restritivo por omissão).

## Não-responsabilidade

- Não persiste nada — toda a persistência é injectada via ports.
- Não autentica principals (assume que o caller já verificou a identidade).
- Não conhece JWT, OAuth, LDAP nem qualquer protocolo de identidade.

## Exemplo mínimo

```rust
use core_security::{SecurityService, VerifiedPrincipal, SecurityPolicyRepository};

let repo = InMemorySecurityPolicyRepository::new();
let audit = InMemoryAuditLog::new();
let roles = InMemoryRoleMembership::new();

let svc = SecurityService::with_all(repo, audit, roles);
let principal = VerifiedPrincipal::human("user-1");
let token = svc.authorize(&ctx, Some("doc-42"), Utc::now()).await?;
```

Por omissão, o serviço usa `SecurityRuntimePolicy::production()`: sem políticas activas, a autorização é negada; falhas de auditoria/eventos/SoD são tratadas como fail-closed. Para bootstrap controlado usar explicitamente `SecurityRuntimePolicy::bootstrap_permissive()`.

## Validação

```sh
cargo test -p core-security
```

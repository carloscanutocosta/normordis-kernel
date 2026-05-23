# core-security

Autorização zero-trust com políticas soberanas, delegações e auditoria de decisões.

## Objectivo

Define o contrato de domínio para controlo de acesso baseado em políticas (PBAC): verificação de principal, avaliação de regras, concessão/revogação de delegações e registo imutável de decisões de autorização.

## Posição arquitectural

`crates/kernel/core` — domínio puro. Não depende de SQLite nem de qualquer infraestrutura concreta. Os adapters concretos estão em `crates/kernel/infra/security-sqlite` e `crates/kernel/infra/rh-security-bridge`.

## Responsabilidade

- Tipos `VerifiedPrincipal`, `PrincipalKind` (Human/System).
- Port `SecurityPolicyRepository` (leitura/escrita de políticas e delegações).
- Port `SecurityAuditLog` (registo de decisões).
- Port `RoleMembershipRepository` (atribuição de roles a principals).
- `SecurityService` — serviço de domínio com lógica de autorização em 4 gates.
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

let svc = SecurityService::new(repo, audit, roles);
let principal = VerifiedPrincipal::human("user-1");
let token = svc.authorize(&principal, "document.read", "doc-42")?;
```

## Validação

```sh
cargo test -p core-security
```

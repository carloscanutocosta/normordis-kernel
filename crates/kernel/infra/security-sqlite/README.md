# security-sqlite

Adaptador SQLite para `core-security` — persistência de políticas, delegações e auditoria de decisões.

## Objectivo

Implementa `SecurityPolicyRepository` e `SecurityAuditLog` de `core-security` sobre SQLite, com suporte a revogação em cascata via CTE recursiva.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite`, `core-security` e `rusqlite`.

## Responsabilidade

- Persistir políticas de segurança por principal.
- Gerir delegações com cadeia `granted_via` e revogação em cascata.
- Registar todas as decisões de autorização (`AuditDecision`).
- Expor `SECURITY_SQLITE_MIGRATIONS` para uso externo.

## Não-responsabilidade

- Não implementa lógica de autorização — a lógica está em `core-security::SecurityService`.
- Não gere roles de RH — use `rh-security-bridge` para isso.
- Não encripta a base de dados.

## Exemplo mínimo

```rust
use security_sqlite::SecuritySqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_security::{SecurityService, InMemoryRoleMembership, NoopSecurityAuditLog};

let config = SqliteRelationalConfig::read_write_create("security.db");
let store = SecuritySqliteStore::open(&config)?;
let svc = SecurityService::new(store, NoopSecurityAuditLog, InMemoryRoleMembership::new());
```

## Validação

```sh
cargo test -p security-sqlite
```

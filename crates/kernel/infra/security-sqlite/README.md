# security-sqlite

Adaptador SQLite para `core-security` — persistência de políticas, delegações e auditoria de decisões com cadeia de hash local.

## Objectivo

Implementa `SecurityPolicyRepository` e `SecurityAuditLog` de `core-security` sobre SQLite, com revogação em cascata via CTE recursiva e verificação de integridade operacional da auditoria.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite`, `core-security` e `rusqlite`.

## Responsabilidade

- Persistir políticas soberanas globais ao repositório injectado.
- Gerir delegações com cadeia `granted_via` e revogação em cascata.
- Registar decisões de autorização (`SecurityAuthDecision`) para auditoria operacional.
- Verificar a cadeia de hash local com `verify_audit_chain()`.
- Expor `SECURITY_SQLITE_MIGRATIONS` para uso externo.

## Não-responsabilidade

- Não implementa lógica de autorização — a lógica está em `core-security::SecurityService`.
- Não gere roles de RH — use `rh-security-bridge` para isso.
- Não encripta a base de dados.
- Não substitui WORM, assinatura externa ou custódia probatória em core-audit.

## Exemplo mínimo

```rust
use security_sqlite::SecuritySqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_security::SecurityService;

let config = SqliteRelationalConfig::read_write_create("security.db");
let repo = SecuritySqliteStore::open(&config)?;
let audit = SecuritySqliteStore::open(&config)?;
let svc = SecurityService::with_audit(repo, audit);
```

## Validação

```sh
cargo test -p security-sqlite
```

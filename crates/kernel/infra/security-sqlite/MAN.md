# MAN — security-sqlite

## Objectivo

Persistência SQLite para o domínio de segurança e autorização. Implementa `SecurityPolicyRepository` e `SecurityAuditLog` de `core-security` com revogação em cascata de delegações via CTE recursiva.

---

## Contrato público

```rust
pub struct SecuritySqliteStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SecuritySqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, SecurityError>;
}

/// Migrações SQL exportadas para uso externo.
pub const SECURITY_SQLITE_MIGRATIONS: &[&str];
```

`SecuritySqliteStore` implementa:

```rust
impl SecurityPolicyRepository for SecuritySqliteStore { ... }
impl SecurityAuditLog for SecuritySqliteStore { ... }
```

---

## Schema (3 migrações)

**Migração 1 — Políticas:**
```sql
CREATE TABLE IF NOT EXISTS security_policies (
    PrincipalId  TEXT PRIMARY KEY,
    PolicyJson   TEXT NOT NULL,
    UpdatedAt    TEXT NOT NULL
);
```

**Migração 2 — Delegações:**
```sql
CREATE TABLE IF NOT EXISTS security_delegations (
    Id              TEXT PRIMARY KEY,
    FromPrincipal   TEXT NOT NULL,
    ToPrincipal     TEXT NOT NULL,
    ActionCode      TEXT NOT NULL,
    ResourcePattern TEXT,
    GrantedAt       TEXT NOT NULL,
    ExpiresAt       TEXT,
    GrantedVia      TEXT,          -- FK para outra delegação (cadeia)
    RevokedAt       TEXT,
    RevokedBy       TEXT,
    RevocationReason TEXT
);
CREATE INDEX IF NOT EXISTS idx_delegations_to ON security_delegations(ToPrincipal);
CREATE INDEX IF NOT EXISTS idx_delegations_from ON security_delegations(FromPrincipal);
```

**Migração 3 — Auditoria:**
```sql
CREATE TABLE IF NOT EXISTS security_auth_decisions (
    Id          TEXT PRIMARY KEY,
    Principal   TEXT NOT NULL,
    Action      TEXT NOT NULL,
    Resource    TEXT,
    Decision    TEXT NOT NULL,  -- 'Granted' | 'Denied'
    GrantedBy   TEXT,
    DecidedAt   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_auth_decisions_principal ON security_auth_decisions(Principal);
```

---

## Revogação em cascata

`revoke_delegation` usa uma CTE recursiva para revogar automaticamente todas as delegações cuja cadeia `granted_via` inclua o id revogado:

```sql
WITH RECURSIVE cascade(id) AS (
    SELECT ?1
    UNION ALL
    SELECT d.Id FROM security_delegations d
    INNER JOIN cascade c ON d.GrantedVia = c.id
    WHERE d.RevokedAt IS NULL
)
UPDATE security_delegations
SET RevokedAt = ?2, RevokedBy = ?3, RevocationReason = ?4
WHERE Id IN (SELECT id FROM cascade);
```

---

## Como usar

```rust
use security_sqlite::SecuritySqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_security::{SecurityService, InMemoryRoleMembership, NoopSecurityAuditLog};

let config = SqliteRelationalConfig::read_write_create("security.db");
let repo = SecuritySqliteStore::open(&config)?;

// Para auditoria persistente, usar também SecuritySqliteStore como audit log:
let audit = SecuritySqliteStore::open(&config)?;

let svc = SecurityService::new(repo, audit, InMemoryRoleMembership::new());
```

---

## Invariantes

- As migrações são idempotentes (`CREATE TABLE IF NOT EXISTS`).
- `open()` executa as 3 migrações em sequência antes de devolver o store.
- A ligação é partilhada via `Arc<Mutex<Connection>>` — thread-safe.
- Delegações revogadas não são eliminadas — ficam marcadas com `RevokedAt`.

---

## Limites actuais

- Sem encriptação da base de dados.
- Auditoria cresce indefinidamente (sem purge/rotate automático).
- A CTE recursiva de revogação em cascata requer que SQLite tenha `RECURSIVE` habilitado (padrão desde 3.8.3).

---

## ToDo

- [ ] Purge de registos de auditoria por política de retenção.
- [ ] Índice composto `(ToPrincipal, ActionCode)` para queries de autorização frequentes.

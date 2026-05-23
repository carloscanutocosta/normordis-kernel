# MAN — rh-security-bridge

## Objectivo

Ponte SQLite entre o domínio de RH/organização e o sistema de autorização de `core-security`. Persiste memberships principal→role com temporalidade e auditoria de atribuição.

---

## Contrato público

```rust
pub struct RhSecurityBridgeStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl RhSecurityBridgeStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, SecurityError>;

    /// Atribui um role a um principal, com validade opcional.
    pub fn assign_principal_to_role(
        &self,
        principal_id: &str,
        role_id: &str,
        assigned_by: &str,
        valid_from: Option<DateTime<Utc>>,
        valid_to: Option<DateTime<Utc>>,
    ) -> Result<MemberId, SecurityError>;

    /// Revoga um membership pelo seu id.
    pub fn revoke_membership(
        &self,
        member_id: &MemberId,
        revoked_by: &str,
    ) -> Result<(), SecurityError>;

    /// Lista todos os roles activos de um principal (filtra por data actual).
    pub fn list_principal_roles(
        &self,
        principal_id: &str,
    ) -> Result<Vec<RoleId>, SecurityError>;

    /// Lista todos os membros activos de um role (filtra por data actual).
    pub fn list_role_members(
        &self,
        role_id: &RoleId,
    ) -> Result<Vec<String>, SecurityError>;
}

pub struct MemberId(pub String);

/// Migrações exportadas.
pub const RH_SECURITY_BRIDGE_MIGRATIONS: &[&str];
```

`RhSecurityBridgeStore` implementa `RoleMembershipRepository` de `core-security`.

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS security_role_members (
    MemberId     TEXT PRIMARY KEY,
    PrincipalId  TEXT NOT NULL,
    RoleId       TEXT NOT NULL,
    AssignedBy   TEXT NOT NULL,
    AssignedAt   TEXT NOT NULL,
    ValidFrom    TEXT,            -- NULL = imediato
    ValidTo      TEXT,            -- NULL = sem expiração
    RevokedAt    TEXT,
    RevokedBy    TEXT
);
CREATE INDEX IF NOT EXISTS idx_role_members_principal ON security_role_members(PrincipalId);
CREATE INDEX IF NOT EXISTS idx_role_members_role ON security_role_members(RoleId);
```

---

## Como usar

```rust
use rh_security_bridge::RhSecurityBridgeStore;
use core_security::{SecurityService, SecuritySqliteStore};

let bridge = RhSecurityBridgeStore::open(&config)?;
let repo = SecuritySqliteStore::open(&config)?;
let audit = NoopSecurityAuditLog;

// Injectar bridge como RoleMembershipRepository
let svc = SecurityService::new(repo, audit, bridge);

// Atribuir role com validade de 30 dias
let member_id = bridge.assign_principal_to_role(
    "trabalhador-1",
    "role-chefia",
    "rh-sistema",
    Some(Utc::now()),
    Some(Utc::now() + Duration::days(30)),
)?;
```

---

## Invariantes

- `list_principal_roles` e `list_role_members` filtram por `valid_from <= now <= valid_to` (ou NULL).
- Memberships revogados não são eliminados — ficam marcados com `RevokedAt`.
- `MemberId` é um UUID gerado internamente na atribuição.

---

## Limites actuais

- Sem sincronização automática com AD/LDAP.
- Sem notificação de expiração de memberships (sem background job).
- Roles são strings opacas — sem validação de existência do role.

---

## ToDo

- [ ] Job de expiração automática de memberships com `ValidTo` no passado.
- [ ] Suporte a importação batch de memberships a partir de CSV/LDAP.

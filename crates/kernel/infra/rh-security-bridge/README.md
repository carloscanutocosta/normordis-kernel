# rh-security-bridge

Ponte entre RH/organização e autorização — implementa `RoleMembershipRepository` e `OrgScopeValidator` sobre SQLite.

## Objectivo

Persiste relações principal→role com validade temporal e permite resolver principal IAM/técnico para pessoa RH antes de validar âmbito orgânico.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura e bridge entre domínios. Depende de `adapter-sqlite`, `core-security` e `rusqlite`.

## Responsabilidade

- Persistir atribuições de roles a principals (`assign_principal_to_role`).
- Revogar memberships com registo de quem revogou.
- Listar roles de um principal e membros de um role.
- Implementar `RoleMembershipRepository` para injecção em `SecurityService`.
- Implementar `OrgScopeValidator` consultando afectações RH.
- Mapear explicitamente `principal_id` para `person_id` quando estes IDs não coincidem.

## Não-responsabilidade

- Não define os roles — os roles são strings opacas geridas pelo domínio de negócio.
- Não avalia autorização — a avaliação é feita em `SecurityService`.
- Não sincroniza automaticamente com sistemas externos de identidade (AD, LDAP).
- Não cria nem governa a tabela RH `person_assignment`.

## Exemplo mínimo

```rust
use rh_security_bridge::RhSecurityBridgeStore;
use adapter_sqlite::SqliteRelationalConfig;
use chrono::Utc;
use core_security::RoleId;

let config = SqliteRelationalConfig::read_write_create("rh-bridge.db");
let bridge = RhSecurityBridgeStore::open(&config)?;
let now = Utc::now();

bridge.link_principal_to_person("aad:user-1", "person-1", "system", now)?;
bridge.assign_principal_to_role(
    "aad:user-1",
    &RoleId("role-admin".into()),
    "system",
    now,
    None,
    now,
)?;
```

## Validação

```sh
cargo test -p rh-security-bridge
```

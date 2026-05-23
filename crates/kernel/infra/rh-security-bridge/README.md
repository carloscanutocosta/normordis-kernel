# rh-security-bridge

Ponte entre o módulo de RH/org e o sistema de autorização — implementa `RoleMembershipRepository` sobre SQLite.

## Objectivo

Persiste as relações principal→role com suporte a validade temporal (`valid_from`/`valid_to`) e auditoria de quem atribuiu cada membership. Permite que o sistema de RH alimente o controlo de acesso sem dependência directa entre os dois domínios.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura e bridge entre domínios. Depende de `adapter-sqlite`, `core-security` (port `RoleMembershipRepository`) e `rusqlite`.

## Responsabilidade

- Persistir atribuições de roles a principals (`assign_principal_to_role`).
- Revogar memberships com registo de quem revogou.
- Listar roles de um principal e membros de um role.
- Implementar `RoleMembershipRepository` para injecção em `SecurityService`.

## Não-responsabilidade

- Não define os roles — os roles são strings opacas geridas pelo domínio de negócio.
- Não avalia autorização — a avaliação é feita em `SecurityService`.
- Não sincroniza automaticamente com sistemas externos de identidade (AD, LDAP).

## Exemplo mínimo

```rust
use rh_security_bridge::RhSecurityBridgeStore;
use adapter_sqlite::SqliteRelationalConfig;

let config = SqliteRelationalConfig::read_write_create("rh-bridge.db");
let bridge = RhSecurityBridgeStore::open(&config)?;
bridge.assign_principal_to_role("user-1", "role-admin", "system", None, None)?;
```

## Validação

```sh
cargo test -p rh-security-bridge
```

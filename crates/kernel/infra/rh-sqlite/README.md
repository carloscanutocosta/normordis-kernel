# rh-sqlite

Adapter SQLite para identidade e catálogo de roles do Mini-Kernel RS.

## Objectivo

Persistir utilizadores locais e o catálogo institucional de roles funcionais,
implementando os ports `UsersSqliteStore` (gestão de utilizadores e contexto corrente)
e `RoleRepository` (catálogo de roles para controlo de acesso a apps).

## Posição arquitectural

`crates/kernel/infra/rh-sqlite` — adapter de infra, sem semântica de domínio.

Depende de `core-rh` (tipos e ports). Usa `adapter-sqlite` para migração e ligação.

## Responsabilidade

- Persistência de utilizadores locais (`local_user`) e contexto corrente (`current_user_context`).
- Catálogo gerido de roles funcionais (`platform_roles`), com suporte a `is_active`.
- Implementação de `RoleRepository`: `get`, `list_active`, `exists_and_active`, `upsert`, `deactivate`.
- `exists_and_active` verifica existência e actividade numa só query — usado pelo `AppRegistryService`.
- Migrações idempotentes (2 entradas): utilizadores na migração 1, roles na migração 2.

## Não-responsabilidade

- Não implementa LDAP, OAuth, OIDC, SSO, passwords, tokens, rede, UI ou RBAC complexo.
- Não valida roles contra apps — responsabilidade do `AppRegistryService` do `domain-registry`.

## Exemplo mínimo

```rust
use adapter_sqlite::SqliteRelationalConfig;
use core_rh::{Role, RoleId, RoleRepository, UserIdentity, UserRole};
use rh_sqlite::UsersSqliteStore;

let store = UsersSqliteStore::open(
    &SqliteRelationalConfig::read_write_create(&db_path)
)?;

// Utilizadores
store.upsert_user(&UserIdentity {
    user_id:      "ana.silva".to_owned(),
    username:     "ana.silva".to_owned(),
    display_name: "Ana Silva".to_owned(),
    email:        None,
    role:         UserRole::Utilizador,
})?;

// Catálogo de roles
store.upsert(&Role::new("gestor_rh", "Gestor de RH", None, true)?)?;
store.upsert(&Role::new("admin", "Administrador", None, true)?)?;

let active = store.list_active()?;
let exists = store.exists_and_active(&RoleId::new("gestor_rh")?)?; // true
```

## Validação

```sh
cargo test -p rh-sqlite
```

# rh-sqlite

Adapter SQLite para identidade funcional/local do Mini-Kernel RS.

## Responsabilidade

Persiste utilizadores locais registaveis de `core-rh` e resolve o utilizador atual numa base SQLite local.

## Nao responsabilidade

Nao implementa LDAP, OAuth, OIDC, SSO, passwords, tokens, rede, UI ou RBAC complexo.

## Exemplo minimo

```rust
use adapter_sqlite::SqliteRelationalConfig;
use core_rh::{UserIdentity, UserRole};
use rh_sqlite::UsersSqliteStore;

let store = UsersSqliteStore::open(&SqliteRelationalConfig::read_write_create("users.db"))?;
store.upsert_user(&UserIdentity {
    user_id: "ana.silva".to_owned(),
    username: "ana.silva".to_owned(),
    display_name: "Ana Silva".to_owned(),
    email: None,
    role: UserRole::Utilizador,
})?;
# Ok::<(), rh_sqlite::UsersSqliteError>(())
```

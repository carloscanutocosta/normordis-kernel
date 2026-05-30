# adapter-registry-sqlite

Adapter SQLite para o catálogo institucional de apps (`domain-registry`).

## Objectivo

Implementar o port `AppRegistryRepository` sobre SQLite, com suporte a roles de acesso,
histórico de estados append-only, e construção do menu de utilizador por role.

## Posição arquitectural

`crates/kernel/infra/adapter-registry-sqlite` — adapter de infra, sem semântica de domínio.

Implementa `domain-registry::AppRegistryRepository`.
Utiliza `adapter-sqlite` para migração e abertura de ligação.
Não valida roles — essa responsabilidade pertence ao `AppRegistryService` do domínio.

## Responsabilidade

- Persistência do catálogo de apps (`platform_app_registry`).
- Histórico de estados datado, append-only (`platform_app_state_transitions`).
- Roles com acesso por app (`platform_app_allowed_roles`), com substituição atómica.
- Audit trail de roles (`platform_app_role_changes`): baseline no registo + cada mudança; lido via `role_change_log()`.
- Audit trail de metadados (`platform_app_metadata_changes`): uma linha por campo alterado (old→new); lido via `metadata_change_log()`.
- `register()` atómico via SAVEPOINT: registo + estado Draft + roles + baseline de auditoria num único commit.
- Retry sob `SQLITE_BUSY`/`LOCKED` em `register()`, `update_metadata()` e `set_allowed_roles()` (backoff 20→640ms, máx 5) — deployment multi-processo.
- `list_for_roles()` devolve apenas apps `Active`: apps sem restrição + apps com role do utilizador.
- `list()` com filtros SQL (estado, domain, owner, visibility, name_contains com escape de LIKE).
- Batch loading de transições e roles em 2 queries para `list()` e `list_for_roles()`.

## Não-responsabilidade

- Não valida a máquina de estados — responsabilidade do `AppRegistryService`.
- Não valida existência de roles no catálogo — responsabilidade do `AppRegistryService`.
- Não auditoria mudanças de metadados — pendente tabela `platform_app_metadata_log`.
- Não implementa `domain-registry::RoleRepository` — esse port pertence a `rh-sqlite`.

## Exemplo mínimo

```rust
use adapter_registry_sqlite::RegistrySqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use domain_registry::{AppId, AppRegistryRepository, AppRegistryFilter, AppVisibility,
                      RegisterAppRequest, AppState, TransitionStateRequest};

let store = RegistrySqliteStore::open(
    &SqliteRelationalConfig::read_write_create(&db_path)
)?;

// Registo (atómico: registry + Draft + roles)
store.register(&RegisterAppRequest {
    id:            AppId::new("gestao-rh")?,
    name:          "Gestão RH".into(),
    version:       "1.0.0".into(),
    owner:         "equipa-rh".into(),
    domain:        "rh".into(),
    description:   None,
    capabilities:  vec!["pessoas".into()],
    visibility:    AppVisibility::Internal,
    allowed_roles: vec![],
    registered_by: "admin".into(),
}, Utc::now())?;

// Menu para utilizador com role gestor_rh
let menu = store.list_for_roles(&[RoleId::new("gestor_rh")?], 200)?;
```

## Validação

```sh
cargo test -p adapter-registry-sqlite
```

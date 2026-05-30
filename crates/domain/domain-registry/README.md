# domain-registry

Catálogo institucional de apps do workspace — registo, ciclo de vida, visibilidade e controlo de acesso por role.

## Objectivo

Disponibilizar um agregado de domínio que registe as apps disponíveis no workspace, gira o seu ciclo de vida por uma máquina de estados institucional, e controle quais os roles de utilizador que têm acesso a cada app. O resultado directo é a capacidade de construir o menu de navegação do workspace de forma declarativa, com base nos roles do utilizador autenticado.

## Posição arquitectural

`crates/domain/domain-registry` — domínio transversal, sem I/O, sem UI.

Depende de `core-rh` (para `RoleId` e validação contra o catálogo de roles).
Implementado por `adapter-registry-sqlite` em `kernel/infra`.

## Responsabilidade

- Tipo `AppId` validado como identificador único de uma app.
- Registo imutável com `AppRegistration`: metadados, visibilidade, roles permitidos, histórico de estados.
- Máquina de estados institucional: `Draft → Experimental → Active → Suspended → Deprecated → Retired`.
- Histórico de estados append-only e datado (`AppStateTransition`).
- Controlo de acesso por role: `allowed_roles` vazio = acesso livre; não-vazio = acesso restrito.
- Método `is_accessible_to(&[RoleId])` para verificar acesso localmente, sem BD.
- Port `AppRegistryRepository` com `list_for_roles` para construção do menu de utilizador.
- Serviço `AppRegistryService<R, L>` que valida roles contra o catálogo `L: RoleRepository` antes de persistir.
- Filtro `AppRegistryFilter` com pesquisa por `name_contains` (substring, case-insensitive).

## Não-responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não implementa autenticação nem autorização de operações — só declara controlo de acesso ao catálogo.
- Não define o catálogo de roles — esse pertence a `core-rh`.
- Não gere visibilidade por unidade orgânica — esse nível pertence ao service layer da app.

## Exemplo mínimo

```rust
use domain_registry::{AppId, AppRegistryService, AppVisibility, RegisterAppRequest};
use core_rh::RoleId;

// Registar app com acesso restrito a gestores
let svc = AppRegistryService::new(registry_store, role_store);

svc.register(RegisterAppRequest {
    id:            AppId::new("gestao-rh")?,
    name:          "Gestão RH".into(),
    version:       "1.0.0".into(),
    owner:         "equipa-rh".into(),
    domain:        "rh".into(),
    description:   None,
    capabilities:  vec!["pessoas".into(), "contratos".into()],
    visibility:    AppVisibility::Internal,
    allowed_roles: vec![RoleId::new("gestor_rh")?, RoleId::new("admin")?],
    registered_by: "admin".into(),
}, Utc::now())?;

// Construir menu para utilizador com role gestor_rh
let menu = svc.list_for_roles(&[RoleId::new("gestor_rh")?], 100)?;
```

## Validação

```sh
cargo test -p domain-registry
cargo test -p adapter-registry-sqlite
```

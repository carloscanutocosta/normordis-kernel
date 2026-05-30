# MAN — domain-registry

## Objectivo

`domain-registry` é o catálogo institucional de apps do workspace. Responde a "que apps existem,
quem pode vê-las e em que estado estão?". É o mecanismo de governação que permite:

- Registar apps de forma controlada, com histórico imutável de estados.
- Construir o menu de navegação de cada utilizador com base nos seus roles.
- Gerir o ciclo de vida das apps por uma máquina de estados institucional.
- Actualizar metadados operacionais sem perda do histórico.

Depende de `core-rh` para `RoleId` e para a validação de roles contra o catálogo gerido.
É implementado por `adapter-registry-sqlite` em `kernel/infra`.

---

## Contrato público

### Erros

```rust
pub enum RegistryError {
    EmptyField(&'static str),          // campo obrigatório vazio
    AppNotFound(String),               // app não existe no catálogo
    AppAlreadyRegistered(String),      // tentativa de registar id já existente
    InvalidStateTransition {           // transição não permitida pela máquina de estados
        from: String,
        to:   String,
    },
    TerminalState(String),             // app em Retired — nenhuma transição permitida
    RoleNotFound(String),              // role não existe no catálogo
    RoleInactive(String),              // role existe mas está inactivo (is_active = false)
    Storage(String),                   // erro de infra (propagado pelo adapter)
}
```

---

### AppId

```rust
pub struct AppId(String);

AppId::new(s: impl Into<String>) -> Result<Self, RegistryError>
// Rejeita vazio ou só-espaços. Trimmed.

appid.as_str() -> &str
appid.to_string()  // via Display
```

---

### AppState — máquina de estados

```rust
pub enum AppState {
    Draft,        // em definição, não disponível
    Experimental, // disponível em ambiente controlado, em validação
    Active,       // em produção, disponível para uso geral
    Suspended,    // temporariamente indisponível
    Deprecated,   // em fase de substituição, não recomendada para novos usos
    Retired,      // retirada de serviço definitivamente (terminal)
}
```

**Transições válidas:**

| Estado actual | Transições permitidas |
|---|---|
| `Draft` | `Experimental`, `Active`, `Retired` |
| `Experimental` | `Active`, `Suspended`, `Retired` |
| `Active` | `Suspended`, `Deprecated`, `Retired` |
| `Suspended` | `Active`, `Deprecated`, `Retired` |
| `Deprecated` | `Retired` |
| `Retired` | *(terminal — nenhuma transição possível)* |

```rust
AppState::is_terminal()                          -> bool
AppState::valid_transitions()                    -> &[AppState]
AppState::can_transition_to(target: &AppState)   -> bool
AppState::as_str()                               -> &str   // "Draft", "Active", …
AppState::from_str(s: &str)                      -> Result<Self, RegistryError>
```

---

### AppStateTransition

Registo datado e imutável de uma transição de estado.
O conjunto ordenado por `transitioned_at ASC` é a fonte de verdade do estado actual.

```rust
pub struct AppStateTransition {
    pub state:           AppState,
    pub transitioned_at: DateTime<Utc>,
    pub transitioned_by: String,        // identificador de quem transitou
    pub reason:          Option<String>,
}
```

Nunca é alterado nem apagado após persistência.

---

### AppVisibility

Classificação organizacional da app — determina o universo base de utilizadores.
O controlo de acesso operacional é feito por `allowed_roles`.

```rust
pub enum AppVisibility {
    Public,   // disponível a todos os utilizadores autenticados
    Internal, // disponível apenas a utilizadores internos
}
```

**Regra de acesso unificada:**

| `AppVisibility` | `allowed_roles` | Quem pode ver |
|---|---|---|
| `Public` | `[]` | Todos os autenticados |
| `Public` | `[role_a, …]` | Só utilizadores com pelo menos um dos roles |
| `Internal` | `[]` | Todos os utilizadores internos |
| `Internal` | `[role_b, …]` | Só os roles listados, entre os internos |

Quando `allowed_roles` não está vazio, substitui a visibilidade como critério de acesso.
`AppVisibility::Restricted` foi eliminado — o acesso restrito é expresso pelos roles.

---

### RoleId

Re-exportado de `core-rh`. Identificador único de um role funcional no catálogo institucional.

```rust
pub use core_rh::RoleId;

RoleId::new(s: impl Into<String>) -> Result<Self, core_rh::RhError>
// Rejeita vazio e espaços em branco. Exemplos: "gestor_rh", "admin", "chefe_divisao".

roleid.as_str() -> &str
roleid.to_string()
```

---

### AppRegistration

Registo completo de uma app no catálogo.

```rust
pub struct AppRegistration {
    pub id:            AppId,
    pub name:          String,
    pub version:       String,
    pub owner:         String,              // equipa ou responsável
    pub domain:        String,              // domínio de negócio ("rh", "documental", …)
    pub description:   Option<String>,
    pub capabilities:  Vec<String>,         // capacidades declaradas pela app
    pub visibility:    AppVisibility,
    pub allowed_roles: Vec<RoleId>,         // vazio = acesso livre
    pub registered_at: DateTime<Utc>,
    pub registered_by: String,
    pub state_history: Vec<AppStateTransition>,  // ordenado por transitioned_at ASC
}
```

```rust
app.current_state()                   -> Option<&AppState>
app.is_active()                       -> bool
app.is_accessible_to(roles: &[RoleId]) -> bool
// Verdadeiro se allowed_roles estiver vazio OU o utilizador tiver pelo menos um role listado.
// Pode ser chamado sem acesso à BD.
```

---

### RegisterAppRequest

```rust
pub struct RegisterAppRequest {
    pub id:            AppId,
    pub name:          String,
    pub version:       String,
    pub owner:         String,
    pub domain:        String,
    pub description:   Option<String>,
    pub capabilities:  Vec<String>,
    pub visibility:    AppVisibility,
    pub allowed_roles: Vec<RoleId>,     // validados contra o catálogo pelo serviço
    pub registered_by: String,
}

request.validate() -> Result<(), RegistryError>
// Verifica campos obrigatórios: name, version, owner, domain, registered_by.
// Não valida roles — responsabilidade do AppRegistryService.
```

---

### TransitionStateRequest

```rust
pub struct TransitionStateRequest {
    pub app_id:          AppId,
    pub to_state:        AppState,
    pub transitioned_by: String,
    pub reason:          Option<String>,
}

request.validate() -> Result<(), RegistryError>
// Verifica que transitioned_by não é vazio.
```

---

### UpdateAppMetadataRequest

Actualização parcial de metadados. Apenas os campos `Some(…)` são alterados.

```rust
pub struct UpdateAppMetadataRequest {
    pub app_id:       AppId,
    pub version:      Option<String>,
    pub description:  Option<Option<String>>,  // Some(None) = limpa o campo
    pub capabilities: Option<Vec<String>>,
    pub visibility:   Option<AppVisibility>,
    pub owner:        Option<String>,
    pub updated_by:   String,                  // obrigatório
}

request.validate() -> Result<(), RegistryError>
```

---

### AppRegistryFilter

```rust
pub struct AppRegistryFilter {
    pub state:         Option<AppState>,
    pub domain:        Option<String>,
    pub owner:         Option<String>,
    pub visibility:    Option<AppVisibility>,
    pub name_contains: Option<String>,   // substring case-insensitive sobre o nome
}
// Default: todos os campos None (sem filtro)
```

---

### AppRegistryRepository (port)

```rust
pub trait AppRegistryRepository {
    type Error: From<RegistryError>;

    // ─── Escrita ─────────────────────────────────────────────────────────────

    fn register(
        &self,
        request: &RegisterAppRequest,
        registered_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;
    // Insere o registo base + estado inicial Draft + roles, atomicamente.

    fn transition(
        &self,
        request: &TransitionStateRequest,
        transitioned_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;
    // Acrescenta uma linha ao histórico (append-only).
    // A validação da máquina de estados é do AppRegistryService.

    fn update_metadata(
        &self,
        request: &UpdateAppMetadataRequest,
        updated_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;
    // SQL dinâmico — só actualiza os campos Some(…).
    // Cada campo efectivamente alterado (old != new) é registado no audit trail de metadados.

    fn set_allowed_roles(
        &self,
        app_id: &AppId,
        roles: &[RoleId],
        set_by: &str,
        set_at: DateTime<Utc>,
    ) -> Result<(), Self::Error>;
    // Substitui a lista de roles atomicamente (DELETE + INSERT em SAVEPOINT).
    // Lista vazia remove todas as restrições de role.

    // ─── Leitura ─────────────────────────────────────────────────────────────

    fn list_for_roles(
        &self,
        roles: &[RoleId],
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error>;
    // Para construção do menu: devolve apenas apps em estado Active.
    // Inclui apps sem restrição de role (allowed_roles vazio) + apps onde o utilizador tem acesso.
    // Se roles estiver vazio, devolve apenas apps Active sem restrição.

    fn get(&self, id: &AppId) -> Result<Option<AppRegistration>, Self::Error>;

    fn list(
        &self,
        filter: &AppRegistryFilter,
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error>;

    fn state_history(
        &self,
        id: &AppId,
    ) -> Result<Vec<AppStateTransition>, Self::Error>;
    // Devolve RoleNotFound se a app não existir.

    fn exists(&self, id: &AppId) -> Result<bool, Self::Error>;
}
```

---

### AppRegistryService

Ponto de entrada único. Valida semântica de domínio e roles antes de delegar ao repositório.
Genérico sobre `R: AppRegistryRepository` e `L: RoleRepository` (do `core-rh`).

```rust
pub struct AppRegistryService<R, L>
where
    R: AppRegistryRepository,
    L: RoleRepository,
    L::Error: Display,
{
    // …
}

AppRegistryService::new(repo: R, roles: L) -> Self

// ─── Escrita ─────────────────────────────────────────────────────────────────

svc.register(request: RegisterAppRequest, now: DateTime<Utc>) -> Result<(), R::Error>
// validate() + exists() + validate_roles() + repo.register()

svc.transition(request: TransitionStateRequest, now: DateTime<Utc>) -> Result<(), R::Error>
// validate() + get() + máquina de estados + repo.transition()

svc.update_metadata(request: UpdateAppMetadataRequest, now: DateTime<Utc>) -> Result<(), R::Error>
// validate() + exists() + repo.update_metadata() (regista audit por campo alterado)

svc.set_allowed_roles(app_id: AppId, roles: Vec<RoleId>, set_by: &str, now: DateTime<Utc>)
// set_by não-vazio + exists() + validate_roles() + repo.set_allowed_roles()

// ─── Leitura ─────────────────────────────────────────────────────────────────

svc.list_for_roles(roles: &[RoleId], limit: usize) -> Result<Vec<AppRegistration>, R::Error>
svc.get(id: &AppId)                                -> Result<Option<AppRegistration>, R::Error>
svc.list(filter: &AppRegistryFilter, limit: usize) -> Result<Vec<AppRegistration>, R::Error>
svc.list_active(limit: usize)                      -> Result<Vec<AppRegistration>, R::Error>
svc.history(id: &AppId)                            -> Result<Vec<AppStateTransition>, R::Error>
```

---

## Como usar

### 1. Registar uma app com acesso restrito

```rust
use domain_registry::{AppId, AppRegistryService, AppVisibility, RegisterAppRequest};
use core_rh::RoleId;

let svc = AppRegistryService::new(registry_store, role_store);

svc.register(RegisterAppRequest {
    id:            AppId::new("gestao-rh")?,
    name:          "Gestão RH".into(),
    version:       "1.0.0".into(),
    owner:         "equipa-rh".into(),
    domain:        "rh".into(),
    description:   Some("Gestão de pessoal e contratos".into()),
    capabilities:  vec!["pessoas".into(), "contratos".into(), "faltas".into()],
    visibility:    AppVisibility::Internal,
    allowed_roles: vec![RoleId::new("gestor_rh")?, RoleId::new("admin")?],
    registered_by: "admin".into(),
}, Utc::now())?;
// Rejeita se "gestor_rh" ou "admin" não existirem no catálogo de roles.
```

### 2. Construir o menu de utilizador

```rust
// O utilizador tem roles [gestor_rh]. Quais apps vê?
let roles = vec![RoleId::new("gestor_rh")?];
let menu = svc.list_for_roles(&roles, 200)?;

for app in &menu {
    println!("{} — {}", app.id, app.name);
}
// Devolve apenas apps em estado Active (navegáveis).
// Inclui sempre apps sem restrição de role (acesso livre).
// Inclui apps onde "gestor_rh" está em allowed_roles.
// Exclui apps restritas a outros roles, e apps em Draft/Suspended/Deprecated/Retired.

// Verificação local sem BD (para menus já carregados):
let visible: Vec<_> = apps.iter().filter(|a| a.is_accessible_to(&roles)).collect();
```

### 3. Gerir o ciclo de vida

```rust
use domain_registry::{AppState, TransitionStateRequest};

// Draft → Active (salta Experimental para apps simples)
svc.transition(TransitionStateRequest {
    app_id:          AppId::new("gestao-rh")?,
    to_state:        AppState::Active,
    transitioned_by: "admin".into(),
    reason:          Some("aprovada em validação".into()),
}, Utc::now())?;

// Mais tarde: Active → Deprecated
svc.transition(TransitionStateRequest {
    app_id:          AppId::new("gestao-rh")?,
    to_state:        AppState::Deprecated,
    transitioned_by: "admin".into(),
    reason:          Some("substituída pela versão 2.0".into()),
}, Utc::now())?;

// Consultar histórico completo
let history = svc.history(&AppId::new("gestao-rh")?)?;
for t in &history {
    println!("{} → {} por {}", t.transitioned_at, t.state, t.transitioned_by);
}
```

### 4. Actualizar metadados e roles

```rust
use domain_registry::UpdateAppMetadataRequest;

// Nova versão em produção — regista uma entrada de auditoria (version: 1.0.0 → 2.0.0)
svc.update_metadata(UpdateAppMetadataRequest {
    app_id:       AppId::new("gestao-rh")?,
    version:      Some("2.0.0".into()),
    description:  None,        // sem alteração
    capabilities: None,
    visibility:   None,
    owner:        None,
    updated_by:   "deploy-bot".into(),
}, Utc::now())?;

// Adicionar role de direcção após reorganização
svc.set_allowed_roles(
    AppId::new("gestao-rh")?,
    vec![RoleId::new("gestor_rh")?, RoleId::new("admin")?, RoleId::new("direccao")?],
    "admin",
    Utc::now(),
)?;
// Substitui a lista anterior atomicamente.
// Rejeita se "direccao" não existir no catálogo.
```

---

## Invariantes

- O estado inicial de uma app é sempre `Draft`, criado no momento do registo com o timestamp de registo.
- O histórico de estados é append-only — nunca se apaga nem altera uma transição.
- O estado actual é sempre o último elemento de `state_history` ordenado por `transitioned_at ASC`.
- Uma app em `Retired` não pode transitar para nenhum outro estado.
- Transições inválidas falham no serviço antes de chegar ao repositório.
- `allowed_roles` é substituído integralmente por `set_allowed_roles` — não há merge.
- Todos os roles em `allowed_roles` são validados antes de persistir: inexistente → `RoleNotFound`, inactivo → `RoleInactive`.
- `register()` e `set_allowed_roles()` são atómicos (SAVEPOINT no adapter SQLite) e com retry sob `SQLITE_BUSY`/`LOCKED` (backoff 20→640ms, máx 5 tentativas).
- O audit trail de roles é completo: `register()` regista a baseline inicial e cada `set_allowed_roles()` regista a mudança (`set_by`, `set_at`, snapshot dos roles).
- O audit trail de metadados regista cada campo efectivamente alterado (`old → new`, `changed_by`, `changed_at`); valores inalterados não geram entrada.
- `transitioned_by`, `set_by` e `updated_by` são obrigatórios — rastreabilidade institucional.
- `list_for_roles()` devolve apenas apps em estado `Active` — só apps navegáveis no menu.

---

## Limites actuais

- Sem paginação em `list()` — apenas `limit`. Para catálogos com centenas de apps, pode ser necessário `offset`.
- Roles são `Vec<RoleId>` não ordenado — a ordem de exibição no menu é responsabilidade da camada de apresentação.
- `domain` e `name` não são validados contra um vocabulário controlado — depende de convenção da equipa.
- Sem notificação de alterações — integração com observabilidade (telemetria) é responsabilidade da app consumidora.

---

## ToDo

- [ ] Paginação (`offset`) em `list()` para catálogos grandes.
- [ ] Filtro por role em `AppRegistryFilter` (apps visíveis para um dado role).
- [ ] Campo `deprecated_by: Option<AppId>` para registar qual a app que substitui uma deprecated.
- [ ] Validação de `domain` contra vocabulário controlado (core-org ou tabela própria).

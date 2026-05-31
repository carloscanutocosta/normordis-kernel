# core-config — Manual de Referência

`core-config` declara e valida perfis de configuração do Mini-Kernel RS. Não executa I/O,
não instancia adapters, não carrega ficheiros. A validação é sempre explícita e chamada pelo consumidor.

---

## Tipos públicos

### `MiniKernelProfile`

Perfil completo do kernel. Agrega todos os subperfis.

```rust
pub struct MiniKernelProfile {
    pub app: AppProfile,
    pub runtime: RuntimeProfile,
    pub storage: StorageProfiles,
    pub crypto: CryptoProfile,
    pub logging: LoggingProfile,
    pub audit: AuditProfile,
}
```

Todos os campos são `pub`. A validação é opt-in via `validate()`.

#### Factory methods

| Método | Ambiente | `offline_mode` | Crypto | Storage | Logging |
|---|---|---|---|---|---|
| `dev_default(base_dir)` | `Dev` | `true` | activa, `key_id = "dev-local-key"` | 4 × SQLite cifradas | activo |
| `prod(base_dir, key_id)` | `Prod` | `false` | activa, `key_id` fornecido pelo chamador | 4 × SQLite cifradas | activo |
| `test_memory()` | `Test` | `true` | inactiva | 2 × Memory não cifradas | inactivo |

**`dev_default(base_dir)`** — Conveniente para desenvolvimento local. Usa `"dev-local-key"` como
identificador de chave. **Não usar em produção.**

**`prod(base_dir, key_id)`** — O `key_id` é obrigatório e fornecido pelo operador. O campo
`app_id` e `display_name` usam valores de exemplo e devem ser sobrescritos pela aplicação.

**`test_memory()`** — Storage em memória, sem ficheiros, sem crypto. Adequado para testes unitários.

Os quatro storage profiles criados por `dev_default` e `prod`:

| Nome | Ficheiro | Propósito |
|---|---|---|
| `main` | `main.sqlite` | `StoragePurpose::Main` |
| `audit` | `audit.sqlite` | `StoragePurpose::Audit` |
| `documents` | `documents.sqlite` | `StoragePurpose::Documents` |
| `cache` | `cache.sqlite` | `StoragePurpose::Cache` |

#### Método `validate()`

```rust
fn validate(&self) -> Result<(), ConfigError>
```

Valida todos os subperfis individualmente e depois as invariantes cruzadas. Ver secção
[Regras de validação](#regras-de-validação).

---

### `AppProfile`

```rust
pub struct AppProfile {
    pub app_id: String,
    pub display_name: String,
    pub environment: Environment,
}
```

---

### `Environment`

```rust
#[non_exhaustive]
pub enum Environment { Dev, Test, Prod }
```

Serializa como string lowercase: `"dev"`, `"test"`, `"prod"`.
Valores em PascalCase (`"Dev"`) ou outros formatos são rejeitados na desserialização.

---

### `RuntimeProfile`

```rust
pub struct RuntimeProfile {
    pub profile_name: String,
    pub offline_mode: bool,
}
```

---

### `StorageProfiles`

```rust
pub struct StorageProfiles {
    pub default_profile: String,
    pub profiles: Vec<StorageProfile>,
}
```

`profile(name: &str) -> Option<&StorageProfile>` — pesquisa linear por nome.

---

### `StorageProfile`

```rust
pub struct StorageProfile {
    pub name: String,
    pub backend: StorageBackend,
    pub database_path: Option<PathBuf>,
    pub encrypted: bool,
    pub purpose: StoragePurpose,
}
```

---

### `StorageBackend`

```rust
#[non_exhaustive]
pub enum StorageBackend { Memory, Sqlite }
```

Serializa como `"memory"`, `"sqlite"`.

| Variante | `database_path` | `encrypted` |
|---|---|---|
| `Sqlite` | obrigatório, não vazio | qualquer |
| `Memory` | deve ser `None` | deve ser `false` |

**Nota sobre `database_path` em `Sqlite`:** aceita paths absolutos (o operador pode especificar
a localização exacta da base de dados). Ao contrário de `PathsConfig`, não existem guardas de
path traversal — esta validação é responsabilidade do bootstrap/host quando o perfil vem de
fonte externa.

---

### `StoragePurpose`

```rust
#[non_exhaustive]
pub enum StoragePurpose { Main, Audit, Documents, Cache, Temp, Other(String) }
```

Serializa como `"main"`, `"audit"`, `"documents"`, `"cache"`, `"temp"`, `{"other": "valor"}`.

`Other(String)` — o valor não pode ser vazio nem conter apenas whitespace.

---

### `CryptoProfile`

```rust
pub struct CryptoProfile {
    pub enabled: bool,
    pub key_id: Option<String>,
}
```

---

### `LoggingProfile`

```rust
pub struct LoggingProfile {
    pub enabled: bool,
    pub log_dir: Option<PathBuf>,
    pub file_name: String,
    pub max_file_size_mb: u64,
    pub max_files: usize,
    pub retention_days: u64,
}
```

Constantes de defeito exportadas: `DEFAULT_LOG_FILE_NAME`, `DEFAULT_MAX_FILE_SIZE_MB`,
`DEFAULT_MAX_FILES`, `DEFAULT_RETENTION_DAYS`.

Quando `enabled = false`, nenhum campo é validado.

---

### `AuditProfile`

```rust
pub struct AuditProfile {
    pub enabled: bool,
    pub namespace: String,
    pub storage_profile: String,
}
```

Constantes de defeito exportadas: `DEFAULT_AUDIT_NAMESPACE`, `DEFAULT_AUDIT_STORAGE_PROFILE`.

Quando `enabled = false`, nenhum campo é validado.

---

### `AppConfig` / `PathsConfig` / `AppOptions`

Configuração local simples, destinada a bootstrap de aplicação desktop.
Serializa e desserializa de JSON com `#[serde(default)]` em todos os campos.

```rust
pub struct AppConfig {
    pub paths: PathsConfig,
    pub options: AppOptions,
}

pub struct PathsConfig {
    pub database_dir: PathBuf,   // defeito: "database"
    pub data_dir: PathBuf,       // defeito: "assets"
    pub artifacts_dir: PathBuf,  // defeito: "artifacts"
    pub temp_dir: PathBuf,       // defeito: "tmp"
    pub logs_dir: PathBuf,       // defeito: "logs"
}

pub struct AppOptions {
    pub app_name: String,        // defeito: "miniapp"
    pub environment: Environment, // defeito: Environment::Dev
}
```

`PathsConfig` usa paths **relativos** a um `base_dir` fornecido por `resolve_paths`.
Paths absolutos e componentes `..` são rejeitados por `validate_app_config`.

---

### `ResolvedPaths`

```rust
pub struct ResolvedPaths {
    pub database_dir: PathBuf,
    pub data_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub logs_dir: PathBuf,
}
```

Produzido por `resolve_paths(base_dir, &config.paths)` — une `base_dir` com cada campo de
`PathsConfig`. Nunca executa I/O.

---

### `ConfigError`

```rust
#[non_exhaustive]
pub enum ConfigError {
    InvalidAppProfile { reason: String },
    InvalidRuntimeProfile { reason: String },
    InvalidStorageProfile { reason: String },
    DuplicateStorageProfile { name: String },
    MissingStorageProfile { name: String },
    InvalidCryptoProfile { reason: String },
    InvalidLoggingProfile { reason: String },
    InvalidAuditProfile { reason: String },
    InconsistentProfile { reason: String },
    MalformedJson { reason: String },
}
```

`reason` e `name` são detalhes internos — não expor ao utilizador final.
Usar `public_message()` para a mensagem pública e `code()` para o código canónico.
Converte infalivelmente para `MiniError` via `From<ConfigError>` ou `to_mini_error()`.

---

## Funções públicas

| Função | Descrição |
|---|---|
| `validate_profile(profile)` | Valida um `MiniKernelProfile` completo |
| `validate_app_profile(profile)` | Valida apenas `AppProfile` |
| `validate_runtime_profile(profile)` | Valida apenas `RuntimeProfile` |
| `validate_storage_profiles(storage)` | Valida `StorageProfiles` e cada `StorageProfile` |
| `validate_storage_profile(profile)` | Valida um único `StorageProfile` |
| `validate_crypto_profile(profile)` | Valida `CryptoProfile` |
| `validate_logging_profile(profile)` | Valida `LoggingProfile` (skip se disabled) |
| `validate_audit_profile(profile, storage)` | Valida `AuditProfile` (skip se disabled) |
| `validate_app_config(config)` | Valida `AppConfig` incluindo paths |
| `resolve_paths(base_dir, paths)` | Resolve `PathsConfig` relativamente a `base_dir` |
| `load_app_config_from_json_str(json)` | Desserializa `AppConfig` de JSON (sem validar) |
| `load_validated_app_config_from_json_str(json)` | Desserializa e valida `AppConfig` de JSON |
| `app_config_to_json_string(config)` | Serializa `AppConfig` para JSON indentado |
| `MiniKernelProfile::load_validated_from_json_str(json)` | Desserializa e valida `MiniKernelProfile` de JSON |

---

## Regras de validação

### AppProfile

| Campo | Regra |
|---|---|
| `app_id` | Obrigatório; apenas `[A-Za-z0-9_\-.]`; máx. 128 caracteres |
| `display_name` | Obrigatório (não branco); máx. 255 caracteres |

### RuntimeProfile

| Campo | Regra |
|---|---|
| `profile_name` | Obrigatório; sem whitespace; máx. 64 caracteres |

### StorageProfiles

| Regra |
|---|
| Deve conter pelo menos um profile |
| `default_profile` não pode estar em branco |
| `default_profile` deve corresponder ao `name` de um profile existente |
| Nomes de profiles devem ser únicos |

### StorageProfile

| Campo | Regra |
|---|---|
| `name` | Obrigatório; sem whitespace; sem `:`; máx. 64 caracteres |
| `backend = Sqlite` | `database_path` obrigatório e não vazio |
| `backend = Memory` | `database_path` deve ser `None` |
| `backend = Memory` | `encrypted` deve ser `false` |
| `purpose = Other(s)` | `s` não pode ser branco |

### CryptoProfile

| Condição | Regra |
|---|---|
| `enabled = true` | `key_id` obrigatório; sem whitespace; sem `:`; máx. 128 caracteres |
| `enabled = false` | Nenhuma regra adicional |

### LoggingProfile

Quando `enabled = false`, nenhuma regra é verificada.

| Campo | Regra |
|---|---|
| `log_dir` | Obrigatório quando enabled; não vazio |
| `file_name` | Obrigatório; sem `/`; sem `\` |
| `max_file_size_mb` | Maior que zero |
| `max_files` | Maior que zero |
| `retention_days` | Pelo menos 1 |

### AuditProfile

Quando `enabled = false`, nenhuma regra é verificada.

| Campo | Regra |
|---|---|
| `namespace` | Obrigatório; sem whitespace; sem `:`; máx. 128 caracteres |
| `storage_profile` | Obrigatório; sem whitespace; sem `:` |
| `storage_profile` | Deve corresponder a um profile existente em `StorageProfiles` |
| `storage_profile` | O profile referenciado deve ter `StoragePurpose::Audit` |

### Invariantes cruzadas (`validate_profile`)

| Invariante |
|---|
| Se qualquer `StorageProfile` tiver `encrypted = true`, então `crypto.enabled` deve ser `true` |

### AppConfig (`validate_app_config`)

| Campo | Regra |
|---|---|
| `options.app_name` | Obrigatório (não branco); máx. 128 caracteres |
| `paths.*` | Cada path deve ser relativo (sem root), sem componentes `..`, e não vazio |

---

## Política de paths

### `PathsConfig` — paths relativos, validados

`PathsConfig` usa caminhos **relativos** ao `base_dir` fornecido por `resolve_paths`.
`validate_app_config` aplica as seguintes regras a cada campo:

| Condição rejeitada | Detecção |
|---|---|
| Path vazio | `OsStr::is_empty()` |
| Path com root (`/`, `C:\`, `\\server\` …) | `Component::RootDir` |
| Prefixo de drive sem root (`C:evil`) | `Component::Prefix(_)` |
| Componente `..` (traversal para directório pai) | `Component::ParentDir` |

A verificação percorre `path.components()` num único passo, cobrindo todos os casos — incluindo
paths drive-relativos do Windows (prefixo sem root, e.g. `C:relativo`) que escapam a `has_root()`.

Isto garante que `resolve_paths(base_dir, paths)` nunca produz um caminho fora de `base_dir`,
independentemente do conteúdo do ficheiro JSON.

### `StorageProfile.database_path` — decisão de design

`database_path` em `StorageProfile` (SQLite) **aceita paths absolutos e relativos**.
Não existem guardas de traversal.

**Razão:** os factory methods (`dev_default`, `prod`) constroem `database_path` como
`base_dir.join("file.sqlite")` — sempre um path absoluto. Impor caminhos relativos aqui
impediria o uso normal.

**Responsabilidade:** quando `MiniKernelProfile` vem de fonte externa (JSON, rede, etc.), o
bootstrap/host é responsável por validar `database_path` antes de construir o perfil.
`MiniKernelProfile::load_validated_from_json_str` desserializa e valida o perfil, mas não
aplica restrições de path sobre `database_path`.

## Garantias de segurança

### Protecção contra path traversal em `AppConfig`

`validate_app_config` aplica as regras acima a todos os campos de `PathsConfig`.
`load_validated_app_config_from_json_str` é a forma segura de carregar `AppConfig`
de JSON externo numa única chamada.

### Memory storage não pode ser cifrado

`StorageBackend::Memory` com `encrypted = true` é rejeitado por `validate_storage_profile`.
A cifra em memória não tem semântica definida — a validação torna este estado impossível.

### Limites de comprimento

Todos os campos identificadores têm limites explícitos para prevenir abusos de configuração
(ex: `app_id` de 100 MB):

| Campo | Limite |
|---|---|
| `app_id` | 128 caracteres |
| `display_name` | 255 caracteres |
| `profile_name` | 64 caracteres |
| `key_id` | 128 caracteres |
| `storage profile name` | 64 caracteres |
| `namespace` | 128 caracteres |
| `app_name` | 128 caracteres |

---

## Formato JSON (`AppConfig`)

`AppConfig` serializa com `serde_json::to_string_pretty`. Todos os campos têm valores de
defeito — um objecto vazio `{}` é válido e usa os defaults.

```json
{
  "paths": {
    "database_dir": "database",
    "data_dir": "assets",
    "artifacts_dir": "artifacts",
    "temp_dir": "tmp",
    "logs_dir": "logs"
  },
  "options": {
    "app_name": "miniapp",
    "environment": "dev"
  }
}
```

`environment` serializa em **snake_case**: `"dev"`, `"test"`, `"prod"`.
Valores em PascalCase (`"Dev"`) ou outros formatos causam erro de desserialização.
Um objecto `{}` vazio é válido — todos os campos usam os seus defeitos.

**Política de campos desconhecidos:** `AppConfig`, `PathsConfig` e `AppOptions` rejeitam
campos desconhecidos (`#[serde(deny_unknown_fields)]`). Isto detecta erros de configuração
como `"data_directory"` em vez de `"data_dir"`. Campos ausentes continuam a usar defeitos.

**Carregamento:**
- `load_app_config_from_json_str` — desserializa, não valida. Para cenários onde o chamador
  precisa de inspecionar antes de validar.
- `load_validated_app_config_from_json_str` — forma recomendada para uso em produção.

**`MiniKernelProfile`** também implementa `Serialize`/`Deserialize`. `StorageBackend`
serializa como `"sqlite"`, `"memory"`; `StoragePurpose` como `"main"`, `"audit"`,
`"documents"`, `"cache"`, `"temp"`, `{"other": "valor"}`. Para carregar e validar de JSON:
`MiniKernelProfile::load_validated_from_json_str`. Não tem `deny_unknown_fields` — os
sub-tipos de `MiniKernelProfile` são construídos programaticamente e não são um contrato
de configuração externa estabilizado.

---

## Códigos de erro

| Variante | Código canónico |
|---|---|
| `InvalidAppProfile` | `MINI.CONFIG.INVALID_APP_PROFILE` |
| `InvalidRuntimeProfile` | `MINI.CONFIG.INVALID_RUNTIME_PROFILE` |
| `InvalidStorageProfile` | `MINI.CONFIG.INVALID_STORAGE_PROFILE` |
| `DuplicateStorageProfile` | `MINI.CONFIG.DUPLICATE_STORAGE_PROFILE` |
| `MissingStorageProfile` | `MINI.CONFIG.MISSING_STORAGE_PROFILE` |
| `InvalidCryptoProfile` | `MINI.CONFIG.INVALID_CRYPTO_PROFILE` |
| `InvalidLoggingProfile` | `MINI.CONFIG.INVALID_LOGGING_PROFILE` |
| `InvalidAuditProfile` | `MINI.CONFIG.INVALID_AUDIT_PROFILE` |
| `InconsistentProfile` | `MINI.CONFIG.INCONSISTENT_PROFILE` |
| `MalformedJson` | `MINI.CONFIG.MALFORMED_JSON` |

Componente: `core-config`.

---

## Estabilidade da API

Os enums públicos `ConfigError`, `StorageBackend`, `StoragePurpose` e `Environment` são marcados
com `#[non_exhaustive]`. Isto permite adicionar novas variantes em versões minor sem breaking
change para consumidores externos.

**`#[non_exhaustive]` aplica-se a todas as crates externas à crate definidora** — incluindo
outras crates do mesmo workspace. Qualquer crate que não seja `core-config` deve incluir um
braço `_ =>` em matches sobre estes enums:

```rust
// correcto em qualquer crate consumidora (incluindo no mesmo workspace)
match config_error {
    ConfigError::InvalidAppProfile { .. } => { ... }
    ConfigError::MissingStorageProfile { .. } => { ... }
    _ => { ... }  // obrigatório
}
```

Apenas dentro do próprio módulo de `core-config` os matches podem ser exaustivos sem `_ =>`.

---

## Fronteiras do crate

**O crate faz:**
- Declarar tipos de configuração
- Validar configurações (sem I/O)
- Serializar/desserializar `AppConfig` de/para JSON
- Resolver `PathsConfig` em `ResolvedPaths` (sem I/O)
- Converter erros para `MiniError`

**O crate não faz:**
- Abrir bases de dados ou criar adapters
- Criar directórios ou ficheiros
- Carregar ou gravar ficheiros de configuração (responsabilidade de `app-bootstrap`)
- Carregar segredos ou chaves criptográficas
- Executar wiring de runtime
- Validar a existência física de paths

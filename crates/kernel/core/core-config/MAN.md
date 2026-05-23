# core-config

## Contrato publico

O crate exporta:

- `MiniKernelProfile`
- `AppProfile`
- `RuntimeProfile`
- `StorageProfiles`
- `StorageProfile`
- `CryptoProfile`
- `LoggingProfile`
- `AuditProfile`
- `AppConfig`
- `PathsConfig`
- `AppOptions`
- `ResolvedPaths`
- `ConfigError`

`core-config` declara e valida. O runtime/bootstrap instancia e injeta adapters concretos.

Tambem exporta helpers puros para configuracao local simples:

- `validate_app_config`
- `resolve_paths`
- `load_app_config_from_json_str`
- `app_config_to_json_string`

## Como usar

```rust
use core_config::MiniKernelProfile;

let profile = MiniKernelProfile::dev_default("data");
profile.validate()?;
# Ok::<(), core_config::ConfigError>(())
```

`MiniKernelProfile::dev_default(base_dir)` declara quatro SQLite locais:

- `main.sqlite`
- `audit.sqlite`
- `documents.sqlite`
- `cache.sqlite`

`MiniKernelProfile::test_memory()` declara storage em memoria para testes.

## Invariantes e regras

- Campos obrigatorios de texto nao podem estar vazios nem conter apenas whitespace.
- `app_id` e obrigatorio e aceita apenas letras, numeros, `_`, `-` e `.`.
- `profile_name` e obrigatorio e nao pode conter whitespace.
- `StorageProfiles` deve ter pelo menos um profile.
- `default_profile` deve existir.
- Nomes de storage devem ser unicos, obrigatorios, sem whitespace e sem `:`.
- `StorageBackend::Sqlite` exige `database_path` nao vazio.
- `StorageBackend::Memory` exige `database_path = None`.
- Se algum storage tiver `encrypted = true`, `crypto.enabled` deve ser `true`.
- Se `crypto.enabled = true`, `key_id` e obrigatorio, sem whitespace e sem `:`.
- Logging ativo exige `log_dir` nao vazio.
- `file_name` de logging e obrigatorio e nao pode conter `/` nem `\`.
- Auditoria referencia um `storage_profile` existente.
- `storage_profile` de auditoria nao pode conter whitespace nem `:`.
- Namespace de auditoria e obrigatorio, sem whitespace e sem `:`.
- `AppConfig.options.app_name` e obrigatorio.
- `PathsConfig` nao aceita paths vazios.

## Erros

`ConfigError` converte para `MiniError` com componente `core-config`.

Codigos canonicos:

- `MINI.CONFIG.INVALID_APP_PROFILE`
- `MINI.CONFIG.INVALID_RUNTIME_PROFILE`
- `MINI.CONFIG.INVALID_STORAGE_PROFILE`
- `MINI.CONFIG.DUPLICATE_STORAGE_PROFILE`
- `MINI.CONFIG.MISSING_STORAGE_PROFILE`
- `MINI.CONFIG.INVALID_CRYPTO_PROFILE`
- `MINI.CONFIG.INVALID_LOGGING_PROFILE`
- `MINI.CONFIG.INVALID_AUDIT_PROFILE`
- `MINI.CONFIG.INCONSISTENT_PROFILE`

## Limitacoes atuais

- Nao resolve paths especificos por plataforma.
- Nao instancia SQLite, storage, crypto, logging nem audit.
- Nao valida existencia fisica de paths.
- Nao carrega configuracao a partir de ficheiros.
- Nao grava ficheiros JSON de configuracao; essa materializacao pertence ao bootstrap/infra.

## ToDo futuro

- Integrar o runtime/bootstrap para resolver `storage_profile` em adapters concretos.
- Criar loader externo no runtime/bootstrap para ler ficheiros e chamar validacao explicita.
- Fazer `core-audit` receber `AuditProfile` e `storage_profile` via runtime.

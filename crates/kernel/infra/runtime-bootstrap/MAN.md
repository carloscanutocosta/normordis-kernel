# Manual do modulo runtime-bootstrap

## Objetivo

Compor componentes do Mini-Kernel RS sem contaminar cores com detalhes de infraestrutura.

## KernelRuntime oficial

`KernelRuntime` e o caminho oficial novo para materializar um runtime local a partir
de `core_config::MiniKernelProfile`.

Regra arquitetural:

```text
core-config define perfis
runtime-bootstrap resolve e instancia
cores executam
```

## Uso

```rust
use runtime_bootstrap::KernelRuntime;

let profile = core_config::MiniKernelProfile::dev_default(data_dir);
let runtime = KernelRuntime::open(&profile, key_provider)?;

runtime.audit().record_event(event_type, actor, target, details)?;
runtime.shutdown()?;
```

`KernelRuntime::open`:

- valida o `MiniKernelProfile`;
- resolve `profile.audit.storage_profile`;
- abre `StorageBackend::Sqlite` ou `StorageBackend::Memory`;
- usa `profile.audit.namespace` em `AuditStoreConfig`;
- cria logger tecnico opcional quando `profile.logging.enabled = true`;
- nao cria runtime async, DI container, UI ou dependencia Tauri.

## Contrato publico

- `KernelRuntime::open(&MiniKernelProfile, keys) -> Result<KernelRuntime<_>, MiniError>`
- `KernelRuntime::audit() -> &AuditDbService<_>`
- `KernelRuntime::logger() -> Option<&dyn support_logging::TechnicalLogger>`
- `KernelRuntime::shutdown() -> Result<(), MiniError>`
- `AuditDbConfig`, `AuditDbRuntime`, `AuditDbService`, `AuditDbStore` e `AuditDbStorage`
  permanecem disponiveis por compatibilidade.

## Auditoria dedicada

`AuditDbRuntime` abre uma base `audit.db` e monta:

```text
SqliteRawStorage
JsonStorageCodec
CryptoStorageProtector
ProtectedStorage
StorageAuditStore
AuditService
```

## Uso legado compativel

```rust
use runtime_bootstrap::{AuditDbConfig, AuditDbRuntime};

let config = AuditDbConfig::from_data_dir(data_dir);
let runtime = AuditDbRuntime::open(config, key_provider)?;

runtime.service().record_event(event_type, actor, target, details)?;
runtime.shutdown()?;
```

## Garantias

- `audit.db` e separado da base funcional.
- O core nao depende de SQLite.
- Escritas de evento usam `support-storage`.
- `SqliteRawStorage` suporta escrita condicional atomica para rejeitar overwrite de eventos.
- `StorageBackend::Memory` permite testes e runtime dev sem SQLite.
- O logger tecnico de `support-logging` nao substitui `core-audit` e nao recebe eventos
  auditaveis automaticamente.
- `shutdown()` e idempotente para runtimes SQLite e Memory.
- Depois de `shutdown()`, novas escritas SQLite atraves do runtime falham de forma controlada
  no store de auditoria.

## Erros publicos

- `MINI.RUNTIME.INVALID_STORAGE_PROFILE`
- `MINI.RUNTIME.UNSUPPORTED_STORAGE_BACKEND`
- `MINI.RUNTIME.RUNTIME_OPEN_FAILED`
- `MINI.RUNTIME.AUDIT_RUNTIME_FAILED`
- `MINI.RUNTIME.LOGGING_RUNTIME_FAILED`

## Limites atuais

- Nao gere ciclo de vida de recovery passphrase.
- Nao importa automaticamente bases antigas criadas pelo adapter legado `support-audit-sqlite`.
- Export, assinatura e cadeia hash pertencem ao `core-audit`; este crate apenas compoe a infra.
- Ainda nao implementa plugin system, dynamic module loading, network sync, scheduler,
  metrics ou telemetry.

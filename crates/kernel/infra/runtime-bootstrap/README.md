# runtime-bootstrap

`runtime-bootstrap` compoe componentes de infra do Mini-Kernel RS.

O caminho oficial novo e `KernelRuntime + core_config::MiniKernelProfile`:

```text
core-config define perfis
runtime-bootstrap resolve e instancia
cores executam
```

Nesta fase o runtime resolve o storage profile de auditoria, abre SQLite ou Memory,
monta `ProtectedStorage -> StorageAuditStore -> AuditService` e expoe o servico de
auditoria.

## KernelRuntime

```rust
use runtime_bootstrap::KernelRuntime;

let profile = core_config::MiniKernelProfile::dev_default(data_dir);
let runtime = KernelRuntime::open(&profile, key_provider)?;

runtime.audit().record_event(event_type, actor, target, details)?;
runtime.shutdown()?;
```

## Compatibilidade

`AuditDbConfig` e `AuditDbRuntime` continuam disponiveis para abrir uma base SQLite
dedicada de auditoria local:

```text
audit.db
  adapter-sqlite::SqliteRawStorage
  support-storage::ProtectedStorage
  core-audit::StorageAuditStore
  core-audit::AuditService
```

`audit.db` e uma base SQLite dedicada a evidencia institucional/local auditavel.

O `core-audit` continua sem conhecer SQLite. A decisao de abrir storage concreto
pertence ao bootstrap/runtime.

## Fronteira

```text
support-logging = diagnostico tecnico operacional
core-audit + audit.db = evidencia institucional/local auditavel
```

Nao usar logs tecnicos como auditoria.

# runtime-bootstrap

`runtime-bootstrap` compõe componentes de infra do Mini-Kernel RS.

```text
core-config define perfis
runtime-bootstrap resolve e instancia
cores executam
```

O runtime resolve o storage profile de auditoria, abre a base SQLite relacional,
monta `AuditSqliteStore<CryptoDetailsEncryptor> -> AuditService` e expõe o serviço
de auditoria com `details_json` cifrado com XChaCha20-Poly1305.

## KernelRuntime

```rust
use runtime_bootstrap::KernelRuntime;

let profile = core_config::MiniKernelProfile::dev_default(data_dir);
let runtime = KernelRuntime::open(&profile, key_provider)?;

runtime.audit().record_event(event_type, actor, target, details)?;
runtime.audit().verify_chain()?;
runtime.shutdown()?;
```

`KernelRuntime` não é genérico — a chave é extraída do provider na construção.

## AuditDbRuntime (base dedicada)

```rust
use runtime_bootstrap::{AuditDbConfig, AuditDbRuntime};

let config = AuditDbConfig::from_data_dir(data_dir);
let runtime = AuditDbRuntime::open(config, key_provider)?;

runtime.service().record_event(event_type, actor, target, details)?;
runtime.shutdown()?;
```

## Stack de auditoria

```text
AuditSqliteStore<CryptoDetailsEncryptor>
  ├── CryptoDetailsEncryptor (XChaCha20-Poly1305, AAD = event_id)
  ├── Schema relacional: audit_events + audit_chain_state
  ├── Triggers append-only (UPDATE/DELETE bloqueados ao nível da BD)
  ├── UNIQUE(sequence) + CHECK(sequence > 0)
  └── BEGIN IMMEDIATE em record() — serialização multi-processo
```

## Fronteira

```text
support-logging  = diagnóstico técnico operacional
core-audit + audit.db = evidência institucional auditável (AP Portuguesa/Europeia, RGPD)
```

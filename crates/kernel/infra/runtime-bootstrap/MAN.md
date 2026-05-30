# Manual do módulo runtime-bootstrap

## Objetivo

Compor componentes do Mini-Kernel RS sem contaminar os cores com detalhes de infraestrutura.

## KernelRuntime oficial

`KernelRuntime` é o caminho oficial para materializar um runtime local a partir
de `core_config::MiniKernelProfile`.

Regra arquitectural:

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
- abre `StorageBackend::Sqlite` (ficheiro) ou `StorageBackend::Memory` (`:memory:`);
- extrai a chave criptográfica do provider e constrói `CryptoDetailsEncryptor`;
- abre `AuditSqliteStore<CryptoDetailsEncryptor>` com as migrações aplicadas;
- cria logger técnico opcional quando `profile.logging.enabled = true`;
- não cria runtime async, DI container, UI ou dependência Tauri.

## Contrato público

```rust
// KernelRuntime — não genérico (chave extraída na construção)
KernelRuntime::open<P: KeyProvider + KeyResolver>(&MiniKernelProfile, P) -> Result<KernelRuntime, MiniError>
KernelRuntime::audit() -> &AuditDbService
KernelRuntime::logger() -> Option<&dyn TechnicalLogger>
KernelRuntime::shutdown() -> Result<(), MiniError>

// AuditDbRuntime — base dedicada de auditoria
AuditDbRuntime::open<P: KeyProvider>(AuditDbConfig, P) -> Result<AuditDbRuntime, MiniError>
AuditDbRuntime::service() -> &AuditDbService
AuditDbRuntime::shutdown() -> Result<(), MiniError>
```

Tipos exportados: `AuditDbConfig`, `AuditDbRuntime`, `AuditDbService`, `AuditDbStore`,
`CryptoDetailsEncryptor`, `AUDIT_DB_FILE_NAME`.

## Stack de auditoria

```text
AuditSqliteStore<CryptoDetailsEncryptor>
  ├── CryptoDetailsEncryptor
  │     ├── XChaCha20-Poly1305 (via support-crypto)
  │     ├── AAD = event_id (liga cifra ao evento, impede substituição)
  │     └── Chave extraída do KeyProvider na construção
  ├── Schema relacional (adapter-audit-sqlite):
  │     ├── audit_events (append-only)
  │     │     ├── sequence UNIQUE CHECK(> 0)
  │     │     ├── record_hash SHA-256 (cadeia verificável)
  │     │     └── details_json cifrado em repouso
  │     └── audit_chain_state (cabeça da cadeia)
  ├── Triggers BEFORE UPDATE/DELETE — adulteração bloqueada ao nível da BD
  └── BEGIN IMMEDIATE em record() — serialização multi-processo
```

## Garantias

- `audit.db` é separado da base funcional.
- O core (`core-audit`) não depende de SQLite nem de criptografia.
- `details_json` é cifrado com XChaCha20-Poly1305; campos de indexação (actor, target, datas) ficam em plaintext para eficiência SQL.
- `record()` usa `BEGIN IMMEDIATE` — serializa escritores entre processos distintos.
- `UNIQUE(sequence)` é segunda linha de defesa contra race conditions multi-processo.
- Triggers `BEFORE UPDATE/DELETE` bloqueam adulteração directa na BD.
- `shutdown()` é idempotente; escritas após shutdown falham com `StoreFailed`.
- `StorageBackend::Memory` usa SQLite `:memory:` — adequado para testes sem ficheiro.

## Erros públicos

| Código | Quando |
|---|---|
| `MINI.RUNTIME.INVALID_STORAGE_PROFILE` | Profile de auditoria em falta ou inválido |
| `MINI.RUNTIME.UNSUPPORTED_STORAGE_BACKEND` | Backend não suportado (ex: S3) |
| `MINI.RUNTIME.RUNTIME_OPEN_FAILED` | Erro genérico na abertura do runtime |
| `MINI.RUNTIME.AUDIT_RUNTIME_FAILED` | Falha ao abrir o store de auditoria |
| `MINI.RUNTIME.LOGGING_RUNTIME_FAILED` | Falha ao inicializar o logger técnico |

## Limites actuais

- Não gere ciclo de vida de recovery passphrase.
- Não importa automaticamente bases antigas criadas pelo adapter legado `support-audit-sqlite`.
- Export, assinatura e cadeia hash pertencem ao `core-audit`; este crate apenas compõe a infra.
- Sem plugin system, dynamic module loading, network sync, scheduler, metrics ou telemetry.

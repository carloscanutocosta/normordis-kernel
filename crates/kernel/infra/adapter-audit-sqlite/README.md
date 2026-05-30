# adapter-audit-sqlite

Adapter SQLite de produção para `core-audit`.

Implementa `AuditStore` directamente em SQLite, com atomicidade nas escritas,
serialização multi-processo, imutabilidade ao nível da BD e consultas temporais
eficientes por índice SQL.

## Quando usar

Use este adapter em produção sempre que:

- O volume de eventos ultrapassar ~100K registos
- Precisar de `verify_chain_since` incremental (dezenas de milhões de eventos)
- Precisar de `verify_chain_from_checkpoint` com hash externo de confiança
- Precisar de `list_by_date_range` eficiente (índice `idx_audit_time`)
- Precisar de atomicidade e serialização multi-processo (`BEGIN IMMEDIATE`)

Para testes unitários e volumes pequenos, `StorageAuditStore` (incluído em `core-audit`) é suficiente.

## Schema

### `audit_events` — append-only, nunca UPDATE/DELETE

```sql
CREATE TABLE audit_events (
    event_id          TEXT    NOT NULL PRIMARY KEY,
    event_type        TEXT    NOT NULL,
    actor_id          TEXT    NOT NULL,
    actor_name        TEXT,
    actor_type        TEXT,
    target_type       TEXT    NOT NULL,
    target_id         TEXT    NOT NULL,
    occurred_at       TEXT    NOT NULL,   -- RFC3339 UTC
    details_json      TEXT,               -- JSON opcional (cifrado em produção)
    sequence          INTEGER NOT NULL,   -- 1-indexed, nunca reutilizado
    prev_record_hash  TEXT,               -- hash do registo anterior na cadeia
    record_hash       TEXT    NOT NULL    -- SHA-256(schema_v + seq + prev + event)
);
```

### `audit_chain_state` — linha única (id = 1)

```sql
CREATE TABLE audit_chain_state (
    id               INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    sequence         INTEGER NOT NULL DEFAULT 0,
    head_event_id    TEXT,
    head_record_hash TEXT
);
```

### Índices e constraints (4 migrações)

| Migração | O que adiciona |
|---|---|
| 1 | Tabelas, `idx_audit_actor_time`, `idx_audit_target`, `idx_audit_sequence`, `idx_audit_time` (não-único) |
| 2 | `idx_audit_time ON audit_events (occurred_at ASC)` — suporta `list_by_date_range` |
| 3 | Triggers `BEFORE UPDATE/DELETE` — adulteração bloqueada ao nível da BD |
| 4 | `idx_audit_sequence_unique UNIQUE (sequence ASC)` + trigger `CHECK(sequence > 0)` |

O `UNIQUE(sequence)` é a segunda linha de defesa contra race conditions multi-processo;
o trigger `CHECK(sequence > 0)` impede valores inválidos.

## Utilização

```rust
use adapter_audit_sqlite::AuditSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_audit::{AuditActor, AuditService, AuditTarget};

let config = SqliteRelationalConfig::read_write_create("audit.db");
let store  = AuditSqliteStore::open(&config)?;
let svc    = AuditService::new(store);

// Gravar evento
svc.record_event(
    "documento.criado",
    AuditActor::new("user-123")?,
    AuditTarget::new("documento", "doc-456")?,
    None,
)?;

// Consulta por actor com paginação
let page = svc.list_by_actor("user-123", 50, 0)?;

// Consulta temporal eficiente
let from = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
let to   = chrono::Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
let eventos = svc.list_by_date_range(from, to, 100, 0)?;

// Verificação completa da cadeia
let report = svc.verify_chain()?;

// Verificação incremental — apenas eventos após o último checkpoint
let report_inc = svc.verify_chain_since(ultimo_seq_verificado + 1)?;

// Verificação com checkpoint externo de confiança (guardado fora da BD)
// Prova que o prefixo não foi adulterado mesmo que a BD esteja comprometida
let report_ext = svc.verify_chain_from_checkpoint(checkpoint_seq, &checkpoint_hash)?;

// Exportação e assinatura do manifesto
let key    = core_audit::AuditSigningKey::from_bytes(raw_key_bytes);
let signed = svc.sign_and_export(&key, Some("audit-key-prod".to_string()))?;
core_audit::verify_signed_manifest(&signed)?;
```

## Atomicidade e serialização multi-processo

`record()` usa `BEGIN IMMEDIATE` para adquirir o write-lock SQLite à entrada da
transacção. Isto serializa escritores entre processos distintos antes de ler
`audit_chain_state.sequence`, eliminando a race condition de sequência duplicada.

O `Mutex<Connection>` serializa chamadas dentro do mesmo processo; o `BEGIN IMMEDIATE`
cobre o caso multi-processo (ex: dois workers a gravar na mesma `audit.db`).

Em caso de conflito (`SQLITE_BUSY`), a operação retorna `AuditError::StoreFailed` —
o chamador pode fazer retry.

## Imutabilidade ao nível da BD

Triggers `BEFORE UPDATE/DELETE ON audit_events` bloqueiam qualquer adulteração directa,
mesmo por um operador com acesso à BD via CLI SQLite.

```
UPDATE audit_events SET event_type = 'x' WHERE ...
-- Erro: audit_events is append-only: UPDATE not allowed
```

## Integridade em leitura

Cada `get` e `list_*` recomputa o `record_hash` SHA-256 e compara com o valor
armazenado. Se divergirem, a operação falha com `AuditError::IntegrityFailed`.

## Verificação incremental da cadeia

**`verify_chain_since(N)`** — âncora lida da BD; detecta corrupção acidental; O(novos):

```
Dia 1:  verify_chain()           → verifica todos os N eventos
Dia 2:  verify_chain_since(N+1)  → verifica apenas os novos eventos
```

**`verify_chain_from_checkpoint(seq, hash)`** — âncora external de confiança; prova que
o prefixo não foi adulterado mesmo se a BD estiver comprometida:

```rust
// Guarda externamente após verificação completa:
// checkpoint_seq = report.checked_events, checkpoint_hash = report.head_record_hash

// Na próxima verificação, com hash vindo do ficheiro assinado / HSM:
let report = svc.verify_chain_from_checkpoint(checkpoint_seq, &checkpoint_hash)?;
// Se o evento na posição checkpoint_seq não bater com checkpoint_hash → falha imediata
// Rejeita checkpoint_seq == 0 com ChainVerificationFailed
```

## Encriptação de `details_json`

Com `PlaintextEncryptor` (por defeito): `details_json` guardado em texto simples.
Em produção, injete `CryptoDetailsEncryptor` (definido em `runtime-bootstrap`):

```rust
use adapter_audit_sqlite::AuditSqliteStore;
use runtime_bootstrap::CryptoDetailsEncryptor;

let encryptor = CryptoDetailsEncryptor::from_provider(&key_provider)?;
let store = AuditSqliteStore::open_with_encryptor(&config, encryptor)?;
```

O hash da cadeia é calculado sobre o evento em plaintext — a encriptação não afecta
a verificação.

## Dependências

```toml
[dependencies]
core-audit     = { path = "../../core/core-audit" }
adapter-sqlite = { path = "../adapter-sqlite" }
support-errors = { path = "../../support/support-errors" }
rusqlite       = { workspace = true }   # bundled
chrono         = { workspace = true }
serde_json     = { workspace = true }
thiserror      = { workspace = true }
```

## Testes

29 testes cobrindo:

- CRUD: `record`, `get`, `details_json`, duplicado rejeitado
- Append-only: triggers bloqueiam UPDATE e DELETE directos
- Encriptação: encriptador invocado em write, desencriptação correcta em read, AAD vinculado ao event_id
- Listagens: `list_by_actor`, `list_by_target`, `list_all` com paginação
- Consulta temporal: `list_by_date_range` com janela, limite exclusivo, paginação
- Cadeia: `verify_chain`, `verify_chain_since`, `verify_chain_from_checkpoint` (hash correcto, errado, fora de intervalo, sequência zero)
- Constraints: UNIQUE(sequence) bloqueia duplicados, trigger CHECK(sequence > 0)
- Manifesto: `export_manifest`, assinatura e verificação Ed25519

```
cargo test -p adapter-audit-sqlite
```

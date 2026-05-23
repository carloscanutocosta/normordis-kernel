# MAN — numerador-sqlite

## Objectivo

Persistência SQLite para o sistema NNS de numeração normalizada de sequências. Implementa `NumberingStore` e `NumberingSequenceRepository` com atribuição atómica e seed de contador.

---

## Contrato público

```rust
pub struct NumeradorDb {
    conn: rusqlite::Connection,
}

impl NumeradorDb {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, NumeradorError>;
}

/// Migrações exportadas.
pub const NUMERADOR_MIGRATIONS: &[&str];
```

`NumeradorDb` implementa:

```rust
impl NumberingStore for NumeradorDb { ... }
impl NumberingSequenceRepository for NumeradorDb { ... }
```

---

## Schema (3 migrações)

**Migração 1 — Series:**
```sql
CREATE TABLE IF NOT EXISTS nns_series (
    SeriesId     TEXT PRIMARY KEY,
    Scope        TEXT NOT NULL,
    Kind         TEXT NOT NULL,     -- 'sequential' | 'period_reset' | ...
    ResetPolicy  TEXT NOT NULL,     -- 'never' | 'yearly' | 'monthly'
    FormatParts  TEXT NOT NULL,     -- JSON: Vec<FormatPart>
    IsActive     INTEGER NOT NULL DEFAULT 1,
    CreatedAt    TEXT NOT NULL,
    CreatedBy    TEXT NOT NULL
);
```

**Migração 2 — Contadores e atribuições:**
```sql
CREATE TABLE IF NOT EXISTS nns_counter (
    SeriesId   TEXT NOT NULL REFERENCES nns_series(SeriesId),
    PeriodKey  TEXT NOT NULL,   -- ex: '2025', '2025-01'
    Counter    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (SeriesId, PeriodKey)
);

CREATE TABLE IF NOT EXISTS nns_assignment (
    AssignmentId   TEXT PRIMARY KEY,
    SeriesId       TEXT NOT NULL REFERENCES nns_series(SeriesId),
    PeriodKey      TEXT NOT NULL,
    SequenceNumber INTEGER NOT NULL,
    FormattedNumber TEXT NOT NULL,
    TargetKind     TEXT,
    TargetId       TEXT,
    AssignedAt     TEXT NOT NULL,
    AssignedBy     TEXT NOT NULL,
    UNIQUE(SeriesId, PeriodKey, SequenceNumber)
);
```

**Migração 3 — Metadados opcionais:**

`ensure_metadata_columns()` adiciona via `ALTER TABLE IF NOT EXISTS`:
- `Subject TEXT` — assunto/sumário
- `Recipient TEXT` — destinatário
- `ClassificationCode TEXT` — código MEF
- `Notes TEXT` — notas livres

---

## Atribuição atómica com retry

`assign` no `NumberingStore` usa optimistic locking com backoff:

```rust
// Pseudo-código
for attempt in 0..MAX_RETRIES {
    let counter = get_or_create_counter(series_id, period_key)?;
    let next = counter + 1;
    let result = try_insert_assignment(series_id, period_key, next, ...);
    match result {
        Ok(_) => {
            update_counter(series_id, period_key, next)?;
            return Ok(assigned_number);
        }
        Err(UNIQUE_CONSTRAINT) => {
            sleep(backoff(attempt));
            continue;
        }
        Err(e) => return Err(e),
    }
}
Err(NumeradorError::AssignmentConflict)
```

---

## Seed do contador

`seed_counter(series_id, period_key)` inicializa o contador ao valor máximo já atribuído para evitar colisões em bases de dados migradas:

```sql
UPDATE nns_counter
SET Counter = (
    SELECT COALESCE(MAX(SequenceNumber), 0)
    FROM nns_assignment
    WHERE SeriesId = ?1 AND PeriodKey = ?2
)
WHERE SeriesId = ?1 AND PeriodKey = ?2;
```

> **Importante**: chamar `seed_counter` antes do primeiro `assign` em BDs onde já existem atribuições importadas.

---

## Como usar

```rust
use numerador_sqlite::NumeradorDb;
use domain_numerador::{NumeradorService, NumberingSequence, ResetPolicy, FormatPart};

let db = NumeradorDb::open(&config)?;

// Criar série
db.save_series(&NumberingSequence {
    series_id: "oficio-dgf".into(),
    scope: "DGF".into(),
    kind: NumberingKind::Sequential,
    reset_policy: ResetPolicy::Yearly,
    format_parts: vec![
        FormatPart::Literal("OF".into()),
        FormatPart::Separator("/".into()),
        FormatPart::Counter { width: 4, pad: '0' },
        FormatPart::Separator("/".into()),
        FormatPart::Year,
    ],
    is_active: true,
    created_by: "system".into(),
    created_at: Utc::now(),
})?;

// Atribuir número
let svc = NumeradorService::new(db);
let assigned = svc.assign(&AssignNumberRequest {
    series_id: "oficio-dgf".into(),
    assigned_by: "user-1".into(),
    target: Some(TargetRef { kind: "oficio".into(), id: "doc-42".into() }),
    ..Default::default()
})?;
println!("{}", assigned.formatted_number); // "OF/0001/2025"
```

---

## Invariantes

- O par `(SeriesId, PeriodKey, SequenceNumber)` é UNIQUE — garante que não há números duplicados mesmo com concorrência.
- O `Counter` em `nns_counter` é sempre >= ao máximo `SequenceNumber` em `nns_assignment`.
- Séries inactivas (`IsActive = 0`) não aceitam novas atribuições.

---

## Limites actuais

- `conn` não é thread-safe por defeito — use `Arc<Mutex<NumeradorDb>>` no caller.
- O retry usa sleep síncrono — pode bloquear o thread em carga alta.
- Sem suporte a reversão de atribuições (números são definitivos).

---

## ToDo

- [ ] Retry assíncrono (tokio::time::sleep).
- [ ] Endpoint de auditoria: listar atribuições por série e período.
- [ ] Purge de contadores de períodos muito antigos.

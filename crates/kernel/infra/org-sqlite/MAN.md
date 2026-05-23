# MAN — org-sqlite

## Objectivo

Persistência SQLite para o domínio organizacional. Implementa os ports de unidades orgânicas, posições, competências, delegações e instrumentos legais, com suporte a hierarquia temporal via CTE recursiva.

---

## Contrato público

```rust
pub struct OrgSqliteStore {
    conn: rusqlite::Connection,
}

impl OrgSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, OrgError>;

    /// Devolve a hierarquia completa de unidades activas num instante.
    pub fn hierarchy_at(&self, at: DateTime<Utc>) -> Result<Vec<OrgUnit>, OrgError>;

    /// Desactiva uma unidade orgânica (guard: sem filhas activas, sem posições activas).
    pub fn deactivate_unit(&self, unit_id: &str, deactivated_by: &str) -> Result<(), OrgError>;
}

/// Migrações exportadas.
pub const ORG_SQLITE_MIGRATIONS: &[&str];
```

`OrgSqliteStore` implementa:

```rust
impl LegalInstrumentRepository for OrgSqliteStore { ... }
impl OrgUnitRepository for OrgSqliteStore { ... }
impl OrgPositionRepository for OrgSqliteStore { ... }
impl CompetencyRepository for OrgSqliteStore { ... }
impl DelegationRepository for OrgSqliteStore { ... }
```

---

## Schema (1 migração)

```sql
CREATE TABLE IF NOT EXISTS legal_instruments (
    Id          TEXT PRIMARY KEY,
    Reference   TEXT NOT NULL,
    Date        TEXT,
    Description TEXT,
    CreatedAt   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS org_units (
    Id            TEXT PRIMARY KEY,
    Code          TEXT NOT NULL UNIQUE,
    Name          TEXT NOT NULL,
    ParentId      TEXT REFERENCES org_units(Id),
    EffectiveFrom TEXT NOT NULL,
    EffectiveTo   TEXT,
    InstrumentId  TEXT REFERENCES legal_instruments(Id),
    CreatedBy     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS org_positions (
    Id         TEXT PRIMARY KEY,
    UnitId     TEXT NOT NULL REFERENCES org_units(Id),
    Code       TEXT NOT NULL,
    Title      TEXT NOT NULL,
    EffectiveFrom TEXT NOT NULL,
    EffectiveTo   TEXT,
    CreatedBy     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS competencies (
    Id          TEXT PRIMARY KEY,
    Code        TEXT NOT NULL,
    Description TEXT NOT NULL,
    Category    TEXT,
    CreatedAt   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS delegations (
    Id             TEXT PRIMARY KEY,
    FromPositionId TEXT NOT NULL REFERENCES org_positions(Id),
    ToPositionId   TEXT NOT NULL REFERENCES org_positions(Id),
    CompetencyId   TEXT NOT NULL REFERENCES competencies(Id),
    ValidFrom      TEXT NOT NULL,
    ValidTo        TEXT,
    InstrumentId   TEXT REFERENCES legal_instruments(Id),
    DelegatedBy    TEXT NOT NULL
);

-- Índices para queries de hierarquia
CREATE INDEX IF NOT EXISTS idx_org_units_parent ON org_units(ParentId);
CREATE INDEX IF NOT EXISTS idx_org_positions_unit ON org_positions(UnitId);
```

---

## Hierarquia temporal (CTE recursiva)

`hierarchy_at(timestamp)` usa uma CTE recursiva para reconstruir a árvore de unidades activas num dado instante:

```sql
WITH RECURSIVE tree(Id, Code, Name, ParentId, Depth) AS (
    SELECT Id, Code, Name, ParentId, 0
    FROM org_units
    WHERE ParentId IS NULL
      AND EffectiveFrom <= ?1
      AND (EffectiveTo IS NULL OR EffectiveTo > ?1)
    UNION ALL
    SELECT u.Id, u.Code, u.Name, u.ParentId, t.Depth + 1
    FROM org_units u
    INNER JOIN tree t ON u.ParentId = t.Id
    WHERE u.EffectiveFrom <= ?1
      AND (u.EffectiveTo IS NULL OR u.EffectiveTo > ?1)
)
SELECT * FROM tree ORDER BY Depth, Code;
```

---

## Guarda de desactivação

`deactivate_unit` verifica antes de desactivar:
1. Não há unidades filhas activas.
2. Não há posições activas nessa unidade.

Se alguma condição falhar, devolve `OrgError::UnitHasActiveChildren` ou `OrgError::UnitHasActivePositions`.

---

## Como usar

```rust
use org_sqlite::OrgSqliteStore;

let store = OrgSqliteStore::open(&config)?;

// Criar unidade raiz
store.save_unit(&OrgUnit {
    id: "u-dgf".into(),
    code: "DGF".into(),
    name: "Direcção-Geral de Finanças".into(),
    parent_id: None,
    effective_from: Utc::now(),
    effective_to: None,
    instrument_id: None,
    created_by: "system".into(),
})?;

// Consultar hierarquia actual
let tree = store.hierarchy_at(Utc::now())?;
```

---

## Invariantes

- `deactivate_unit` é atómica — verifica e desactiva na mesma transacção.
- Unidades desactivadas têm `effective_to` preenchido — não são eliminadas.
- `hierarchy_at` devolve apenas nós activos no instante pedido.
- As migrações são idempotentes.

---

## Limites actuais

- `conn` não está protegido por Mutex — não thread-safe por defeito.
- Sem suporte a reactivação de unidades (seria uma nova versão da unidade).
- Sem exportação da hierarquia em formato de árvore aninhada.

---

## ToDo

- [ ] Thread-safety via `Arc<Mutex<>>`.
- [ ] Exportação da hierarquia em JSON aninhado.
- [ ] Suporte a merge de unidades (quando duas unidades se fundem).

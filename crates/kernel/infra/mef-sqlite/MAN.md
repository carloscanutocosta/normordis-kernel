# MAN — mef-sqlite

## Objectivo

Persistência SQLite para a classificação MEF (Matriz de Estrutura Funcional). Implementa `MefRepository` com tabela temporal (PK composta por código + data de início de vigência), upsert idempotente e migração de tabelas legadas.

---

## Contrato público

```rust
pub struct MefSqliteStore {
    conn: rusqlite::Connection,
}

pub struct MefSqliteError(/* ... */);
impl From<MefError> for MefSqliteError { ... }

impl MefSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, MefSqliteError>;
}

/// Migrações exportadas.
pub const MEF_MIGRATIONS: &[&str];
```

`MefSqliteStore` implementa `MefRepository`:

```rust
impl MefRepository for MefSqliteStore {
    type Error = MefSqliteError;

    fn get_current(&self) -> Result<Vec<MefEntry>, MefSqliteError>;
    fn get_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<MefEntry>, MefSqliteError>;
    fn get_entry(&self, code: &MefCode) -> Result<Option<MefEntry>, MefSqliteError>;
    fn get_history(&self, code: &MefCode) -> Result<Vec<MefEntry>, MefSqliteError>;
    fn resolve_path(&self, code: &MefCode) -> Result<Vec<MefEntry>, MefSqliteError>;
    fn upsert_entry(&self, request: &UpsertMefEntryRequest) -> Result<(), MefSqliteError>;
    fn deactivate_entry(
        &self,
        code: &MefCode,
        changed_by: &str,
        change_reason: Option<&str>,
        diploma: Option<&DiplomaRef>,
    ) -> Result<(), MefSqliteError>;
}
```

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS platform_mef_classification (
    Code          TEXT NOT NULL,
    Label         TEXT NOT NULL,
    ParentCode    TEXT,
    IsUsable      INTEGER NOT NULL DEFAULT 1,
    EffectiveFrom TEXT NOT NULL,
    EffectiveTo   TEXT,           -- NULL = versão activa
    ChangedBy     TEXT NOT NULL,
    ChangeReason  TEXT,
    DiplomaRef    TEXT,           -- JSON: DiplomaRef
    DiplomaDate   TEXT,
    PRIMARY KEY (Code, EffectiveFrom)
);
CREATE INDEX IF NOT EXISTS idx_mef_code ON platform_mef_classification(Code);
CREATE INDEX IF NOT EXISTS idx_mef_active ON platform_mef_classification(Code) WHERE EffectiveTo IS NULL;
CREATE INDEX IF NOT EXISTS idx_mef_parent ON platform_mef_classification(ParentCode);
```

### View de compatibilidade

```sql
CREATE VIEW IF NOT EXISTS platform_mef_classification_current AS
SELECT Code, Label, ParentCode, IsUsable
FROM platform_mef_classification
WHERE EffectiveTo IS NULL;
```

---

## Upsert idempotente

`upsert_entry` verifica se o conteúdo mudou antes de criar nova versão:

```
1. Busca versão activa (EffectiveTo IS NULL) para o código.
2. Se não existe → insere nova linha com EffectiveFrom = now.
3. Se existe e (label, parent, is_usable) são iguais → não faz nada (idempotente).
4. Se existe e conteúdo diferente → fecha versão actual (EffectiveTo = now)
   e insere nova linha (EffectiveFrom = now).
```

---

## Migração de legado

`migrate_from_legacy_if_needed()` (chamado internamente):
1. Verifica se existe uma tabela legada com schema antigo (tabela flat sem temporalidade).
2. Se existir, copia as linhas para `platform_mef_classification` com `EffectiveFrom = '1970-01-01'`.
3. Chama `set_diploma_on_initial_entries()` para tentar preencher `DiplomaRef` nos registos migrados.
4. Cria a view de compatibilidade.

---

## Como usar

```rust
use mef_sqlite::MefSqliteStore;
use domain_mef::{MefRepository, MefCode, UpsertMefEntryRequest, DiplomaRef};

let store = MefSqliteStore::open(&config)?;

// Inserir/actualizar entrada MEF
store.upsert_entry(&UpsertMefEntryRequest {
    code: MefCode::new("0401")?,
    label: "Administração Tributária".into(),
    parent_code: Some(MefCode::new("04")?),
    is_usable: false,
    changed_by: "system".into(),
    change_reason: Some("Portaria de criação".into()),
    diploma: Some(DiplomaRef::new("Portaria n.º 1258/2009")?.with_date("2009-10-15")),
})?;

// Consultar entradas actuais
let current = store.get_current()?;

// Resolver caminho hierárquico
let path = store.resolve_path(&MefCode::new("0401.04.01")?)?;
// path[0] = raiz, path[last] = "0401.04.01"

// Reconstituir contexto num instante passado
let at_2023 = store.get_at(DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")?.into())?;

// Desactivar entrada abolida
store.deactivate_entry(
    &MefCode::new("0401.99")?,
    "admin",
    Some("Código abolido pela Portaria n.º 55/2024"),
    Some(&DiplomaRef::new("Portaria n.º 55/2024")?),
)?;
```

---

## Invariantes

- A PK `(Code, EffectiveFrom)` garante que não há duas versões com a mesma data de início para o mesmo código.
- `get_current()` e `get_entry()` filtram por `EffectiveTo IS NULL`.
- `upsert_entry` é idempotente quanto ao conteúdo — só cria nova versão se `label`, `parent_code` ou `is_usable` mudaram.
- `deactivate_entry` é idempotente — se já estiver desactivado, não faz nada.
- `resolve_path` devolve o caminho da raiz até ao código, usando a versão activa de cada nó.

---

## Limites actuais

- `conn` não é thread-safe — não protegido por Mutex.
- `resolve_path` faz múltiplas queries sequenciais (uma por nível) em vez de CTE recursiva.
- Sem suporte a pesquisa por label (full-text).

---

## ToDo

- [ ] `resolve_path` via CTE recursiva numa única query.
- [ ] Thread-safety via `Arc<Mutex<>>`.
- [ ] Pesquisa full-text sobre `Label`.
- [ ] Exportação da tabela MEF em CSV/JSON para partilha externa.

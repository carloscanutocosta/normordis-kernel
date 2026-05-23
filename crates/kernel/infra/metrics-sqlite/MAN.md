# MAN — metrics-sqlite

## Objectivo

Persistência SQLite para o domínio de métricas e avaliação de desempenho. Implementa `StorageMetricStore` de `core-metrics` e expõe as migrações de schema.

---

## Contrato público

```rust
pub struct MetricsSqliteStore {
    conn: Mutex<rusqlite::Connection>,
}

impl MetricsSqliteStore {
    /// Abre (ou cria) a base de dados e executa as migrações.
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, MetricError>;

    /// Constrói a partir de uma ligação já existente (útil em testes).
    pub fn from_connection(conn: rusqlite::Connection) -> Result<Self, MetricError>;

    /// Executa apenas as migrações (para uso em cenários de migração progressiva).
    pub fn migrate(conn: &rusqlite::Connection) -> Result<(), MetricError>;
}

/// Migrações SQL exportadas para uso por ferramentas externas de migração.
pub const METRICS_SQLITE_MIGRATIONS: &[&str];
```

`MetricsSqliteStore` implementa `StorageMetricStore`, que é o trait combinado:

```rust
impl MetricDefinitionStore for MetricsSqliteStore { ... }
impl MetricVersionStore for MetricsSqliteStore { ... }
impl EvaluationCycleStore for MetricsSqliteStore { ... }
impl IndicatorInstanceStore for MetricsSqliteStore { ... }
impl MeasurementResultStore for MetricsSqliteStore { ... }
impl TargetDefinitionStore for MetricsSqliteStore { ... }
impl StorageMetricStore for MetricsSqliteStore {}
```

---

## Schema (resumo)

As migrações criam as tabelas:

| Tabela | Conteúdo |
|---|---|
| `metric_definitions` | Definições de métricas com status |
| `metric_versions` | Versões com fórmula e requisitos de evidência |
| `evaluation_cycles` | Ciclos com tipo, status e datas |
| `indicator_instances` | Instâncias por ciclo e sujeito |
| `measurement_results` | Resultados com evidências e status |
| `target_definitions` | Targets por instância |

---

## Como usar

```rust
use metrics_sqlite::MetricsSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_metrics::{MetricServiceBuilder, MetricDefinition, MetricDefinitionStatus};

let config = SqliteRelationalConfig::read_write_create("metrics.db");
let store = MetricsSqliteStore::open(&config)?;
let service = MetricServiceBuilder::from_unified(store).build();

// Todas as operações passam pelo MetricService de core-metrics
```

### Com ligação existente (testes)

```rust
let conn = rusqlite::Connection::open_in_memory().unwrap();
let store = MetricsSqliteStore::from_connection(conn).unwrap();
```

---

## Invariantes

- As migrações são idempotentes (`CREATE TABLE IF NOT EXISTS`).
- `open()` e `from_connection()` executam sempre as migrações antes de devolver o store.
- A ligação é protegida por `Mutex` — thread-safe para uso em `MetricService`.

---

## Limites actuais

- Sem encriptação (base de dados em texto claro).
- Sem suporte a base de dados em memória partilhada entre stores distintos.
- A granularidade de lock é ao nível da ligação inteira (um só `Mutex`).

---

## ToDo

- [ ] Suporte a SQLCipher para bases de dados de métricas sensíveis.
- [ ] Índices adicionais para queries de agregação por ciclo.

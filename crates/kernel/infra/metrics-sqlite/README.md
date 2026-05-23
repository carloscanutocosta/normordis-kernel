# metrics-sqlite

Adaptador SQLite para `core-metrics` — persistência concreta de métricas, versões, ciclos, instâncias e resultados.

## Objectivo

Implementa todos os stores de `core-metrics` (`StorageMetricStore`) sobre uma base de dados SQLite local, aplicando as migrações de schema necessárias na abertura.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite` (API relacional partilhada), `core-metrics` (ports e tipos) e `rusqlite`.

## Responsabilidade

- Abrir a ligação SQLite e executar as migrações (`METRICS_SQLITE_MIGRATIONS`).
- Implementar `StorageMetricStore` (que agrega todos os stores individuais de `core-metrics`).
- Fornecer `MetricsSqliteStore` como tipo concreto para injecção em `MetricServiceBuilder`.

## Não-responsabilidade

- Não implementa lógica de negócio — toda a lógica reside em `core-metrics`.
- Não encripta a base de dados (use `adapter-sqlite` com SQLCipher se necessário).
- Não gere migrações de schema entre versões da aplicação (use `versioning-sqlite`).

## Exemplo mínimo

```rust
use metrics_sqlite::MetricsSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_metrics::MetricServiceBuilder;

let config = SqliteRelationalConfig::read_write_create("metrics.db");
let store = MetricsSqliteStore::open(&config)?;
let service = MetricServiceBuilder::from_unified(store).build();
```

## Validação

```sh
cargo test -p metrics-sqlite
```

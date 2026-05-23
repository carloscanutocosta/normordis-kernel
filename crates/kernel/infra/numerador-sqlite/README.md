# numerador-sqlite

Adaptador SQLite para o domínio NNS (Numeração Normalizada de Sequências) — contadores atómicos com retry e seed a partir de dados existentes.

## Objectivo

Implementa `NumberingStore` e `NumberingSequenceRepository` de `domain-numerador` sobre SQLite, com atribuição atómica de números e backoff automático em colisões UNIQUE.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite`, `domain-numerador` e `rusqlite`.

## Responsabilidade

- Persistir séries de numeração (`nns_series`), contadores por período (`nns_counter`) e atribuições (`nns_assignment`).
- Atribuir números de forma atómica com retry em caso de colisão concorrente.
- Fazer seed do contador a partir de atribuições existentes (`seed_counter`) para evitar UNIQUE constraint em bases de dados migradas.
- Garantir que colunas de metadados opcionais existem (via `ensure_metadata_columns` com `ALTER TABLE IF NOT EXISTS`).

## Não-responsabilidade

- Não define o formato do número — o formato é calculado em `domain-numerador`.
- Não controla permissões de quem pode atribuir números.
- Não limpa atribuições antigas.

## Exemplo mínimo

```rust
use numerador_sqlite::NumeradorDb;
use adapter_sqlite::SqliteRelationalConfig;

let config = SqliteRelationalConfig::read_write_create("numerador.db");
let db = NumeradorDb::open(&config)?;
```

## Validação

```sh
cargo test -p numerador-sqlite
```

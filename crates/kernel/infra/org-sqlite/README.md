# org-sqlite

Adaptador SQLite para o domínio organizacional — unidades orgânicas, posições, competências e delegações hierárquicas.

## Objectivo

Implementa os ports de organização (`OrgUnitRepository`, `OrgPositionRepository`, `CompetencyRepository`, `DelegationRepository`, `LegalInstrumentRepository`) sobre SQLite, com suporte a hierarquia recursiva e vistas temporais.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite` e dos ports definidos no domínio organizacional.

## Responsabilidade

- Persistir a estrutura hierárquica de unidades orgânicas.
- Gerir posições e competências associadas.
- Registar delegações de competências entre posições.
- Navegar a hierarquia em qualquer instante (via CTE recursiva com `effective_from`/`effective_to`).
- Guardar referências a instrumentos legais que fundamentam alterações estruturais.

## Não-responsabilidade

- Não decide autorização — use `security-sqlite` e `rh-security-bridge`.
- Não numera documentos nem ofícios.
- Não conhece o sistema de avaliação SIADAP.

## Exemplo mínimo

```rust
use org_sqlite::OrgSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;

let config = SqliteRelationalConfig::read_write_create("org.db");
let store = OrgSqliteStore::open(&config)?;
let units = store.hierarchy_at(Utc::now())?;
```

## Validação

```sh
cargo test -p org-sqlite
```

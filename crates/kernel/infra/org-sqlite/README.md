# org-sqlite

Adaptador SQLite para o domínio organizacional (`core-org`).

## Responsabilidade

Implementa todos os ports de `core-org` sobre SQLite:

| Port | Implementação |
|---|---|
| `OrgUnitRepository` | CRUD, OCC, CTE recursiva ascendente e descendente, pesquisa paginada |
| `OrgPositionRepository` | CRUD, OCC, `find_by_code`, `list_by_kind`, `list_for_unit_and_kind`, `list_all_at`, `find_effective_substitute` |
| `LegalInstrumentRepository` | Upsert, list, list_effective_at |
| `CompetencyRepository` | Upsert, list_for_position_at |
| `DelegationRepository` | Upsert, get_effective_at |
| `OrgAuditPort` (via `OrgAuditAdapter`) | Liga `core-org` ao `core-audit::AuditStore` |

## Posição arquitectural

```
core-org (domínio) ──define ports──→ OrgUnitRepository, …, OrgAuditPort
                                              ↑
org-sqlite (infra) ──implementa──────────────┘
        │
        └── OrgAuditAdapter ──usa──→ core-audit::AuditStore
```

Não depende de Tauri, UI, filesystem ou de outros domínios core (depende apenas de
`core-org`, `core-audit` e `adapter-sqlite`).

## Modelo de acesso à BD

`OrgSqliteStore` envolve uma `Connection` em `Arc<Mutex<>>` — é `Send + Sync + Clone`
e thread-safe, seguindo o padrão de `security-sqlite`/`metrics-sqlite`. Abertura e
migração usam os helpers de `adapter-sqlite` (`open_relational_connection`,
`run_relational_migrations`). Operações multi-passo (ex.: `deactivate`, que verifica
filhos e posições antes de escrever) correm numa transacção `IMMEDIATE`, eliminando
a janela TOCTOU.

## Funcionalidades chave

- **OCC** — `update()` usa `WHERE version = ?` e incrementa `version + 1`; devolve
  `VersionConflict` ou `UnitNotFound` conforme o caso.
- **Hierarquia sem limite de profundidade** — `hierarchy_at` (ascendente) e
  `list_subtree` (descendente) usam CTEs recursivas SQLite.
- **Pesquisa paginada** — `search_by_name(term, OrgPage)` devolve `PagedResult<OrgUnit>`
  com `total`, `items` e `has_more()`.
- **Substituto legal** — `find_effective_substitute(position_id, date)` devolve a
  posição que substitui a dada, activa na data.
- **Status de posições** — `OrgPositionStatus` (Active / Suspended / Extinct) com
  máquina de estados homóloga à de unidades.
- **Auditoria ligada** — `OrgAuditAdapter` traduz cada `OrgAuditEvent` para um
  `core_audit::AuditEvent` (cadeia de hashes, COSO `outcome`/`control_id`).

## Auditoria — `OrgAuditAdapter`

Liga a camada de serviço de `core-org` ao store de auditoria sem acoplar os domínios:

```rust
use std::sync::Arc;
use org_sqlite::{OrgSqliteStore, OrgAuditAdapter};
use adapter_audit_sqlite::AuditSqliteStore;
use core_org::{OrgUnitService, OrgNoopDomainEvents};

let audit_store = Arc::new(AuditSqliteStore::open(&audit_cfg)?);
let org_store   = OrgSqliteStore::open(&org_cfg)?;

let svc = OrgUnitService::new(
    org_store,
    OrgAuditAdapter::new(audit_store), // OrgAuditPort → core-audit
    OrgNoopDomainEvents,
);

// Cada operação grava um AuditEvent ("org.orgunit.created", outcome=Success,
// payload com o snapshot da unidade) na cadeia de hashes verificável.
svc.create(unit, "joao.silva")?;
```

## Utilização típica (repositório directo)

```rust
use org_sqlite::OrgSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;
use core_org::OrgPage;

let store = OrgSqliteStore::open(&SqliteRelationalConfig::read_write_create("org.db"))?;

// Pesquisa paginada
let result = store.search_by_name("Finanças", OrgPage::first(20))?;
println!("{} unidades encontradas", result.total);

// Substituto legal activo numa data
let sub = store.find_effective_substitute(&chefe_id, date(2025, 6, 1))?;

// Todas as posições activas de um tipo numa unidade
let chefias = store.list_positions_for_unit_and_kind(&unit_id, &PositionKind::Chefia, hoje)?;
```

## Testes

```sh
cargo test -p org-sqlite
```

21 testes de integração — CRUD, OCC (`VersionConflict`), hierarquia recursiva,
pesquisa paginada, substituto legal, queries compostas, e dois testes end-to-end
do `OrgAuditAdapter` (criação e transição de estado geram `AuditEvent` correctos).

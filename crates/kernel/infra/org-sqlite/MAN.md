# MAN — org-sqlite

## Objectivo

Persistência SQLite para o domínio organizacional `core-org`. Implementa todos os
cinco ports de repositório com suporte a hierarquia recursiva, vistas temporais,
Optimistic Concurrency Control (OCC) e pesquisa paginada. Inclui o `OrgAuditAdapter`,
que liga a camada de serviço de `core-org` ao `core-audit`.

---

## Contrato público

```rust
/// Thread-safe e clonável: Arc<Mutex<Connection>>.
#[derive(Clone)]
pub struct OrgSqliteStore { /* conn: Arc<Mutex<Connection>> */ }

impl OrgSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, OrgSqliteError>;
    pub fn from_connection(conn: Connection) -> Result<Self, OrgSqliteError>;
    pub fn migrate(&self) -> Result<(), OrgSqliteError>;
}

/// Adaptador de auditoria: OrgAuditPort → core-audit (AuditStore [+ ControlRegistry]).
pub struct OrgAuditAdapter { /* store, controls: Option<...> */ }

impl OrgAuditAdapter {
    pub fn new(store: Arc<dyn AuditStore>) -> Self;                       // só AuditEvent
    pub fn with_controls(                                                 // + ControlExecution
        store: Arc<dyn AuditStore>,
        controls: Arc<dyn ControlRegistryStore>,
    ) -> Self;
}

/// Catálogo de ControlDefinition dos controlos CTRL-ORG-* (registar no arranque).
pub fn org_control_catalog() -> Vec<ControlDefinition>;

pub const ORG_SQLITE_MIGRATIONS: &[&str];
```

`OrgSqliteStore` implementa os cinco ports de repositório de `core-org` (cada um
com variantes `*_audited`) e o `OrgAuditOutbox`; `OrgAuditAdapter` implementa o
porto de entrega de auditoria:

```rust
impl LegalInstrumentRepository for OrgSqliteStore { ... }
impl OrgUnitRepository         for OrgSqliteStore { ... }
impl OrgPositionRepository     for OrgSqliteStore { ... }
impl CompetencyRepository      for OrgSqliteStore { ... }
impl DelegationRepository      for OrgSqliteStore { ... }
impl OrgAuditOutbox            for OrgSqliteStore { ... }
impl OrgAuditPort              for OrgAuditAdapter { ... }
```

`OrgSqliteStore` é `Send + Sync + Clone`: a `Connection` está protegida por
`Arc<Mutex<>>`, pelo que o mesmo store pode ser partilhado entre threads/tasks.

Façade de conveniência exposta directamente no store (evita disambiguation UFCS
nos callers mais simples):

```rust
// Unidades
store.get_unit(&id)?
store.save_unit(&unit)?
store.update_unit(&unit)?    // OCC
store.deactivate_unit(&id, date)?
store.list_active_at(date)?
store.list_subtree(&root_id, date)?   // descendentes recursivos
store.full_tree_at(date)?             // toda a árvore não-extinta
store.search_by_name(term, page)?     // paginado, LIKE em short/full name

// Posições
store.get_position(&id)?
store.save_position(&pos)?
store.update_position(&pos)?         // OCC
store.deactivate_position(&id, date)?
store.find_position_by_code(code)?
store.find_effective_substitute(&pos_id, date)?
store.list_all_positions_at(date)?                       // todas as activas, cross-unit
store.list_positions_for_unit_and_kind(&unit, &kind, date)?  // por unidade + tipo

// Outros
store.save_instrument(&instr)?
store.save_competency(&comp)?
store.save_delegation(&del)?
store.get_effective_at(&to_pos, date)?
```

---

## Schema

Uma única migração (hash-tracked por `run_relational_migrations`):

```sql
-- Instrumentos jurídicos — referência imutável
CREATE TABLE IF NOT EXISTS legal_instruments (
    instrument_id   TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,       -- portaria | despacho | deliberacao |
                                         -- regulamento_organico | outro:<desc>
    reference       TEXT NOT NULL,
    date            TEXT NOT NULL,       -- YYYY-MM-DD
    description     TEXT NOT NULL,
    effective_from  TEXT NOT NULL,
    effective_until TEXT
);

-- Unidades orgânicas — hierarquia self-referencial, profundidade ilimitada
CREATE TABLE IF NOT EXISTS org_units (
    unit_id         TEXT PRIMARY KEY,
    short_name      TEXT NOT NULL,
    full_name       TEXT NOT NULL,
    service_code    TEXT,
    level           INTEGER NOT NULL CHECK (level >= 1),
    parent_id       TEXT REFERENCES org_units(unit_id),
    created_by      TEXT REFERENCES legal_instruments(instrument_id),
    legal_reference TEXT,
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    status          TEXT NOT NULL DEFAULT 'active',  -- active | suspended | extinct
    email           TEXT,
    phone           TEXT,
    fax             TEXT,
    rua             TEXT,
    numero          TEXT,
    porta           TEXT,
    local           TEXT,
    cp4             TEXT,
    cp3             TEXT,
    localidade      TEXT,
    version         INTEGER NOT NULL DEFAULT 0       -- OCC
);

-- Posições orgânicas — abstractas, independentes do titular
CREATE TABLE IF NOT EXISTS org_positions (
    position_id     TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    title           TEXT NOT NULL,
    kind            TEXT NOT NULL DEFAULT 'outro',   -- direcao | coordenacao |
                                                     -- chefia | adjunto | tecnico |
                                                     -- outro | outro:<desc>
    substitutes     TEXT REFERENCES org_positions(position_id),
    status          TEXT NOT NULL DEFAULT 'active',
    unit_id         TEXT NOT NULL REFERENCES org_units(unit_id),
    created_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    version         INTEGER NOT NULL DEFAULT 0
);

-- Competências — autoridade jurídica para actos administrativos (com OCC)
CREATE TABLE IF NOT EXISTS competencies (
    competency_id   TEXT PRIMARY KEY,
    code            TEXT NOT NULL,
    description     TEXT NOT NULL,
    scope           TEXT NOT NULL,
    assigned_to     TEXT NOT NULL REFERENCES org_positions(position_id),
    granted_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    version         INTEGER NOT NULL DEFAULT 0
);

-- Delegações de competência entre posições (com OCC)
CREATE TABLE IF NOT EXISTS delegations (
    delegation_id   TEXT PRIMARY KEY,
    competency_id   TEXT NOT NULL REFERENCES competencies(competency_id),
    from_position   TEXT NOT NULL REFERENCES org_positions(position_id),
    to_position     TEXT NOT NULL REFERENCES org_positions(position_id),
    instrument_id   TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
    valid_from      TEXT NOT NULL,
    valid_until     TEXT,
    version         INTEGER NOT NULL DEFAULT 0
);

-- Outboxes — capturados atomicamente com o estado (M1). delivered: 0=pendente,
-- 1=entregue, 2=dead-letter (esgotou attempts).
CREATE TABLE IF NOT EXISTS org_audit_outbox (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    event_json  TEXT NOT NULL,   -- OrgAuditEvent serializado (event_id estável)
    created_at  TEXT NOT NULL,
    delivered   INTEGER NOT NULL DEFAULT 0,
    attempts    INTEGER NOT NULL DEFAULT 0,
    last_error  TEXT
);
CREATE TABLE IF NOT EXISTS org_domain_outbox ( /* idem, OrgDomainEvent */ );
```

> **Migrações append-only.** M0 é a baseline de entidades; M1 acrescenta `version`
> a competencies/delegations (`ALTER`) e as tabelas de outbox (`CREATE`). Nunca
> editar uma migração existente — adicionar sempre uma nova no fim.

Índices: `level`, `parent_id`, `(valid_from, valid_until)`, `(short_name, full_name)`,
`(kind, status)`, `substitutes`, `(assigned_to, valid_from, valid_until)`,
`(to_position, valid_from, valid_until)`.

---

## Optimistic Concurrency Control (OCC)

`update()` para unidades e posições usa:

```sql
UPDATE org_units
SET …, version = version + 1
WHERE unit_id = ?1 AND version = ?22
```

Se `affected == 0`:

1. Verifica se a entidade existe (`SELECT COUNT(*) WHERE id = ?`).
2. Se existe → `OrgError::VersionConflict(id)`.
3. Se não existe → `OrgError::UnitNotFound(id)`.

O caller deve re-buscar a entidade após `VersionConflict` e tentar novamente.

---

## Hierarquia recursiva

### `hierarchy_at(id, date)` — ascendente (unidade → raiz)

```sql
WITH RECURSIVE chain AS (
    SELECT … FROM org_units WHERE unit_id = ?1 AND <temporal>
    UNION ALL
    SELECT u.… FROM org_units u
    INNER JOIN chain c ON u.unit_id = c.parent_id
    WHERE <temporal>
)
SELECT * FROM chain ORDER BY level DESC
```

Resultado ordenado do nó mais profundo para a raiz.

### `list_subtree(root_id, date)` — descendente (raiz → folhas)

```sql
WITH RECURSIVE subtree AS (
    SELECT … FROM org_units WHERE unit_id = ?1 AND <temporal>
    UNION ALL
    SELECT u.… FROM org_units u
    INNER JOIN subtree s ON u.parent_id = s.unit_id
    WHERE <temporal>
)
SELECT * FROM subtree ORDER BY level, short_name
```

Inclui o próprio nó raiz. Ordenado top-down.

---

## Pesquisa paginada

```rust
fn search_by_name(&self, term: &str, page: OrgPage)
    -> Result<PagedResult<OrgUnit>, OrgError>
```

```sql
-- Contagem total
SELECT COUNT(*) FROM org_units
WHERE status != 'extinct' AND (short_name LIKE ?1 OR full_name LIKE ?1)

-- Página de dados
SELECT … FROM org_units
WHERE status != 'extinct' AND (short_name LIKE ?1 OR full_name LIKE ?1)
ORDER BY level, short_name
LIMIT ?2 OFFSET ?3
```

`OrgPage::first(20)` — primeira página de 20 resultados.
`PagedResult::has_more()` — indica se há mais páginas.

---

## Substituto legal efectivo

```rust
fn find_effective_substitute(
    &self,
    position_id: &OrgPositionId,
    date: NaiveDate,
) -> Result<Option<OrgPosition>, OrgError>
```

```sql
SELECT … FROM org_positions
WHERE substitutes = ?1
  AND status = 'active'
  AND valid_from <= ?2
  AND (valid_until IS NULL OR valid_until > ?2)
ORDER BY code LIMIT 1
```

Devolve a posição activa cujo campo `substitutes` aponta para `position_id`.

---

## Guarda de desactivação de unidades

`deactivate_unit` verifica antes de escrever:

1. `COUNT(*) WHERE parent_id = ? AND status = 'active'` → `CannotDeactivateWithActiveChildren`
2. `COUNT(*) WHERE unit_id = ? AND (valid_until IS NULL OR valid_until > ?)` → `CannotDeactivateWithActivePositions`

Se ambas passarem: `UPDATE … SET status = 'extinct', valid_until = ?, version = version + 1`.

---

## Evidência COSO — outbox + OrgAuditAdapter

### Captura atómica (outbox)

Os métodos `*_audited` das repos (`create_audited`, `update_audited`,
`deactivate_audited`, `save_audited`) escrevem o estado, o `OrgAuditEvent`
(`org_audit_outbox`) **e** o `OrgDomainEvent` (`org_domain_outbox`) numa única
transacção `IMMEDIATE` — nenhuma mudança de estado fica sem evidência nem sem o
respectivo evento de integração. Falhas (rejeição, conflito) são enfileiradas via
`enqueue_audit` (sem estado associado).

### Entrega idempotente e resiliente (drain)

`drain_audit_outbox(&dyn OrgAuditPort)` e `drain_domain_outbox(&dyn OrgDomainEventPort)`
lêem os pendentes (por `seq`) e entregam. O `event_id` estável torna a reentrega
idempotente (`DuplicateEvent` → tratado como entregue). Uma mensagem que falha vê
`attempts` incrementado e, ao atingir `MAX_OUTBOX_ATTEMPTS`, é movida para
dead-letter (`delivered=2`) — **sem bloquear** a fila (não há head-of-line
blocking). `pending_*_count` e `dead_letter_*_count` expõem o estado. O
`OrgOutboxDrainer` (core-org) faz a entrega periódica supervisionada em background.

### OrgAuditAdapter (OrgAuditPort → core-audit)

| Campo `OrgAuditEvent` | Destino em `core-audit` |
|---|---|
| `event_id` | `AuditEvent.event_id` (identidade estável → idempotência) |
| `entity_kind` + `action` | `event_type` = `"org.<entidade_lower>.<acção>"` |
| `actor` | `AuditActor { actor_id, actor_type: "user" }` |
| `entity_kind` / `entity_id` | `AuditTarget { target_type, target_id }` |
| `outcome` | `AuditOutcome::Success` \| `Failure` |
| `control_id` | `AuditEvent.control_id` + `ControlExecution.control_id` |
| `payload` | `details_json` (snapshot ou contexto da falha) |

`OrgAuditAdapter::with_controls(audit, registry)` grava também uma
`ControlExecution` (`Passed`/`Failed`) com `execution_id` determinístico
`<event_id>:<control_id>` (idempotente). `org_control_catalog()` expõe as
`ControlDefinition` dos controlos `CTRL-ORG-*` para registo no arranque.

A dependência aponta org → audit **na infra**, nunca no domínio.

---

## Utilização com a camada de serviço

```rust
use std::sync::Arc;
use org_sqlite::{OrgSqliteStore, OrgAuditAdapter};
use adapter_audit_sqlite::AuditSqliteStore;
use core_org::{OrgUnitService, OrgPositionService, OrgNoopDomainEvents};

let org_store   = OrgSqliteStore::open(&org_cfg)?;
let audit_store = Arc::new(AuditSqliteStore::open(&audit_cfg)?);

// Serviço de unidades com auditoria real (3 genéricos: repo, audit, eventos)
let unit_svc = OrgUnitService::new(
    org_store.clone(),
    OrgAuditAdapter::new(audit_store.clone()),
    OrgNoopDomainEvents, // substituir por um publisher real para integrar core-rh
);
unit_svc.create(unit, "joao.silva")?;   // validate_strict + hierarquia + auditoria + evento
unit_svc.suspend(&id, "joao.silva")?;   // transition_status + OCC + auditoria + evento
unit_svc.deactivate(&id, date, "x")?;  // transition_status + deactivate + auditoria + evento

// Serviço de posições
let pos_svc = OrgPositionService::new(
    org_store,
    OrgAuditAdapter::new(audit_store),
    OrgNoopDomainEvents,
);
pos_svc.create(position, "admin")?;    // validate + ciclo de substituição + auditoria + evento
```

Em testes ou contextos sem auditoria/integração, usar `OrgNoopAudit` e
`OrgNoopDomainEvents` (de `core-org`).

---

## Invariantes

- `update` é idempotente se a versão coincidir (re-run com os mesmos dados não cria duplicados).
- `save` (upsert) não toca em `version` — adequado para importações.
- Unidades extintas nunca são eliminadas — `valid_until` + `status = 'extinct'`.
- CTEs recursivas não têm limite de profundidade — hierarquias de N níveis são suportadas.
- `deactivate` corre em transacção `IMMEDIATE` — verificação de filhos/posições e
  escrita são atómicas (sem janela TOCTOU).
- Migrações são hash-tracked por `run_relational_migrations` — idempotentes.

---

## Limites conhecidos

- O `Mutex<Connection>` serializa as operações dentro do processo. Para alta
  concorrência de escrita, considerar um pool de ligações ou particionamento.
- `search_by_name` usa LIKE sem escaping de `%` e `_` — adequado para nomes de organismos portugueses.
- Os eventos de domínio (`OrgDomainEventPort`) usam `OrgNoopDomainEvents` por
  defeito — o publisher que integra `core-rh` é fornecido pela camada de aplicação.

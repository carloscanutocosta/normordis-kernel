# ADR-CORE-MET-001 — Core de Métricas Operacionais e Governadas

**Estado:** Aceite  
**Data:** 2026-05-17  
**Crate:** `core-metrics` + `metrics-sqlite`

---

## Contexto

As apps do ecossistema mini-apps (laboratório de normordis) precisam de emitir métricas institucionais — tempos de processo, contagens de documentos, taxas de execução — para alimentar painéis BSC e avaliações SIADAP (Lei n.º 66-B/2007). Até agora estas métricas eram exportadas ad-hoc para Excel por cada equipa, sem modelo de dados comum, sem rastreabilidade e sem possibilidade de auditoria ex-post.

`core-metrics` resolve isso de forma análoga a como `core-audit` resolve a rastreabilidade de eventos: cada app usa um core bem definido, persistindo em SQLite em mini-apps (e futuramente em PostgreSQL em produção), com um modelo de governação que liga emissões operacionais a definições aprovadas pelo órgão de gestão.

---

## Decisão

### 1. Separação entre emissão operacional e governação

**`MetricEvent`** — emitido pelas apps em runtime. Contém `metric_code` (referência à definição canónica), `metric_version_id` e `evaluation_cycle_id` opcionais, além de `value`, `unit`, `entity_id`, `org_unit_id`, `labels`, `payload` e `timestamp`.

**Camada de governação** — tipos criados e geridos pelo órgão de gestão, nunca pelas apps em runtime:
- `MetricDefinition` — define o que é medido; `code` é estável e único
- `MetricVersion` — fórmula concreta, vigência temporal, `evidence_requirements`
- `TargetDefinition` — objectivos/limiares por âmbito e período; não altera a fórmula
- `EvaluationCycle` — período formal (SIADAP anual, BSC trimestral, etc.)
- `IndicatorInstance` — ligação MetricVersion × EvaluationCycle × UO × responsável
- `MeasurementResult` — resultado materializado; imutável após validação
- `EvidenceLink` — ligação auditável a fontes (audit events, documentos, snapshots)

### 2. Traits de store por responsabilidade

Cada grupo de tipos tem o seu próprio trait no módulo `governance`:

| Trait | Uso típico |
|---|---|
| `MetricStore` | Apps emit events; indexação por code, ciclo, entidade |
| `MetricDefinitionStore` | Backoffice; UNIQUE constraint em `code` |
| `MetricVersionStore` | Backoffice; `get_active_version_for_code(code, at)` com JOIN |
| `TargetDefinitionStore` | Backoffice; queries por âmbito |
| `EvaluationCycleStore` | Backoffice; UNIQUE constraint em `code` |
| `IndicatorInstanceStore` | Cálculo; listagem por ciclo e UO |
| `MeasurementResultStore` | Cálculo + validação; EvidenceLink para auditabilidade |

Todos os traits exigem `Send + Sync` para compatibilidade com servidores web.

### 3. Adapter SQLite (`metrics-sqlite`)

`MetricsSqliteStore` implementa todos os traits numa única conexão SQLite envolta em `Mutex<Connection>` (necessário para `Sync`). Migrations em `METRICS_SQLITE_MIGRATIONS`.

Schema: 8 tabelas (`metric_events`, `metric_definitions`, `metric_versions`, `target_definitions`, `evaluation_cycles`, `indicator_instances`, `measurement_results`, `evidence_links`) com foreign keys e índices por uso.

Mapeamento de erros:
- `UNIQUE constraint violation` → `MetricError::Conflict`
- Outros erros SQLite → `MetricError::RepoUnavailable`

DateTime armazenado como `%Y-%m-%dT%H:%M:%S%.3fZ` (ISO 8601).

### 4. Relação com outros cores

| Core | Relação |
|---|---|
| `core-audit` | `EvidenceLink.evidence_type = AuditEvent` referencia eventos de auditoria |
| `core-documental` | `EvidenceLink.evidence_type = Document` referencia documentos finais |
| `core-ingest` | Apps de ingestão emitem `MetricEvent` via `MetricEmitter` |

---

## Padrões de uso

### App emite uma métrica operacional

```rust
let emitter: Arc<dyn MetricEmitter> = ...;
let event = new_event("evt-001", "proc.duration", 42.5, Some("days"), None);
emitter.emit(event)?;
```

### Obter a versão activa de uma métrica num instante

```rust
let store: &dyn MetricVersionStore = ...;
let version = store.get_active_version_for_code("proc.duration", Utc::now())?;
```

### Registar um resultado oficial

```rust
let store: &dyn MeasurementResultStore = ...;
// Calcular...
store.save_result(&result)?;
// Validar após revisão:
store.update_result_status(&result.id, &MeasurementStatus::Validated, "director")?;
// Obter leitura oficial:
let official = store.get_official_result(&instance_id)?;
```

---

## Invariantes

1. `MetricEvent.metric_code` deve corresponder a um `MetricDefinition.code` válido — validado na camada de serviço, não no store.
2. `MeasurementResult` não pode ser editado após `Validated` — correcções criam novo resultado com `rectifies_result_id` e marcam o anterior como `Rectified`.
3. `MetricDefinition.code` e `EvaluationCycle.code` são imutáveis após criação.
4. `TargetDefinition` não altera a fórmula da versão — apenas define limiares de leitura.

---

## Alternativas consideradas

**Usar `support-storage` (key-value) como backend** — implementado numa primeira fase via `StorageMetricStore`. Descartado para os tipos de governação por exigir queries relacionais (JOIN metric_definitions × metric_versions, filtros por múltiplos campos). Mantido para `MetricEvent` em contextos sem SQLite.

**PostgreSQL directamente** — rejeitado para mini-apps (laboratório local). `metrics-sqlite` actua como provador de conceito; a migração para Postgres em normordis será feita trocando o adapter sem alterar `core-metrics`.

**Merge com `core-audit`** — rejeitado. Métricas são agregáveis e comparáveis entre períodos; eventos de auditoria são factos imutáveis pontuais. Semânticas e padrões de query distintos justificam cores separados.

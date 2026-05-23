# MAN — core-metrics

## Objectivo

Domínio de métricas e avaliação de desempenho. Define os tipos, ports e serviços que governam o ciclo de vida de indicadores, medições, ciclos de avaliação e quotas SIADAP.

---

## Contrato público

### Erros

```rust
pub struct MetricError { pub code: ErrorCode, pub component: Component, ... }
pub const COMPONENT: Component; // "core-metrics"
// Códigos: EmptyCode, InvalidCode, NotFound, AlreadyExists, VersionOverlap,
//           CycleNotClosed, MissingEvidence, InvalidStatus, QuotaViolation, ...
```

### Emissão de eventos

```rust
pub trait MetricEmitter: Send + Sync {
    fn emit(&self, event: &MetricEvent) -> Result<(), MetricError>;
}
pub struct FanoutEmitter { ... } // distribui por múltiplos emitters
pub struct InMemoryMetricRegistry { ... } // buffer em memória (testes)

pub struct MetricEvent {
    pub metric_code: String,
    pub instance_id: String,
    pub value: f64,
    pub recorded_at: DateTime<Utc>,
}
```

### Definição de métricas

```rust
pub struct MetricDefinition {
    pub code: String,         // ^[a-z][a-z0-9_.-]*$
    pub name: String,
    pub description: Option<String>,
    pub status: MetricDefinitionStatus,
    pub created_at: DateTime<Utc>,
}
pub enum MetricDefinitionStatus { Draft, Active, Suspended, Retired }
```

### Versão de métrica

```rust
pub struct MetricVersion {
    pub metric_code: String,
    pub version: u32,
    pub formula: Option<String>,
    pub calculation_binding: Option<CalculationBinding>,
    pub evidence_requirements: Vec<EvidenceRequirement>,
    pub status: MetricVersionStatus,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
}
pub struct CalculationBinding { pub engine: String, pub expression: String }
pub struct EvidenceRequirement { pub kind: EvidenceType, pub mandatory: bool }
pub enum EvidenceType { Document, Photo, Report, Other(String) }
pub enum MetricVersionStatus { Draft, Published, Deprecated }
```

### Ciclos de avaliação

```rust
pub struct EvaluationCycle {
    pub id: String,
    pub name: String,
    pub cycle_type: CycleType,
    pub status: CycleStatus,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}
pub enum CycleType { Annual, Semester, Quarter, Custom }
pub enum CycleStatus { Open, Closed, Archived }
```

### Instância de indicador

```rust
pub struct IndicatorInstance {
    pub id: String,
    pub metric_code: String,
    pub metric_version: u32,
    pub cycle_id: String,
    pub subject_id: String,   // entidade avaliada
    pub target: Option<TargetDefinition>,
    pub status: InstanceStatus,
}
pub enum InstanceStatus { Open, Submitted, Validated, Rejected, Closed }
```

### Resultado de medição

```rust
pub struct MeasurementResult {
    pub instance_id: String,
    pub value: f64,
    pub evidence: Vec<EvidenceLink>,
    pub status: MeasurementStatus,
    pub recorded_at: DateTime<Utc>,
    pub recorded_by: String,
}
pub struct EvidenceLink { pub kind: EvidenceType, pub reference: String }
pub enum MeasurementStatus { Draft, Submitted, Validated, Invalidated, Rectified }
```

### Targets

```rust
pub struct TargetDefinition {
    pub scope: ScopeType,
    pub thresholds: Vec<Threshold>,
}
pub enum ScopeType { Individual, Team, Unit, Organization }
pub struct Threshold { pub label: String, pub min: f64, pub max: Option<f64> }
```

### Paginação

```rust
pub struct ListOptions { pub page: u32, pub page_size: u32 }
pub struct Page<T> { pub items: Vec<T>, pub total: u64 }
```

### Auditoria de governação

```rust
pub struct GovernanceChangeLog { pub entries: Vec<GovernanceLogEntry> }
pub struct GovernanceLogEntry {
    pub entity_kind: String,
    pub entity_id: String,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
    pub description: String,
}
```

### Stores (ports)

```rust
pub trait MetricStore: Send + Sync { ... }        // deprecated alias
pub trait MetricDefinitionStore: Send + Sync { ... }
pub trait MetricVersionStore: Send + Sync { ... }
pub trait EvaluationCycleStore: Send + Sync { ... }
pub trait IndicatorInstanceStore: Send + Sync { ... }
pub trait MeasurementResultStore: Send + Sync { ... }
pub trait TargetDefinitionStore: Send + Sync { ... }
pub trait StorageMetricStore: MetricDefinitionStore + MetricVersionStore
    + EvaluationCycleStore + IndicatorInstanceStore
    + MeasurementResultStore + TargetDefinitionStore {}
```

### Serviço principal

```rust
pub struct MetricServiceBuilder;
impl MetricServiceBuilder {
    pub fn from_unified<S: StorageMetricStore>(store: S) -> Self;
    pub fn with_emitter(self, emitter: impl MetricEmitter) -> Self;
    pub fn build(self) -> MetricService;
}

pub struct MetricService { ... }
impl MetricService {
    pub fn emit_event(&self, event: MetricEvent) -> Result<(), MetricError>;
    pub fn publish_version(&self, version: MetricVersion) -> Result<(), MetricError>;
    pub fn close_cycle(&self, cycle_id: &str) -> Result<(), MetricError>;
    pub fn create_instance(&self, req: IndicatorInstance) -> Result<(), MetricError>;
    pub fn open_instances_for_cycle(&self, cycle_id: &str, subjects: &[String]) -> Result<(), MetricError>;
    pub fn save_result(&self, result: MeasurementResult) -> Result<(), MetricError>;
    pub fn save_results_batch(&self, results: Vec<MeasurementResult>) -> Result<(), MetricError>;
    pub fn validate_result(&self, instance_id: &str, by: &str) -> Result<(), MetricError>;
    pub fn invalidate_result(&self, instance_id: &str, by: &str, reason: &str) -> Result<(), MetricError>;
    pub fn rectify_result(&self, instance_id: &str, new_result: MeasurementResult) -> Result<(), MetricError>;
    pub fn calculate_result(&self, instance_id: &str) -> Result<f64, MetricError>;
    pub fn get_official_result(&self, instance_id: &str) -> Result<Option<MeasurementResult>, MetricError>;
}
```

### Fórmulas

```rust
pub trait FormulaEngine: Send + Sync {
    fn calculate(&self, expression: &str, context: &HashMap<String, f64>) -> Result<f64, MetricError>;
}
pub struct BasicFormulaEngine; // operações aritméticas básicas
```

### Agregação hierárquica

```rust
pub trait OrgHierarchyProvider: Send + Sync {
    fn get_children(&self, unit_id: &str) -> Vec<String>;
}
pub struct StaticOrgHierarchy { ... }
pub struct LevelAggregationService<H: OrgHierarchyProvider> { ... }
impl<H: OrgHierarchyProvider> LevelAggregationService<H> {
    pub fn aggregate(&self, kind: AggregationKind, unit_id: &str, values: &HashMap<String, f64>) -> f64;
}
pub enum AggregationKind { Sum, Average, Min, Max, WeightedAverage }
```

### SIADAP

```rust
// Siadap 1 — serviços
pub struct Siadap1EvaluationResult { pub rating: Siadap1Rating, pub score: f64 }
pub struct Siadap1QuotaConfig { pub excellent_pct: f64, pub relevant_pct: f64 }
pub enum Siadap1Rating { Excellent, Relevant, Adequate, Inadequate }
pub fn validate_siadap1_quotas(results: &[Siadap1EvaluationResult], config: &Siadap1QuotaConfig) -> QuotaValidationReport;
pub fn weighted_score_siadap1(objectives_score: f64, competency_score: f64) -> f64; // 75%+25%

// Siadap 2 — dirigentes
pub struct Siadap2EvaluationResult { pub rating: Siadap2Rating, pub score: f64 }
pub struct Siadap2QuotaConfig { pub excellent_pct: f64 }
pub enum Siadap2Rating { Excellent, Relevant, Adequate, Inadequate }
pub fn validate_siadap2_quotas(...) -> QuotaValidationReport;
pub fn weighted_score_siadap2(objectives_score: f64, competency_score: f64) -> f64; // 60%+40%

// Siadap 3 — trabalhadores
pub struct Siadap3EvaluationResult { pub rating: Siadap3Rating, pub score: f64 }
pub struct Siadap3QuotaConfig { pub excellent_pct: f64, pub relevant_pct: f64 }
pub enum Siadap3Rating { Excellent, Relevant, Adequate, Inadequate }
pub fn validate_siadap3_quotas(...) -> QuotaValidationReport;
pub fn weighted_score_siadap3(objectives_score: f64, competency_score: f64) -> f64; // 60%+40%

pub struct QuotaValidationReport { pub violations: Vec<QuotaViolation>, pub is_valid: bool }
pub struct QuotaViolation { pub quota_kind: String, pub allowed_pct: f64, pub actual_pct: f64 }

// Janela de avaliação intercalar
pub struct IntermediaryEvaluationWindow {
    pub cycle_id: String,
    pub opens_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
}
```

---

## Como usar

### Ciclo completo de avaliação

```rust
let service = MetricServiceBuilder::from_unified(my_store).build();

// 1. Publicar versão de uma métrica
service.publish_version(MetricVersion { metric_code: "obj.entregas".into(), version: 1, ... })?;

// 2. Abrir ciclo
// (gerido via EvaluationCycleStore directamente)

// 3. Abrir instâncias para todos os sujeitos do ciclo
service.open_instances_for_cycle("ciclo-2025", &["trabalhador-1", "trabalhador-2"])?;

// 4. Registar resultado
service.save_result(MeasurementResult {
    instance_id: "inst-xyz".into(),
    value: 4.5,
    evidence: vec![],
    status: MeasurementStatus::Submitted,
    recorded_by: "avaliador-1".into(),
    recorded_at: Utc::now(),
})?;

// 5. Validar e fechar ciclo
service.validate_result("inst-xyz", "avaliador-1")?;
service.close_cycle("ciclo-2025")?;
```

### Validação de quotas SIADAP 3

```rust
let results = vec![
    Siadap3EvaluationResult { rating: Siadap3Rating::Excellent, score: 5.0 },
    Siadap3EvaluationResult { rating: Siadap3Rating::Relevant, score: 4.0 },
    // ...
];
let config = Siadap3QuotaConfig { excellent_pct: 25.0, relevant_pct: 50.0 };
let report = validate_siadap3_quotas(&results, &config);
if !report.is_valid {
    for v in &report.violations {
        eprintln!("Quota violada: {} — permitido {:.0}%, actual {:.0}%", v.quota_kind, v.allowed_pct, v.actual_pct);
    }
}
```

---

## Invariantes

- Versões de métricas não se podem sobrepor temporalmente (`publish_version` rejeita sobreposição).
- Um ciclo só pode ser fechado quando todas as instâncias têm resultado `Validated`.
- `rectify_result` cria uma cadeia de rectificação — não apaga o resultado anterior.
- Pesos SIADAP são fixos por lei: S1/S3 objectivos 75%+competências 25% para S1, 60%+40% para S2 e S3.
- Códigos de métrica validados pela regex `^[a-z][a-z0-9_.-]*$`.

---

## Limites actuais

- `BasicFormulaEngine` suporta apenas `+`, `-`, `*`, `/` e variáveis simples.
- Sem suporte a métricas multi-nível num único `calculate_result` (requer `LevelAggregationService` separado).
- Sem internacionalização de labels de rating.

---

## ToDo

- [ ] Motor de fórmulas com funções (média, min, max, if).
- [ ] Suporte a periodicidade automática de ciclos (anual recorrente).
- [ ] Exportação de relatório SIADAP em formato normalizado.

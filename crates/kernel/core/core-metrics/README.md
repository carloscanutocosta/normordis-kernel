# core-metrics

Motor de avaliação de desempenho e métricas operacionais do mini-kernel.

## Objectivo

Fornece o contrato de domínio para definição, versionamento, atribuição, medição e agregação de métricas e indicadores de desempenho, com suporte nativo ao modelo SIADAP português (Siadap1, Siadap2, Siadap3).

## Posição arquitectural

`crates/kernel/core` — camada de domínio pura. Sem dependências de infraestrutura. Toda a persistência é injectada via ports (`MetricStore`, `MetricDefinitionStore`, etc.).

## Responsabilidade

- Definir os tipos de domínio: `MetricDefinition`, `MetricVersion`, `IndicatorInstance`, `MeasurementResult`, `EvaluationCycle`, `TargetDefinition`.
- Expor `MetricService` como serviço de domínio que coordena o ciclo de vida completo.
- Implementar lógica de quotas SIADAP (validação, cálculo de pontuação ponderada).
- Emitir eventos de métricas via `MetricEmitter` / `FanoutEmitter`.
- Calcular agregações por fórmula (`BasicFormulaEngine`) e por nível hierárquico (`LevelAggregationService`).

## Não-responsabilidade

- Não persiste dados — toda a persistência é delegada nos stores injectados.
- Não faz autenticação nem autorização.
- Não conhece SQLite, ficheiros ou qualquer infraestrutura concreta.

## Exemplo mínimo

```rust
use core_metrics::{MetricServiceBuilder, MetricEmitter, FanoutEmitter};

let store = MyMetricStore::new(); // implementação concreta do store
let emitter = FanoutEmitter::new(vec![]);
let service = MetricServiceBuilder::from_unified(store)
    .with_emitter(emitter)
    .build();

service.emit_event(MetricEvent { ... })?;
```

## Validação

```sh
cargo test -p core-metrics
```

# domain-numerador

Domínio NNS (Numeração Normalizada de Sequências) — séries de numeração configuráveis com formato, política de reset e atribuição atómica.

## Objectivo

Define os tipos, ports e serviço de domínio para numeração normalizada de sequências: séries de numeração com formato configurável (prefixo, ano, contador com padding), política de reset (nunca/anual/mensal) e atribuição atómica de números a documentos ou procedimentos.

## Posição arquitectural

`crates/domain` — domínio de negócio puro. Sem dependências de infraestrutura. A persistência é delegada nos ports `NumberingStore` e `NumberingSequenceRepository`.

## Responsabilidade

- Definir tipos: `NumberingSequence`, `NumberFormat`, `FormatPart`, `ResetPolicy`, `NumberingKind`.
- Definir o port `NumberingStore` (atribuição e consulta de números).
- Definir o port `NumberingSequenceRepository` (gestão de séries).
- Implementar `NumeradorService` com lógica de atribuição, cálculo de `period_key` e formatação.
- Suportar filtros de consulta (`AssignmentFilter`, `AssignmentMetadata`).

## Não-responsabilidade

- Não persiste dados — toda a persistência é delegada em `numerador-sqlite` ou outra implementação.
- Não controla permissões de quem pode atribuir números.
- Não gere a revogação ou reutilização de números (números são definitivos).

## Exemplo mínimo

```rust
use domain_numerador::{NumeradorService, AssignNumberRequest};

let service = NumeradorService::new(my_store);
let assigned = service.assign(&AssignNumberRequest {
    series_id: "oficio-dgf".into(),
    assigned_by: "user-1".into(),
    ..Default::default()
})?;
println!("{}", assigned.formatted_number); // "OF/0001/2025"
```

## Validação

```sh
cargo test -p domain-numerador
```

# support-interoperability

Orquestracao headless substituivel para interoperabilidade institucional.

## Objetivo

Fornecer uma camada transversal para mini-apps e hosts criarem pedidos de
exportacao interoperavel, aplicarem autorizacao injetavel e chamarem o port de
materializacao definido por `core-exports`.

## Responsabilidade

- Construir `ExportMaterializationRequest` de `core-exports`.
- Normalizar datasets tabulares e inferir colunas quando apropriado.
- Aplicar uma `ExportAuthorizationPolicy` substituivel.
- Orquestrar `ExportMaterializerPort` sem conhecer o adapter concreto.
- Ser reutilizavel por Tauri, CLI, servicos e testes.

## Nao responsabilidade

- Nao escreve ficheiros.
- Nao conhece SQLite, CSV, XML, XLSX, ZIP ou filesystem.
- Nao decide politicas reais de acesso; apenas recebe uma policy injetada.
- Nao substitui `core-exports`; depende dele como contrato comum.
- Nao substitui `infra-export`; chama-o atraves do port quando o host o injeta.

## Exemplo minimo

```rust
use core_exports::ExportFormat;
use serde_json::json;
use support_interoperability::{ExportAuthorizationContext, ExportRequestBuilder};

let mut row = core_exports::TabularRow::new();
row.insert("id".into(), json!("A-1"));

let request = ExportRequestBuilder::new(
    "exp:demo:1",
    ExportFormat::Csv,
    "target/demo.csv",
)
.columns(["id"])
.row(row)
.build()?;

let ctx = ExportAuthorizationContext {
    actor: "user:1".into(),
    purpose: "interoperability".into(),
    correlation_id: "corr-1".into(),
};

ctx.validate()?;
# Ok::<(), support_interoperability::InteroperabilityError>(())
```

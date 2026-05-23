# pdf-pipeline

Fila de renderização PDF assíncrona com gestão de jobs, métricas e cache de fontes Typst.

## Objectivo

Desacopla a submissão de pedidos de renderização PDF da execução efectiva, permitindo processar jobs em background com rastreamento de estado, métricas de latência e reutilização de fontes Typst em memória (`WarmFonts`).

## Posição arquitectural

`crates/kernel/infra/pdf` — serviço de infraestrutura. Depende de `documentos-pdf` (rendering) e `support-pdf` (PdfError).

## Responsabilidade

- Gerir uma fila de jobs PDF (`PdfJob`) com estados (`PdfJobState`).
- Processar jobs em background com `PdfPipeline`.
- Expor métricas de execução (`JobMetrics`): duração, erros, throughput.
- Reutilizar `WarmFonts` entre renders para performance.
- Devolver um `EnqueueResult` com o ID do job após submissão.

## Não-responsabilidade

- Não persiste jobs em base de dados — os jobs estão em memória.
- Não envia notificações quando um job termina.
- Não limita a fila (sem backpressure configurável).

## Exemplo mínimo

```rust
use pdf_pipeline::{PdfPipeline, PipelineConfig, PdfJobRequest};

let pipeline = PdfPipeline::start(PipelineConfig::default())?;
let result = pipeline.enqueue(PdfJobRequest {
    source: "#set page(paper: \"a4\")\nOlá".into(),
    ..Default::default()
})?;
println!("Job: {}", result.job_id);
```

## Validação

```sh
cargo test -p pdf-pipeline
```

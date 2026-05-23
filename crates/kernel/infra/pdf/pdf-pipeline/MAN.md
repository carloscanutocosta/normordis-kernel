# MAN — pdf-pipeline

## Objectivo

Fila assíncrona de renderização PDF. Permite submeter pedidos de rendering e processar em background com rastreamento de estado, métricas e reutilização de fontes Typst.

---

## Contrato público

```rust
pub struct PdfPipeline { ... }

impl PdfPipeline {
    /// Inicia o pipeline com a configuração dada.
    pub fn start(config: PipelineConfig) -> Result<Self, PipelineError>;

    /// Submete um job para a fila. Devolve imediatamente com o ID do job.
    pub fn enqueue(&self, request: PdfJobRequest) -> Result<EnqueueResult, PipelineError>;

    /// Consulta o estado de um job pelo ID.
    pub fn status(&self, job_id: &str) -> Result<PdfJobState, PipelineError>;

    /// Aguarda até o job terminar e devolve o PDF ou erro.
    pub fn wait(&self, job_id: &str) -> Result<Vec<u8>, PipelineError>;

    /// Métricas agregadas do pipeline desde o arranque.
    pub fn metrics(&self) -> JobMetrics;
}

pub struct PipelineConfig {
    pub worker_threads: usize,
    pub max_queue_size: Option<usize>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self { worker_threads: 2, max_queue_size: None }
    }
}

pub struct PdfJobRequest {
    pub source: String,           // código Typst ou NDT
    pub source_kind: JobSourceKind,
    pub metadata: Option<serde_json::Value>,
}

pub enum JobSourceKind { Typst, Ndt }

pub struct EnqueueResult {
    pub job_id: String,
    pub queued_at: DateTime<Utc>,
}

pub struct PdfJob {
    pub id: String,
    pub request: PdfJobRequest,
    pub state: PdfJobState,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub enum PdfJobState {
    Queued,
    Processing,
    Done { pdf: Vec<u8> },
    Failed { reason: String },
}

pub struct JobMetrics {
    pub jobs_enqueued: u64,
    pub jobs_done: u64,
    pub jobs_failed: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: u64,
}

pub struct PipelineError(pub String);

/// Re-exportado de documentos-pdf para conveniência.
pub use documentos_pdf::WarmFonts;
```

---

## Como usar

### Render simples

```rust
use pdf_pipeline::{PdfPipeline, PipelineConfig, PdfJobRequest, JobSourceKind};

let pipeline = PdfPipeline::start(PipelineConfig {
    worker_threads: 4,
    max_queue_size: Some(100),
})?;

let result = pipeline.enqueue(PdfJobRequest {
    source: r#"
        #set page(paper: "a4")
        #set text(lang: "pt")
        = Relatório mensal
    "#.into(),
    source_kind: JobSourceKind::Typst,
    metadata: None,
})?;

// Aguardar resultado (bloqueante)
let pdf_bytes = pipeline.wait(&result.job_id)?;
std::fs::write("relatorio.pdf", pdf_bytes)?;
```

### Múltiplos jobs em paralelo

```rust
let ids: Vec<_> = documentos.iter()
    .map(|doc| pipeline.enqueue(PdfJobRequest {
        source: doc.typst_source.clone(),
        source_kind: JobSourceKind::Typst,
        metadata: Some(serde_json::json!({ "doc_id": doc.id })),
    }).map(|r| r.job_id))
    .collect::<Result<_, _>>()?;

// Recolher resultados
for id in &ids {
    match pipeline.wait(id) {
        Ok(pdf) => println!("PDF pronto: {} bytes", pdf.len()),
        Err(e) => eprintln!("Falhou: {e}"),
    }
}
```

### Consulta de métricas

```rust
let m = pipeline.metrics();
println!("Jobs concluídos: {}/{}", m.jobs_done, m.jobs_enqueued);
println!("Duração média: {:.1}ms | Máx: {}ms", m.avg_duration_ms, m.max_duration_ms);
```

---

## Invariantes

- `WarmFonts` é inicializado uma vez no arranque do pipeline e partilhado entre todos os workers.
- `PdfJobState::Done` contém os bytes do PDF — só acessível via `wait()` ou `status()`.
- Jobs terminados (Done/Failed) são mantidos em memória até o pipeline ser descartado.
- `enqueue` é thread-safe — pode ser chamado de múltiplos threads simultaneamente.

---

## Limites actuais

- Jobs não são persistidos — reiniciar o processo perde todos os jobs em fila e os resultados.
- Sem backpressure configurável por omissão (fila ilimitada se `max_queue_size` for None).
- Sem timeout por job — um job que bloqueie o worker bloqueia o thread indefinidamente.
- `wait()` é bloqueante (não `async`).

---

## ToDo

- [ ] Timeout por job configurável.
- [ ] Persistência de jobs em SQLite para sobreviver a restarts.
- [ ] `wait_async()` com suporte a `tokio`.
- [ ] Callback/webhook quando job termina.

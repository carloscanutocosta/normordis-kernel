use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pedido de geração de PDF submetido pelo cliente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfJobRequest {
    /// Identificador do template (ex: "certidao", "declaracao").
    pub template_id: String,
    /// Versão semântica do template (ex: "1.0.0").
    pub template_version: String,
    /// Fonte Typst completa, com payload já substituído.
    pub source: String,
    /// Versão dos assets (fontes, imagens) utilizados.
    pub assets_version: String,
}

/// Job criado a partir de um `PdfJobRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfJob {
    pub job_id: Uuid,
    /// Hash SHA-256 determinístico que identifica o resultado.
    pub hash: String,
    pub request: PdfJobRequest,
    pub created_at: DateTime<Utc>,
}

/// Estado do ciclo de vida de um job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status")]
pub enum JobStatus {
    Queued,
    Preparing,
    Compiling,
    Exporting,
    Storing,
    Done { from_cache: bool },
    Failed { reason: String },
}

/// Métricas de tempo recolhidas por job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobMetrics {
    /// Tempo em fila antes de ser recolhido por um worker (ms).
    pub queue_wait_ms: u64,
    /// Construção do contexto de compilação (ms).
    pub prepare_ms: u64,
    /// Compilação Typst: `typst::compile()` (ms).
    pub typst_compile_ms: u64,
    /// Exportação PDF: `typst_pdf::pdf()` (ms).
    pub typst_export_ms: u64,
    /// Gravação do PDF em cache (ms).
    pub store_output_ms: u64,
    /// Tempo total do job desde criação até Done/Failed (ms).
    pub total_ms: u64,
}

/// Estado completo de um job, consultável via `PdfPipeline::status()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfJobState {
    pub job: PdfJob,
    pub status: JobStatus,
    /// Worker que processou o job (None enquanto em fila).
    pub worker_id: Option<usize>,
    pub metrics: JobMetrics,
}

/// Resultado devolvido por `PdfPipeline::enqueue()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueResult {
    pub job_id: Uuid,
    pub status: JobStatus,
}

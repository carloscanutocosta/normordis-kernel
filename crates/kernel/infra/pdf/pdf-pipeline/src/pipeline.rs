use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use render_typst::pdf::{compile_with_warm_fonts_timed, WarmFonts};

use crate::types::*;

// ---------------------------------------------------------------------------
// Configuração
// ---------------------------------------------------------------------------

pub struct PipelineConfig {
    /// Número de workers em paralelo. Default: 2.
    pub workers: usize,
    /// Directório onde os PDFs compilados são guardados.
    pub cache_dir: PathBuf,
    /// Directório de fontes personalizadas (`.ttf`/`.otf`). None usa apenas embedded.
    pub fonts_dir: Option<PathBuf>,
    /// Tamanho máximo da fila. Enqueue devolve erro quando atingido.
    pub max_queue_size: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            workers: 2,
            cache_dir: PathBuf::from("runtime/cache"),
            fonts_dir: None,
            max_queue_size: 50,
        }
    }
}

// ---------------------------------------------------------------------------
// Erros
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("fila cheia ({0} jobs pendentes)")]
    QueueFull(usize),
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Estado interno partilhado entre workers
// ---------------------------------------------------------------------------

struct SharedQueue {
    pending: Mutex<VecDeque<PdfJob>>,
    condvar: Condvar,
    shutdown: AtomicBool,
    max_size: usize,
}

pub(crate) struct PipelineInner {
    /// Fontes pré-parseadas uma vez no arranque — partilhadas entre todos os workers.
    pub(crate) warm_fonts: Arc<WarmFonts>,
    cache_dir: PathBuf,
    queue: SharedQueue,
    /// Mapa de estado de todos os jobs conhecidos.
    job_states: Mutex<HashMap<Uuid, PdfJobState>>,
    /// Cache de texto genérico (templates, configurações) com chave = caminho absoluto.
    /// Elimina leituras repetidas de rede para ficheiros que não mudam durante a sessão.
    text_cache: Mutex<HashMap<PathBuf, Arc<str>>>,
}

// ---------------------------------------------------------------------------
// Pipeline público
// ---------------------------------------------------------------------------

pub struct PdfPipeline {
    inner: Arc<PipelineInner>,
    /// Handles guardados para que os threads vivam enquanto o pipeline existir.
    _workers: Vec<std::thread::JoinHandle<()>>,
}

impl PdfPipeline {
    /// Inicializa o pipeline: parseia fontes, cria diretórios, arranca workers.
    pub fn new(config: PipelineConfig) -> Result<Self, PipelineError> {
        std::fs::create_dir_all(&config.cache_dir)?;

        let raw_bytes = load_font_bytes(config.fonts_dir.as_deref())?;
        let warm_fonts = Arc::new(WarmFonts::from_bytes(&raw_bytes));

        let inner = Arc::new(PipelineInner {
            warm_fonts,
            cache_dir: config.cache_dir,
            queue: SharedQueue {
                pending: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                shutdown: AtomicBool::new(false),
                max_size: config.max_queue_size,
            },
            job_states: Mutex::new(HashMap::new()),
            text_cache: Mutex::new(HashMap::new()),
        });

        let mut handles = Vec::with_capacity(config.workers);
        for worker_id in 0..config.workers {
            let inner = Arc::clone(&inner);
            let handle = std::thread::Builder::new()
                .name(format!("pdf-worker-{worker_id}"))
                .spawn(move || worker_loop(worker_id, inner))?;
            handles.push(handle);
        }

        Ok(Self {
            inner,
            _workers: handles,
        })
    }

    /// Submete um pedido de geração.
    ///
    /// Se o hash já existir em cache, devolve `Done { from_cache: true }` imediatamente.
    /// Se a fila estiver cheia, devolve `PipelineError::QueueFull`.
    pub fn enqueue(&self, request: PdfJobRequest) -> Result<EnqueueResult, PipelineError> {
        let hash = compute_hash(&request);
        let cache_path = self.inner.cache_dir.join(format!("{hash}.pdf"));

        // Cache hit — devolução imediata sem entrar na fila
        if cache_path.exists() {
            let job_id = Uuid::new_v4();
            let job = PdfJob {
                job_id,
                hash,
                request,
                created_at: Utc::now(),
            };
            let status = JobStatus::Done { from_cache: true };
            self.inner.job_states.lock().unwrap().insert(
                job_id,
                PdfJobState {
                    job,
                    status: status.clone(),
                    worker_id: None,
                    metrics: JobMetrics::default(),
                },
            );
            return Ok(EnqueueResult { job_id, status });
        }

        // Verificar se já existe um job em curso com o mesmo hash
        {
            let states = self.inner.job_states.lock().unwrap();
            if let Some(existing) = states.values().find(|s| s.job.hash == hash) {
                return Ok(EnqueueResult {
                    job_id: existing.job.job_id,
                    status: existing.status.clone(),
                });
            }
        }

        // Verificar backpressure
        {
            let pending = self.inner.queue.pending.lock().unwrap();
            if pending.len() >= self.inner.queue.max_size {
                return Err(PipelineError::QueueFull(pending.len()));
            }
        }

        // Criar job e enfileirar
        let job_id = Uuid::new_v4();
        let job = PdfJob {
            job_id,
            hash,
            request,
            created_at: Utc::now(),
        };
        let status = JobStatus::Queued;

        self.inner.job_states.lock().unwrap().insert(
            job_id,
            PdfJobState {
                job: job.clone(),
                status: status.clone(),
                worker_id: None,
                metrics: JobMetrics::default(),
            },
        );

        self.inner.queue.pending.lock().unwrap().push_back(job);
        self.inner.queue.condvar.notify_one();

        Ok(EnqueueResult { job_id, status })
    }

    /// Consulta o estado actual de um job.
    pub fn status(&self, job_id: Uuid) -> Option<PdfJobState> {
        self.inner.job_states.lock().unwrap().get(&job_id).cloned()
    }

    /// Devolve um `Arc` para as fontes pré-parseadas no arranque.
    ///
    /// Usar para compilações directas fora do sistema de fila (ex: Tauri commands).
    /// O `Arc` é barato de clonar — todos os dados de fonte são partilhados.
    pub fn warm_fonts(&self) -> Arc<WarmFonts> {
        Arc::clone(&self.inner.warm_fonts)
    }

    /// Lê texto de `key` a partir de cache; na primeira chamada, invoca `loader` para
    /// carregar e armazenar o valor. Devolve `None` apenas se `loader` devolver `None`.
    ///
    /// Usar para templates e outros ficheiros estáticos lidos de partilhas de rede,
    /// eliminando leituras repetidas após a primeira chamada da sessão.
    pub fn get_or_load_text(
        &self,
        key: &Path,
        loader: impl FnOnce(&Path) -> Option<String>,
    ) -> Option<Arc<str>> {
        {
            let cache = self.inner.text_cache.lock().unwrap();
            if let Some(s) = cache.get(key) {
                return Some(Arc::clone(s));
            }
        }
        let value = loader(key)?;
        let arc: Arc<str> = Arc::from(value);
        self.inner
            .text_cache
            .lock()
            .unwrap()
            .insert(key.to_path_buf(), Arc::clone(&arc));
        Some(arc)
    }

    /// Lê os bytes do PDF de um job concluído.
    ///
    /// Devolve `None` se o job ainda não estiver `Done` ou não tiver PDF em cache.
    pub fn read_pdf(&self, job_id: Uuid) -> Option<Vec<u8>> {
        let states = self.inner.job_states.lock().unwrap();
        let state = states.get(&job_id)?;
        if !matches!(state.status, JobStatus::Done { .. }) {
            return None;
        }
        let path = self.inner.cache_dir.join(format!("{}.pdf", state.job.hash));
        std::fs::read(path).ok()
    }
}

impl Drop for PdfPipeline {
    fn drop(&mut self) {
        self.inner.queue.shutdown.store(true, Ordering::SeqCst);
        self.inner.queue.condvar.notify_all();
        // Os handles são dropped aqui, mas os threads terminam no seu próprio tempo.
        // Não fazemos join() para não bloquear o caller.
    }
}

// ---------------------------------------------------------------------------
// Worker loop
// ---------------------------------------------------------------------------

fn worker_loop(worker_id: usize, inner: Arc<PipelineInner>) {
    loop {
        let job = {
            let mut pending = inner.queue.pending.lock().unwrap();
            loop {
                if let Some(job) = pending.pop_front() {
                    break job;
                }
                if inner.queue.shutdown.load(Ordering::SeqCst) {
                    return;
                }
                pending = inner.queue.condvar.wait(pending).unwrap();
            }
        };

        process_job(worker_id, &inner, job);
    }
}

fn process_job(worker_id: usize, inner: &Arc<PipelineInner>, job: PdfJob) {
    let t_total = Instant::now();
    let job_id = job.job_id;
    let hash = job.hash.clone();
    let cache_path = inner.cache_dir.join(format!("{hash}.pdf"));

    // Calcular tempo em fila
    let queue_wait_ms = Utc::now()
        .signed_duration_since(job.created_at)
        .num_milliseconds()
        .max(0) as u64;

    let mut metrics = JobMetrics {
        queue_wait_ms,
        ..Default::default()
    };

    // -- Preparing --
    set_status(inner, job_id, worker_id, JobStatus::Preparing);
    let t_prepare = Instant::now();
    let source = job.request.source.clone();
    metrics.prepare_ms = t_prepare.elapsed().as_millis() as u64;

    // -- Compiling + Exporting --
    set_status(inner, job_id, worker_id, JobStatus::Compiling);
    let compile_result = compile_with_warm_fonts_timed(&source, &inner.warm_fonts);

    let pdf_bytes = match compile_result {
        Ok(r) => {
            metrics.typst_compile_ms = r.compile_ms;
            metrics.typst_export_ms = r.export_ms;
            r.pdf_bytes
        }
        Err(e) => {
            metrics.total_ms = t_total.elapsed().as_millis() as u64;
            set_status_with_metrics(
                inner,
                job_id,
                worker_id,
                JobStatus::Failed {
                    reason: e.to_string(),
                },
                metrics,
            );
            return;
        }
    };

    // -- Storing --
    set_status(inner, job_id, worker_id, JobStatus::Storing);
    let t_store = Instant::now();
    if let Err(e) = std::fs::write(&cache_path, &pdf_bytes) {
        metrics.total_ms = t_total.elapsed().as_millis() as u64;
        set_status_with_metrics(
            inner,
            job_id,
            worker_id,
            JobStatus::Failed {
                reason: format!("erro ao gravar cache: {e}"),
            },
            metrics,
        );
        return;
    }
    metrics.store_output_ms = t_store.elapsed().as_millis() as u64;
    metrics.total_ms = t_total.elapsed().as_millis() as u64;

    set_status_with_metrics(
        inner,
        job_id,
        worker_id,
        JobStatus::Done { from_cache: false },
        metrics,
    );
}

// ---------------------------------------------------------------------------
// Helpers de estado
// ---------------------------------------------------------------------------

fn set_status(inner: &Arc<PipelineInner>, job_id: Uuid, worker_id: usize, status: JobStatus) {
    if let Some(s) = inner.job_states.lock().unwrap().get_mut(&job_id) {
        s.status = status;
        s.worker_id = Some(worker_id);
    }
}

fn set_status_with_metrics(
    inner: &Arc<PipelineInner>,
    job_id: Uuid,
    worker_id: usize,
    status: JobStatus,
    metrics: JobMetrics,
) {
    if let Some(s) = inner.job_states.lock().unwrap().get_mut(&job_id) {
        s.status = status;
        s.worker_id = Some(worker_id);
        s.metrics = metrics;
    }
}

// ---------------------------------------------------------------------------
// Hash determinístico
// ---------------------------------------------------------------------------

fn compute_hash(req: &PdfJobRequest) -> String {
    let mut h = Sha256::new();
    h.update(req.template_id.as_bytes());
    h.update(b"\x00");
    h.update(req.template_version.as_bytes());
    h.update(b"\x00");
    h.update(req.source.as_bytes());
    h.update(b"\x00");
    h.update(req.assets_version.as_bytes());
    hex::encode(h.finalize())
}

// ---------------------------------------------------------------------------
// Carregamento de fontes
// ---------------------------------------------------------------------------

fn load_font_bytes(fonts_dir: Option<&Path>) -> Result<Vec<Vec<u8>>, std::io::Error> {
    let mut fonts = Vec::new();
    if let Some(dir) = fonts_dir {
        load_fonts_recursive(dir, &mut fonts)?;
    }
    Ok(fonts)
}

fn load_fonts_recursive(dir: &Path, out: &mut Vec<Vec<u8>>) -> Result<(), std::io::Error> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            load_fonts_recursive(&path, out)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf") {
                out.push(std::fs::read(&path)?);
            }
        }
    }
    Ok(())
}

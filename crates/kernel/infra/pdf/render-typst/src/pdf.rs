use std::io::Write;
use std::path::Path;
use std::time::Instant;
use thiserror::Error;
use typst_as_lib::TypstEngine;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("erro de compilação Typst: {0}")]
    Compile(String),
    #[error("erro de exportação PDF: {0}")]
    Export(String),
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// Resultado de compilação com métricas de tempo detalhadas.
#[derive(Debug)]
pub struct CompileResult {
    pub pdf_bytes: Vec<u8>,
    /// Tempo de `typst::compile()` em milissegundos.
    pub compile_ms: u64,
    /// Tempo de `typst_pdf::pdf()` em milissegundos.
    pub export_ms: u64,
}

// ─── WarmFonts ────────────────────────────────────────────────────────────────

/// Fontes pré-parseadas e `FontBook` pré-construído para reutilização entre
/// compilações.
///
/// O custo de parsear os ficheiros `.ttf`/`.otf` (incluindo os embedded do
/// `typst_assets`) é pago uma única vez, no arranque do processo.  Cada
/// compilação subsequente recebe referências `Arc` aos objectos já construídos,
/// sem qualquer I/O nem parsing repetido.
///
/// Thread-safe: `Font` é `Arc<Repr>` internamente; `WarmFonts` implementa
/// `Send + Sync` e pode ser partilhado entre threads de `spawn_blocking`.
pub struct WarmFonts {
    library: typst::utils::LazyHash<typst::Library>,
    book: typst::utils::LazyHash<typst::text::FontBook>,
    fonts: Vec<typst::text::Font>,
}

impl WarmFonts {
    /// Constrói `WarmFonts` a partir de bytes de fontes personalizadas
    /// e dos fonts embebidos no binário via `typst_assets`.
    ///
    /// Operação cara — invocar apenas uma vez no arranque do processo.
    pub fn from_bytes(custom_bytes: &[Vec<u8>]) -> Self {
        use typst::foundations::Bytes;
        use typst::text::{Font, FontBook};

        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        // Fontes personalizadas (lidas do disco uma vez externamente)
        for raw in custom_bytes {
            let bytes = Bytes::new(raw.clone());
            for font in Font::iter(bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        // Fontes embedded do typst_assets (Libertinus, DejaVu, etc.)
        for raw in typst_assets::fonts() {
            let bytes = Bytes::new(raw.to_vec());
            for font in Font::iter(bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        log_diag(&format!(
            "[warm-fonts] {} fontes parseadas ({} custom + {} embedded)\n",
            fonts.len(),
            custom_bytes.len(),
            fonts.len().saturating_sub(custom_bytes.len()),
        ));

        use typst::LibraryExt as _;
        Self {
            library: typst::utils::LazyHash::new(typst::Library::builder().build()),
            book: typst::utils::LazyHash::new(book),
            fonts,
        }
    }

    /// Número total de fontes disponíveis (custom + embedded).
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
}

// ─── World mínimo para compilações de fonte única ─────────────────────────────

/// Implementação de `typst::World` para um único documento sem imports externos.
///
/// Partilha `WarmFonts` (via referência) entre chamadas: `library`, `book` e
/// `fonts` são lidos apenas por referência — nenhum dado é copiado ou reparse
/// a cada compilação.
struct SingleSourceWorld<'a> {
    warm: &'a WarmFonts,
    source: typst::syntax::Source,
    /// Ficheiros adicionais servidos ao compilador Typst (ex: imagens).
    /// Cada entrada é (nome-do-ficheiro, bytes).
    extra_files: &'a [(String, Vec<u8>)],
}

impl typst::World for SingleSourceWorld<'_> {
    fn library(&self) -> &typst::utils::LazyHash<typst::Library> {
        &self.warm.library
    }

    fn book(&self) -> &typst::utils::LazyHash<typst::text::FontBook> {
        &self.warm.book
    }

    fn main(&self) -> typst::syntax::FileId {
        self.source.id()
    }

    fn source(&self, id: typst::syntax::FileId) -> typst::diag::FileResult<typst::syntax::Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().into(),
            ))
        }
    }

    fn file(
        &self,
        id: typst::syntax::FileId,
    ) -> typst::diag::FileResult<typst::foundations::Bytes> {
        let vpath = id.vpath().as_rootless_path();
        // Compara pelo nome do ficheiro (último componente) ou pelo path completo.
        let file_name = vpath.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let vpath_str = vpath.to_string_lossy();
        for (name, bytes) in self.extra_files {
            if name == file_name || name.as_str() == vpath_str.as_ref() {
                return Ok(typst::foundations::Bytes::new(bytes.clone()));
            }
        }
        Err(typst::diag::FileError::NotFound(vpath.into()))
    }

    fn font(&self, index: usize) -> Option<typst::text::Font> {
        self.warm.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        use chrono::Datelike as _;
        let now = chrono::Utc::now();
        let now = if let Some(hours) = offset {
            now.checked_add_signed(chrono::Duration::hours(hours))
                .unwrap_or(now)
        } else {
            now
        };
        let d = now.date_naive();
        typst::foundations::Datetime::from_ymd(
            d.year(),
            (d.month0() + 1) as u8,
            (d.day0() + 1) as u8,
        )
    }
}

// ─── Funções de compilação ────────────────────────────────────────────────────

/// Compila Typst para PDF usando fontes totalmente pré-parseadas (`WarmFonts`).
///
/// Caminho mais rápido: nenhum parse de fonte, nenhuma leitura de disco.
/// `library` e `book` são partilhados por referência; `fonts` clonados por Arc.
/// Variante de [`compile_with_warm_fonts_timed`] que aceita ficheiros adicionais
/// (ex: imagens) servidos inline ao compilador Typst.
pub fn compile_with_warm_fonts_and_files(
    typst_source: &str,
    warm: &WarmFonts,
    extra_files: &[(String, Vec<u8>)],
) -> Result<CompileResult, PdfError> {
    use typst::layout::PagedDocument;

    let world = SingleSourceWorld {
        warm,
        source: typst::syntax::Source::detached(typst_source.to_owned()),
        extra_files,
    };

    let t_compile = Instant::now();
    let warned = typst::compile::<PagedDocument>(&world);
    let compile_ms = t_compile.elapsed().as_millis() as u64;

    for w in &warned.warnings {
        log_diag(&format!("[typst warn] {:?}\n", w));
    }

    let doc = warned.output.map_err(|errors| {
        let detail: String = errors
            .iter()
            .map(|d| {
                let loc = world.source.range(d.span).map(|r| {
                    let byte = r.start;
                    let mut line = 1usize;
                    let mut col = 1usize;
                    for (i, ch) in typst_source.char_indices() {
                        if i >= byte {
                            break;
                        }
                        if ch == '\n' {
                            line += 1;
                            col = 1;
                        } else {
                            col += 1;
                        }
                    }
                    format!("linha {}:{}", line, col)
                });
                let snippet = world.source.range(d.span).map(|r| {
                    let s = r.start.min(typst_source.len());
                    let e = r.end.min(typst_source.len());
                    typst_source[s..e].replace('\n', "↵")
                });
                format!(
                    "[{} @ {} — {:?}]",
                    d.message,
                    loc.unwrap_or_else(|| "?".into()),
                    snippet.unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        PdfError::Compile(detail)
    })?;

    comemo::evict(0);

    let t_export = Instant::now();
    let pdf_bytes = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| PdfError::Export(format!("{e:?}")))?;
    let export_ms = t_export.elapsed().as_millis() as u64;

    log_diag(&format!(
        "[warm-world] {} bytes (compile={}ms export={}ms)\n",
        pdf_bytes.len(),
        compile_ms,
        export_ms,
    ));

    Ok(CompileResult {
        pdf_bytes,
        compile_ms,
        export_ms,
    })
}

pub fn compile_with_warm_fonts_timed(
    typst_source: &str,
    warm: &WarmFonts,
) -> Result<CompileResult, PdfError> {
    use typst::layout::PagedDocument;

    let world = SingleSourceWorld {
        warm,
        source: typst::syntax::Source::detached(typst_source.to_owned()),
        extra_files: &[],
    };

    let t_compile = Instant::now();
    let warned = typst::compile::<PagedDocument>(&world);
    let compile_ms = t_compile.elapsed().as_millis() as u64;

    for w in &warned.warnings {
        log_diag(&format!("[typst warn] {:?}\n", w));
    }

    let doc = warned
        .output
        .map_err(|e| PdfError::Compile(format!("{e:?}")))?;

    // Limpa o cache de memoização comemo entre compilações independentes.
    comemo::evict(0);

    let t_export = Instant::now();
    let pdf_bytes = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| PdfError::Export(format!("{e:?}")))?;
    let export_ms = t_export.elapsed().as_millis() as u64;

    log_diag(&format!(
        "[warm-world] {} bytes (compile={}ms export={}ms)\n",
        pdf_bytes.len(),
        compile_ms,
        export_ms,
    ));

    Ok(CompileResult {
        pdf_bytes,
        compile_ms,
        export_ms,
    })
}

/// Compila código-fonte Typst para bytes PDF usando bytes de fontes pré-carregados.
///
/// Evita leitura de disco de rede — fontes já em memória como `Vec<Vec<u8>>`.
/// Ainda faz parse dos bytes em `Font` a cada chamada; prefira
/// [`compile_with_warm_fonts_timed`] quando `WarmFonts` estiver disponível.
pub fn compile_with_fonts_timed(
    typst_source: &str,
    font_bytes: &[Vec<u8>],
) -> Result<CompileResult, PdfError> {
    let engine = TypstEngine::builder()
        .main_file(typst_source)
        .fonts(
            font_bytes
                .iter()
                .map(|b| b.as_slice())
                .chain(typst_assets::fonts().map(|b| -> &[u8] { b })),
        )
        .build();

    let t_compile = Instant::now();
    let warned = engine.compile();
    let compile_ms = t_compile.elapsed().as_millis() as u64;

    for w in &warned.warnings {
        log_diag(&format!("[typst warn] {:?}\n", w));
    }

    let doc = warned
        .output
        .map_err(|e| PdfError::Compile(format!("{e:?}")))?;

    let t_export = Instant::now();
    let pdf_bytes = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| PdfError::Export(format!("{e:?}")))?;
    let export_ms = t_export.elapsed().as_millis() as u64;

    Ok(CompileResult {
        pdf_bytes,
        compile_ms,
        export_ms,
    })
}

/// Compila código-fonte Typst para bytes PDF.
///
/// Se `fonts_dir` existir e contiver ficheiros `.ttf`/`.otf`, são carregados
/// e disponibilizados ao compilador.
pub fn compile_pdf(typst_source: &str, fonts_dir: Option<&Path>) -> Result<Vec<u8>, PdfError> {
    let custom_fonts = load_fonts(fonts_dir)?;

    let embedded_font_count = typst_assets::fonts().count();
    log_diag(&format!(
        "[typst-pdf] source_len={} embedded_fonts={} custom_fonts={} fonts_dir={:?}\n",
        typst_source.len(),
        embedded_font_count,
        custom_fonts.len(),
        fonts_dir
    ));

    let result = compile_with_fonts_timed(typst_source, &custom_fonts)?;

    let pdf_bytes = if result.pdf_bytes.len() < 5_000 && !custom_fonts.is_empty() {
        log_diag(&format!(
            "[typst-pdf] PDF suspeito ({} bytes) com {} custom fonts — a repetir com embedded only\n",
            result.pdf_bytes.len(),
            custom_fonts.len()
        ));
        compile_with_fonts_timed(typst_source, &[])?.pdf_bytes
    } else {
        result.pdf_bytes
    };

    log_diag(&format!("[typst-pdf] pdf_bytes={}\n", pdf_bytes.len()));
    Ok(pdf_bytes)
}

// ─── support_pdf::PdfRenderer ─────────────────────────────────────────────────

impl support_pdf::PdfRenderer for WarmFonts {
    fn render(&self, source: &str) -> Result<Vec<u8>, support_pdf::PdfError> {
        compile_with_warm_fonts_and_files(source, self, &[])
            .map(|r| r.pdf_bytes)
            .map_err(|e| support_pdf::PdfError(e.to_string()))
    }
}

// ─── Diagnóstico ──────────────────────────────────────────────────────────────

fn log_diag(msg: &str) {
    eprint!("{msg}");
    let log_path = std::env::current_exe().ok().and_then(|p| {
        let canonical = p.canonicalize().unwrap_or(p);
        let stripped = strip_unc_prefix(canonical);
        stripped
            .parent()
            .map(|d| d.join("logs").join("typst-pdf-diag.log"))
    });
    if let Some(path) = log_path {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = f.write_all(msg.as_bytes());
        }
    }
}

fn strip_unc_prefix(path: std::path::PathBuf) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        std::path::PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(rest.to_string())
    } else {
        path
    }
}

fn load_fonts(fonts_dir: Option<&Path>) -> Result<Vec<Vec<u8>>, std::io::Error> {
    let mut fonts = Vec::new();
    if let Some(dir) = fonts_dir {
        load_fonts_recursive(dir, &mut fonts)?;
    }
    Ok(fonts)
}

fn load_fonts_recursive(dir: &Path, fonts: &mut Vec<Vec<u8>>) -> Result<(), std::io::Error> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            load_fonts_recursive(&path, fonts)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf") {
                fonts.push(std::fs::read(&path)?);
            }
        }
    }
    Ok(())
}

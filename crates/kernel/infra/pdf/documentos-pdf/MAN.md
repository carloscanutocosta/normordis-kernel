# MAN — documentos-pdf

## Objectivo

Facade de renderização PDF para documentos institucionais. Agrega a renderização Typst (com cache de fontes) e a engine NDT/NDF (`normordis-pdf`) numa única interface, com feature flags para activar apenas o que é necessário.

---

## Contrato público

### Feature `typst`

```rust
/// Cache de fontes Typst em memória — carregada uma vez, reutilizada.
pub struct WarmFonts { ... }

impl WarmFonts {
    /// Carrega as fontes do sistema e/ou fontes embebidas.
    pub fn load() -> Result<Self, PdfError>;
}

/// Renderiza código Typst em bytes PDF.
pub fn render_typst_document(source: &str, fonts: &WarmFonts) -> Result<Vec<u8>, PdfError>;
```

### Feature `normordis`

```rust
/// Compila um documento NDT (formato de template normativo) em estrutura intermédia.
pub fn compile_ndt(source: &str) -> Result<CompiledNdt, PdfError>;

/// Renderiza um documento NDF para visualização (PDF final).
pub fn render_ndf(ndf: &Ndf) -> Result<Vec<u8>, PdfError>;

/// Renderiza um documento NDF com marcadores de assinatura (para workflow de assinatura).
pub fn render_ndf_for_signing(ndf: &Ndf) -> Result<Vec<u8>, PdfError>;

/// Compila NDT e renderiza directamente para PDF (pipeline completo).
pub fn render_ndt(source: &str) -> Result<Vec<u8>, PdfError>;
```

### Erro unificado (re-exportado de `support-pdf`)

```rust
pub use support_pdf::PdfError;

pub struct PdfError(pub String);
impl std::error::Error for PdfError { ... }
```

---

## Como usar

### Renderização Typst com cache de fontes

```rust
use documentos_pdf::{WarmFonts, render_typst_document};

// Carregar fontes uma vez (ex.: no startup da aplicação)
let fonts = WarmFonts::load()?;

// Reutilizar em múltiplos documentos
let pdf = render_typst_document(r#"
#set page(paper: "a4")
#set text(lang: "pt", font: "DejaVu Sans")

= Ofício n.º 001/2025

Exmo. Sr. Director,
"#, &fonts)?;

std::fs::write("oficio.pdf", pdf)?;
```

### Pipeline NDT → PDF

```rust
use documentos_pdf::render_ndt;

let ndt_source = include_str!("templates/oficio.ndt");
let pdf_bytes = render_ndt(ndt_source)?;
```

### Renderização NDF para assinatura

```rust
use documentos_pdf::{compile_ndt, render_ndf_for_signing};

let compiled = compile_ndt(ndt_source)?;
let ndf = compiled.apply_data(&document_data)?;
let pdf_for_signing = render_ndf_for_signing(&ndf)?;
// Enviar para o serviço de assinatura digital
```

---

## Invariantes

- `WarmFonts` deve ser criado uma única vez por processo e partilhado — a criação é cara (leitura de fontes de disco/memória).
- `render_typst_document` é thread-safe se `WarmFonts` for partilhado via `Arc`.
- `PdfError` contém a mensagem de erro original do engine — não é estruturado.

---

## Limites actuais

- Sem suporte a fontes personalizadas sem recompilar o crate.
- `PdfError` é opaco (string) — dificulta handling programático de erros específicos.
- Sem cache de compilação Typst inter-invocações (cada `render_typst_document` recompila).

---

## ToDo

- [ ] Suporte a fontes carregadas de directório configurável.
- [ ] Cache de compilação Typst para templates repetidos.
- [ ] `PdfError` estruturado com variantes.

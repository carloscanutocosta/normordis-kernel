# MAN — support-pdf

## Objectivo

Interface mínima de renderização PDF. Define o contrato que qualquer engine de renderização deve implementar, desacoplando o domínio da engine concreta.

---

## Contrato público

```rust
/// Trait de renderização PDF. Implementado por engines concretas.
pub trait PdfRenderer: Send + Sync {
    /// Renderiza `source` (código fonte no formato da engine)
    /// e devolve os bytes do PDF resultante.
    fn render(&self, source: &str) -> Result<Vec<u8>, PdfError>;
}

/// Erro de renderização — mensagem de texto da engine.
pub struct PdfError(pub String);

impl std::fmt::Display for PdfError { ... }
impl std::error::Error for PdfError { ... }
```

---

## Como usar

### Implementar uma engine

```rust
use support_pdf::{PdfRenderer, PdfError};

pub struct TypstRenderer {
    // estado da engine (fontes, configuração, etc.)
}

impl PdfRenderer for TypstRenderer {
    fn render(&self, source: &str) -> Result<Vec<u8>, PdfError> {
        typst_engine::compile(source)
            .map_err(|e| PdfError(e.to_string()))
    }
}
```

### Usar como port injectado

```rust
use support_pdf::PdfRenderer;

fn generate_report<R: PdfRenderer>(
    renderer: &R,
    template: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let pdf = renderer.render(template)?;
    Ok(pdf)
}

// Em produção:
generate_report(&TypstRenderer::new(), include_str!("report.typ"))?;

// Em testes:
struct MockRenderer;
impl PdfRenderer for MockRenderer {
    fn render(&self, _source: &str) -> Result<Vec<u8>, PdfError> {
        Ok(b"%PDF-1.4 mock".to_vec())
    }
}
generate_report(&MockRenderer, "qualquer coisa")?;
```

---

## Invariantes

- `PdfRenderer` é `Send + Sync` — pode ser partilhado entre threads.
- `PdfError` é opaco — não tem variantes estruturadas. Trate como mensagem de diagnóstico.
- `render` deve ser determinístico dado o mesmo `source` e estado da engine.

---

## Limites actuais

- `PdfError` não é estruturado — impossível distinguir programaticamente erros de sintaxe de erros de fontes.
- Sem suporte a `source` binário (ex.: HTML com recursos embutidos via bytes).
- Sem método de validação sem renderização completa.

---

## ToDo

- [ ] `PdfError` estruturado com variantes (`SyntaxError`, `FontNotFound`, `EngineError`).
- [ ] Método `validate(&self, source: &str) -> Result<(), PdfError>` para verificação sem render.
- [ ] Suporte a `render_bytes(&self, source: &[u8])` para engines que aceitam input binário.

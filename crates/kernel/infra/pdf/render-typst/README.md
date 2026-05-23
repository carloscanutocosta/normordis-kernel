# render-typst

Adapter de infraestrutura para rendering Typst/PDF no Mini-Kernel RS.

## Responsabilidade

- Compilar fonte Typst para PDF quando a feature `pdf` esta ativa.
- Gerir `WarmFonts` para reutilizacao eficiente de fontes no host.
- Servir como backend concreto para pipelines/documentos que escolhem Typst.

## Nao responsabilidade

- Nao define semantica documental.
- Nao decide autorizacao, arquivo, assinatura ou auditoria.
- Nao substitui dominios como `documentos-pdf`.
- Nao fornece helpers transversais de template; isso pertence a
  `support-typst-template`.

## Exemplo minimo

```rust
let source = "#set page(paper: \"a4\")\nOlá, Typst.";
let pdf = render_typst::pdf::compile_pdf(source, None)?;
assert!(!pdf.is_empty());
# Ok::<(), render_typst::pdf::PdfError>(())
```

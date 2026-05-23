# documentos-pdf

Renderização de documentos PDF a partir de templates Typst e formato NDT/NDF — facade sobre `support-pdf` e `normordis-pdf`.

## Objectivo

Centraliza os pontos de entrada para geração de PDFs documentais: renderização Typst com cache de fontes (`WarmFonts`), compilação de documentos NDT e renderização de NDF para visualização e assinatura.

## Posição arquitectural

`crates/kernel/infra/pdf` — adaptador de infraestrutura PDF. Depende de `support-pdf` (trait `PdfRenderer`) e `normordis-pdf` (engine NDT/NDF). As funcionalidades são activadas por feature flags.

## Responsabilidade

- Renderizar strings Typst em PDF (`render_typst_document`).
- Gerir cache de fontes Typst em memória (`WarmFonts`) para performance.
- Compilar e renderizar documentos NDT (`compile_ndt`, `render_ndf`, `render_ndf_for_signing`).
- Re-exportar `PdfError` de `support-pdf` como tipo de erro unificado.

## Não-responsabilidade

- Não faz gestão de fila de jobs — use `pdf-pipeline` para isso.
- Não guarda PDFs em disco nem em base de dados.
- Não converte DOCX — use `support-docx-to-typst`.

## Features

| Feature | Activa |
|---|---|
| `typst` | `render_typst_document`, `WarmFonts` |
| `normordis` | `compile_ndt`, `render_ndf`, `render_ndf_for_signing`, `render_ndt` |

## Exemplo mínimo

```rust
// Com feature "typst"
use documentos_pdf::{render_typst_document, WarmFonts};

let fonts = WarmFonts::load()?;
let pdf_bytes = render_typst_document("#set text(lang: \"pt\")\nOlá mundo", &fonts)?;
```

## Validação

```sh
cargo test -p documentos-pdf
```

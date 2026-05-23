# normaxis-pdf

Pure-Rust institutional PDF generation for the NORMAXIS mini-app framework.

[![Crate](https://img.shields.io/badge/crate-normaxis--pdf-blue)](https://crates.io/crates/normaxis-pdf)
[![License: EUPL-1.2](https://img.shields.io/badge/license-EUPL--1.2-blue.svg)](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)

## Overview

`normaxis-pdf` generates formal documents — reports, letters, certificates, forms — directly from Rust, with no external binary dependency (no LaTeX, no Typst, no Chromium). It targets Portuguese public administration document standards and embeds [Liberation Sans](https://github.com/liberationfonts/liberation-fonts) for out-of-the-box rendering with real glyph metrics.

Three composition models, all mixable in a single document:

| Model | Description |
|---|---|
| **Flow** | Elements stack vertically; automatic page breaks; header re-injection |
| **Fixed Box** | Elements placed at absolute coordinates; no cursor effect |
| **NDT Templates** | JSON-driven document templates with runtime data injection |

## Quick Start

```toml
# Cargo.toml
[dependencies]
normaxis-pdf = "1.0.0"
```

### Flow document

```rust
use normaxis_pdf::{DocumentBuilder, Paragraph, Section, Spacer, TextAlign};

let pdf = DocumentBuilder::new("Relatório Mensal")
    .push(Section::new("1. Introdução", 1))
    .push(Paragraph::new("Este relatório descreve…").align(TextAlign::Justify))
    .push(Spacer::new(6.0))
    .render_to_bytes()?;

std::fs::write("output.pdf", pdf)?;
```

### NCRTF rich text

```rust
use normaxis_pdf::DocumentBuilder;

let ncrtf = r#"{
  "ncrtf": "1.0",
  "blocks": [
    {"type":"heading","level":1,"children":[{"type":"text","text":"Título","marks":[]}]},
    {"type":"paragraph","alignment":"justify","children":[
      {"type":"text","text":"Texto com ","marks":[]},
      {"type":"text","text":"negrito","marks":["bold"]},
      {"type":"text","text":" e itálico.","marks":["italic"]}
    ]}
  ]
}"#;

let pdf = DocumentBuilder::new("Documento")
    .push_ncrtf(ncrtf)?
    .render_to_bytes()?;
```

### NDT template

```rust
use normaxis_pdf::DocumentBuilder;

const TEMPLATE: &str = include_str!("templates/oficio-nacional.ndt.json");

let data = serde_json::json!({
    "ndt_data": "1.0.0",
    "data": {
        "entidade": "Câmara Municipal de Exemplo",
        "numero": "2025/001",
        "data": "25 de Abril de 2025",
        "assunto": "Resposta a pedido de informação",
        "mensagem": "{\"ncrtf\":\"1.0\",\"blocks\":[{\"type\":\"paragraph\",\"alignment\":\"justify\",\"children\":[{\"type\":\"text\",\"text\":\"Informamos que o pedido foi recebido.\",\"marks\":[]}]}]}"
    }
}).to_string();

let pdf = DocumentBuilder::new("Ofício")
    .push_ndt(TEMPLATE, &data)?
    .render_to_bytes()?;
```

## Features

### Flow elements

| Type | Struct / method |
|---|---|
| Paragraph (plain or rich) | `Paragraph::new(text)` |
| Section heading | `Section::new(text, level)` |
| Ordered / bullet / checklist | `List::new(items, ListType::Bullet)` |
| Table | `Table::new(headers, rows)` |
| Image | `FlowImage::new(bytes, width_mm)` |
| Spacer | `Spacer::new(height_mm)` |
| Horizontal rule | `HorizontalRule::new()` |
| Page break | `PageBreak` |

### Fixed Box elements

| Type | Builder method |
|---|---|
| Text at absolute position | `DocumentBuilder::fixed_text(box, text, align)` |
| Image at absolute position | `DocumentBuilder::fixed_image(box, bytes, fit)` |
| Decorative line | `DocumentBuilder::fixed_line(x1, y1, x2, y2, color)` |

### NCRTF v1.0 — rich text format

NCRTF (NORMAXIS Canonical Rich Text Format) is a JSON schema for inline-styled paragraphs. It is the interchange format between editors (such as `@normaxis/nx-doc`) and this renderer.

```json
{
  "ncrtf": "1.0",
  "blocks": [
    {
      "type": "paragraph",
      "alignment": "justify",
      "children": [
        {"type": "text", "text": "Normal, ", "marks": []},
        {"type": "text", "text": "bold", "marks": ["bold"]},
        {"type": "text", "text": " and italic.", "marks": ["italic"]}
      ]
    },
    {
      "type": "list",
      "list_type": "bullet",
      "children": [
        {"indent": 0, "children": [{"type": "text", "text": "Item", "marks": []}]}
      ]
    }
  ]
}
```

Supported block types: `paragraph`, `heading` (levels 1–4), `list` (bullet / ordered / checklist).  
Supported inline marks: `bold`, `italic`, `underline`, `strikethrough`, `code`.

### NDT v1.0.0 — document templates

NDT (NORMAXIS Document Template) is a JSON-driven template format for institutional documents. Templates define a layout schema; runtime data is injected at render time.

**Template file** (`*.ndt.json`):

```json
{
  "ndt": "1.0.0",
  "meta": { "title": "Relatório", "description": "…" },
  "placeholders": {
    "entity_name": { "type": "string", "required": true },
    "body":        { "type": "ncrtf",  "required": false }
  },
  "body": [
    { "type": "heading", "text": "{{entity_name}}", "level": 1 },
    {
      "type": "conditional",
      "condition": "body", "operator": "exists",
      "then": [{ "type": "rich_text", "content": "{{body}}", "source": "placeholder" }],
      "else": [{ "type": "paragraph", "text": "Sem conteúdo." }]
    }
  ]
}
```

**Data file** (`*.ndt-data.json`):

```json
{
  "ndt_data": "1.0.0",
  "data": {
    "entity_name": "Câmara Municipal de Exemplo",
    "body": "{\"ncrtf\":\"1.0\",\"blocks\":[]}"
  }
}
```

Supported body element types: `paragraph`, `heading`, `rich_text`, `table`, `list`, `image`,
`spacer`, `horizontal_rule`, `page_break`, `fixed_text`, `fixed_image`, `fixed_line`,
`fixed_box`, `zone_ref`, `conditional`, `repeat`, `include`.

Supported conditional operators: `exists`, `empty`, `eq`, `neq`, `gt`, `lt`.

## Bundled templates

Ready-to-use NDT templates are provided under `examples/templates/`:

| File | Description |
|---|---|
| `relatorio-simples.ndt.json` | Simple institutional report |
| `oficio-nacional.ndt.json` | Official letter (ofício) |
| `certidao-generica.ndt.json` | Generic certificate (certidão) |
| `formulario-generico.ndt.json` | Generic two-section form |

## Examples

Run any example with:

```bash
cargo run --example <name> -p normaxis-pdf
```

| Example | Description |
|---|---|
| `01_basic_document` | Flow document with headings, paragraphs, table, list |
| `02_ncrtf_document` | Document built from NCRTF rich text JSON |
| `03_ndt_template` | Document rendered from an NDT template + runtime data |
| `04_mixed_layout` | Flow + Fixed Box mixed (office letter style) |

## Fonts

Liberation Sans (Regular, Bold, Italic, Bold Italic) is embedded at compile time via `include_bytes!`. No system fonts are required. The `FontRegistry` type allows registering additional TTF/OTF families at runtime.

## Version constants

```rust
normaxis_pdf::VERSION        // "1.0.0" — crate version
normaxis_pdf::NDT_VERSION    // "1.0.0" — NDT engine version
normaxis_pdf::NCRTF_VERSION  // "1.0"   — NCRTF parser version
```

## API stability

All public items re-exported from `normaxis_pdf::*` are considered stable from v1.0.0 onwards. Internal modules (`normaxis_pdf::template::*`, `normaxis_pdf::richtext::*`, etc.) are not stable and may change between minor versions.

## License

EUPL-1.2 — see [LICENSE](../../LICENSE) or [https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12](https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12).

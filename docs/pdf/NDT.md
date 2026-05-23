# NDT — NORMAXIS Document Template
### Manual do Developer · v2.0.0

> **Versão actual:** NDT 2.0.0 · normaxis-pdf 2.0.0  
> **Formatos aceites:** JSON (`.ndt.json`) e TOML (`.ndt.toml`) — auto-detectados pelo engine  
> **Retro-compatibilidade:** documentos NDT 1.0.0–1.x são aceites sem alterações

---

## O que é o NDT

O NDT é o formato nativo de templates de documentos do NORMAXIS. Define num único ficheiro:

- A **estrutura** do documento (elementos, secções, tabelas)
- Os **estilos** tipográficos (fontes, tamanhos, espaçamentos)
- Os **metadados** (título, entidade, língua)
- As **opções de output** (PDF/A, compressão, classificação de segurança)
- Os **placeholders** `{{campo}}` que são preenchidos com dados reais em runtime

Um ficheiro NDT é um **template** — não um documento acabado. O mesmo template pode ser usado para gerar centenas de documentos com dados diferentes.

**Relação com o NDF:** o NDT é o artefacto de autoria; o NDF (NORMAXIS Document Format) é o artefacto documental gerado. O ciclo canónico é:

```
NDT + NdtData  →  compile_ndt()  →  NDF  →  render_ndf()  →  PDF
```

O NDF preserva o conteúdo resolvido, os estilos, a rastreabilidade e a cadeia de custódia. Ver `NDF.md` para a especificação completa.

---

## JSON vs TOML

O engine detecta o formato automaticamente pelo primeiro carácter não-whitespace:
`{` → JSON, qualquer outro → TOML.

**Use JSON quando:**
- Gerar o NDT programaticamente (Rust, TypeScript, API)
- Integrar via API ou validar por JSON Schema
- O conteúdo tem tabelas aninhadas ou estruturas complexas

**Use TOML quando:**
- A edição humana e os comentários inline forem prioritários
- A secção `[styles]` é o foco principal da edição
- Escrever templates à mão

> Tanto JSON como TOML são versionáveis em Git. O critério de escolha é a origem e o uso do ficheiro, não o sistema de versionamento.

> ⚠️ **Não usar YAML como formato NDT canónico.** A crate `serde_yaml` encontra-se descontinuada e não mantida, criando risco de dependência técnica no ecossistema Rust.

---

## Estrutura raiz

```json
{
  "ndt":       "2.0.0",
  "meta":      { ... },
  "output":    { ... },
  "styles":    { ... },
  "footnotes": [ ... ],
  "signature": { ... },
  "content":   [ ... ]
}
```

Equivalente em TOML:

```toml
ndt = "2.0.0"

[meta]
# ...

[output]
# ...

[styles.nome_do_estilo]
# ...

[[footnotes]]
# ...

[signature]
# ...

[[content]]
# ...
```

---

## meta

Metadados do documento. `title` e `entity` são obrigatórios.

```json
"meta": {
  "title":       "Ofício n.º {{referencia}}",
  "entity":      "Câmara Municipal de Lisboa",
  "lang":        "pt-PT",
  "compat_mode": 15
}
```

| Campo | Tipo | | Descrição |
|---|---|---|---|
| `title` | string | **req** | Título do documento. Aceita `{{placeholders}}`. |
| `entity` | string | **req** | Nome da entidade emissora. |
| `lang` | string | opt | Código BCP 47. Default: `"pt-PT"`. |
| `compat_mode` | u32 | opt | Compat mode Word da origem (12=2007, 15=2013, 16=2016+). Extraído pelo `dotx2ndt`. |

---

## output

Opções de output do PDF. Todos os campos são opcionais.

```json
"output": {
  "standard":       "pdf_a_1b",
  "compression":    "best",
  "classification": "interno",
  "document_ref":   "ACT/2026/001"
}
```

| Campo | Valores | Default | Descrição |
|---|---|---|---|
| `standard` | `"pdf_1_7"` `"pdf_a_1b"` `"pdf_a_2b"` | `"pdf_1_7"` | Conformidade do PDF. `pdf_a_1b` para arquivo de longa duração. |
| `compression` | `"none"` `"fast"` `"default"` `"best"` | `"default"` | Nível de compressão zlib dos streams. |
| `classification` | `"public"` `"internal"` `"confidential"` `"reserved"` | `"public"` | Classificação de segurança. Níveis > public adicionam marca de água automática. |
| `document_ref` | string | — | Referência para rastreabilidade CRA/NIS2. |

---

## styles

Estilos nomeados reutilizáveis. Equivalente aos Paragraph Styles do Word.

```json
"styles": {
  "corpo_formal": {
    "extends":              "normal",
    "font_family":          "LiberationSerif",
    "font_size":            11.0,
    "bold":                 false,
    "italic":               false,
    "color":                "#1A1A1A",
    "alignment":            "justify",
    "line_height":          1.4,
    "space_before_mm":      0.0,
    "space_after_mm":       2.0,
    "indent_left_mm":       0.0,
    "indent_right_mm":      0.0,
    "indent_first_line_mm": 0.0,
    "line_breaking":        "knuth_plass",
    "opentype_features": {
      "kern": true,
      "liga": true,
      "tnum": false
    }
  }
}
```

Em TOML (mais legível para edição):

```toml
[styles.corpo_formal]
extends      = "normal"
font_family  = "LiberationSerif"
font_size    = 11.0
alignment    = "justify"
line_height  = 1.4
space_after_mm = 2.0
line_breaking  = "knuth_plass"

[styles.corpo_formal.opentype_features]
kern = true
liga = true
```

### Campos de NamedStyle

| Campo | Tipo | Descrição |
|---|---|---|
| `extends` | string | Nome do estilo base. Herança em cadeia. Ciclos são detectados como erro. |
| `font_family` | string | `"LiberationSans"` `"LiberationSerif"` `"LiberationMono"` ou alias Word (ex: `"Arial"`). |
| `font_size` | f64 | Tamanho em pontos tipográficos. |
| `bold` | bool | Negrito. |
| `italic` | bool | Itálico. |
| `color` | string | Cor hex `"#RRGGBB"`. |
| `alignment` | enum | `"left"` `"center"` `"right"` `"justify"`. |
| `line_height` | f64 | Múltiplo da altura de linha. Ex: `1.4`. |
| `space_before_mm` | f64 | Espaço antes do parágrafo em mm. Suprimido no topo de página. |
| `space_after_mm` | f64 | Espaço após o parágrafo em mm. |
| `indent_left_mm` | f64 | Indentação esquerda em mm. |
| `indent_right_mm` | f64 | Indentação direita em mm. |
| `indent_first_line_mm` | f64 | Indentação da primeira linha em mm. Negativo = hanging indent. |
| `line_breaking` | enum | `"greedy"` (default) ou `"knuth_plass"` (feature `optimal_wrap`). |
| `opentype_features` | objeto | Ver secção OpenType features. |
| `tab_stops` | array | Lista de tab stops. Ver secção paragraph. |

### Estilos pré-definidos

O engine inclui estes estilos sem necessidade de os declarar no NDT:

| Nome | Font | Size | Align | Uso típico |
|---|---|---|---|---|
| `normal` | LiberationSans | 11pt | justify | Corpo de texto base |
| `heading_1` | LiberationSans Bold | 16pt | left | Título principal (azul #003399) |
| `heading_2` | LiberationSans Bold | 13pt | left | Subtítulo |
| `heading_3` | LiberationSans Bold Italic | 11pt | left | Subsecção |
| `caption` | LiberationSans Italic | 9pt | center | Legendas |
| `table_header` | LiberationSans Bold | 11pt | center | Cabeçalho de tabela |
| `table_body` | LiberationSans | 11pt | left | Corpo de tabela |
| `footnote` | LiberationSans | 9pt | left | Notas de rodapé |
| `toc_1` | LiberationSans Bold | 11pt | left | Índice nível 1 |
| `toc_2` | LiberationSans | 11pt | left | Índice nível 2 (+8mm indent) |
| `toc_3` | LiberationSans | 10pt | left | Índice nível 3 (+16mm indent) |

### Fontes embebidas e aliases Word

| Nome NORMAXIS | Aliases Word resolvidos |
|---|---|
| `LiberationSans` | Arial, Calibri, Helvetica |
| `LiberationSerif` | Times New Roman, Cambria, Georgia |
| `LiberationMono` | Courier New, Consolas |

---

## content

Array de elementos que compõem o documento. Cada elemento tem um campo `"type"` obrigatório.

### paragraph

```json
{
  "type":              "paragraph",
  "style":             "corpo",
  "text":              "Texto simples.",
  "alignment":         "justify",
  "indent_left":       0.0,
  "indent_right":      0.0,
  "indent_first_line": 0.0,
  "space_before":      0.0,
  "space_after":       2.0,
  "keep_with_next":    false,
  "keep_lines":        false,
  "border": {
    "top":    null,
    "bottom": null,
    "left":   { "thickness_mm": 1.5, "color": "#003399" },
    "right":  null,
    "padding_mm": 4.0
  },
  "background": "#F0F4FF",
  "children":   [ ]
}
```

> **`text` vs `children`:** use `"text"` para texto simples (sem formatação mista). Use `"children"` com nós NCRTF inline para texto com bold, italic, links, cores, etc.

**Tab stops:**

```json
{
  "type": "paragraph",
  "text": "Subtotal\t123,45 €",
  "tab_stops": [
    { "position_mm": 150.0, "alignment": "right", "leader": "." }
  ]
}
```

| Campo `tab_stop` | Valores | Descrição |
|---|---|---|
| `position_mm` | f64 | Posição em mm a partir da margem esquerda. |
| `alignment` | `"left"` `"right"` `"center"` `"decimal"` | Tipo de tab stop. |
| `leader` | char | Carácter de preenchimento. Ex: `"."` para pontos líderes. Default: `" "`. |

---

### heading

```json
{
  "type":  "heading",
  "level": 1,
  "text":  "1. Introdução",
  "style": "titulo_secao"
}
```

`level` de 1 a 6. O engine usa automaticamente `heading_N` como estilo base se `style` for omitido.

---

### table

```json
{
  "type":        "table",
  "table_style": "table_striped",
  "rows": [
    {
      "is_header": true,
      "height": { "exact": 8.0 },
      "cells": [
        {
          "content":        "Nome",
          "style":          "table_header",
          "col_span":       1,
          "row_span":       1,
          "alignment":      "left",
          "vertical_align": "middle",
          "padding": { "top": 1.0, "bottom": 1.0, "left": 2.0, "right": 2.0 },
          "borders": {
            "bottom": { "thickness_mm": 0.5, "color": "#003399" }
          }
        }
      ]
    },
    {
      "is_header": false,
      "cells": [
        { "content": "João Silva" }
      ]
    }
  ]
}
```

**Estilos de tabela pré-definidos:**

| `table_style` | Descrição |
|---|---|
| `"table_grid"` | Todas as bordas, sem fundo (default) |
| `"table_bordered"` | Só borda exterior |
| `"table_striped"` | Grid + linhas alternadas (#F5F5F5) |
| `"table_plain"` | Sem bordas, sem fundo |

**`height` de linha:**

```json
{ "auto": null }          // altura determinada pelo conteúdo (default)
{ "at_least": 8.0 }       // mínimo 8mm, pode crescer
{ "exact": 8.0 }          // exactamente 8mm (w:hRule="exact")
```

**Tabela com col_span:**

```json
{
  "type": "table",
  "rows": [
    {
      "is_header": true,
      "cells": [
        { "content": "Identificação", "col_span": 2 },
        { "content": "Contacto",      "col_span": 2 }
      ]
    },
    {
      "cells": [
        { "content": "Nome" },
        { "content": "NIF" },
        { "content": "Email" },
        { "content": "Tel." }
      ]
    }
  ]
}
```

**Tabela aninhada numa célula:**

```json
{
  "content_type": "table",
  "table": {
    "rows": [
      { "cells": [{ "content": "Email" }, { "content": "geral@cm.pt" }] },
      { "cells": [{ "content": "Tel." },  { "content": "21 000 0000" }] }
    ]
  }
}
```

---

### list

```json
{
  "type":      "list",
  "list_type": "bullet",
  "items": [
    { "text": "Primeiro item" },
    { "text": "Segundo item com itálico", "indent": 0 },
    { "text": "Item aninhado",            "indent": 1 }
  ]
}
```

`list_type`: `"bullet"` | `"ordered"` | `"checklist"`

Para checklist:

```json
{
  "type":      "list",
  "list_type": "checklist",
  "items": [
    { "text": "Tarefa concluída", "checked": true  },
    { "text": "Tarefa pendente",  "checked": false }
  ]
}
```

---

### image

```json
{
  "type":          "image",
  "src":           "data:image/png;base64,iVBOR...",
  "alt":           "Logótipo da Câmara Municipal",
  "caption":       "Fig. 1 — Organograma 2026",
  "alignment":     "center",
  "width_mm":      120.0,
  "height_mm":     null
}
```

`src` aceita:
- Data URI base64: `"data:image/png;base64,..."`
- Referência a asset: `"asset:logo"` (a mini-app resolve o asset)

`height_mm: null` calcula proporcionalmente a partir de `width_mm`.

> ⚠️ O campo `alt` é obrigatório para conformidade PDF/UA (v2.1.0+).

---

### toc (índice automático)

Requer two-pass rendering. Colocar no início do documento, antes do primeiro `heading`.

```json
{
  "type":        "toc",
  "title":       "Índice",
  "max_level":   3,
  "leader_char": "."
}
```

---

### spacer

```json
{ "type": "spacer", "height_mm": 6.0 }
```

---

### page_break

```json
{ "type": "page_break" }
```

---

### horizontal_rule

```json
{ "type": "horizontal_rule" }
```

---

### section_break

Muda a orientação e/ou margens para as páginas seguintes.

```json
{
  "type":        "section_break",
  "orientation": "landscape",
  "margins": {
    "top_mm":    15.0,
    "bottom_mm": 15.0,
    "left_mm":   20.0,
    "right_mm":  20.0
  }
}
```

`orientation`: `"portrait"` | `"landscape"`. `margins` é opcional — herda `DocumentStyle` se omitido.

---

### footnote_ref

Usado **dentro de `children[]`** de um `paragraph` para referenciar uma nota de rodapé.

```json
{
  "type": "paragraph",
  "children": [
    { "type": "text", "text": "Conforme legislação vigente" },
    { "type": "footnote_ref", "number": 1 },
    { "type": "text", "text": "." }
  ]
}
```

As notas são definidas na secção `footnotes` do documento raiz:

```json
"footnotes": [
  {
    "number": 1,
    "content": [
      {
        "type":  "paragraph",
        "style": "footnote",
        "text":  "Diário da República n.º 42, de 1 de Março de 2026."
      }
    ]
  }
]
```

---

### acroform_field

Campos de formulário PDF interactivos. Posicionados em coordenadas absolutas.

```json
{
  "type":       "acroform_field",
  "field_type": "text_field",
  "name":       "nome_requerente",
  "tooltip":    "Nome completo do requerente",
  "required":   true,
  "max_length": 100,
  "font_size":  11.0,
  "rect": { "x_mm": 25.0, "y_mm": 240.0, "width_mm": 120.0, "height_mm": 8.0 }
}
```

`field_type`: `"text_field"` | `"check_box"` | `"radio_button"` | `"combo_box"` | `"list_box"`

Para `combo_box` e `list_box`, adicionar `"options": ["Opção A", "Opção B"]`.

> **AcroForm em PDF/A:** campos interactivos AcroForm são permitidos em PDF/A desde que tenham aparência visual fixa associada. XFA (XML Forms Architecture) é proibido em PDF/A. Para arquivo institucional, preferir PDF/A-2b ou superior e evitar XFA. O normaxis-pdf gera AcroForm standard — nunca XFA.

---

## Placeholders

Substituídos por dados do `NdtData` antes do rendering.

```
Sintaxe: {{nome_do_campo}}
```

```json
"text": "Exmo. Sr. {{destinatario}}, conforme ofício {{referencia}}..."
```

```rust
// Em Rust:
let data = NdtData::from([
    ("destinatario", "João Silva"),
    ("referencia",   "REF/2026/001"),
]);
let pdf = render_ndt(template_str, &data)?;
```

> ⚠️ Usar sempre `{{duplas}}`. As chavetas simples `{campo}` colidem com a sintaxe TOML.

---

## Campos calculados

Resolvidos em runtime durante o rendering.

| Campo | Resolve para | Contexto |
|---|---|---|
| `{{page}}` | Número da página actual | footer, header, qualquer texto |
| `{{total_pages}}` | Total de páginas (two-pass) | footer, header, qualquer texto |
| `{{today}}` | "25 de Abril de 2026" | qualquer texto |
| `{{now}}` | "25/04/2026 14:32" | qualquer texto |

---

## OpenType features

```json
"opentype_features": {
  "kern": true,
  "liga": true,
  "tnum": false,
  "smcp": false,
  "sups": false,
  "subs": false
}
```

| Feature | Tag | Default | Quando usar |
|---|---|---|---|
| `kern` | `kern` | `true` | Kerning via GPOS. Sempre activo. |
| `liga` | `liga` | `true` | Ligaduras: fi, fl, ff, ffi, ffl |
| `tnum` | `tnum` | `false` | Números tabelares. **Usar em colunas monetárias.** |
| `smcp` | `smcp` | `false` | Small caps. Fallback sintético a 80% se a fonte não suportar. |
| `sups` | `sups` | `false` | Superscript via glifos OpenType. |
| `subs` | `subs` | `false` | Subscript via glifos OpenType. |

---

## Assinatura digital

```json
"signature": {
  "field": {
    "x_mm":      120.0,
    "y_mm":      40.0,
    "width_mm":  70.0,
    "height_mm": 20.0,
    "page":      1,
    "label":     "Assinatura do Presidente"
  },
  "reason":   "Aprovado em reunião de câmara",
  "location": "Lisboa"
}
```

O certificado e a chave privada são passados via `SignatureConfig` em Rust — não são incluídos no ficheiro NDT por razões de segurança.

> A assinatura digital deve ser tratada como perfil PAdES (ETSI EN 319 102) sobre PDF. Para preservação com assinatura, preferir PDF/A-2b ou superior quando aplicável — o PAdES não está conceptualmente ligado a um perfil PDF/A específico. Requer a feature `signing` activa no `Cargo.toml`.

---

## Exemplo completo — Ofício

```json
{
  "ndt": "2.0.0",

  "meta": {
    "title":  "Ofício n.º {{referencia}}",
    "entity": "Câmara Municipal de Lisboa",
    "lang":   "pt-PT"
  },

  "output": {
    "standard":     "pdf_a_1b",
    "compression":  "best",
    "document_ref": "{{referencia}}"
  },

  "styles": {
    "data_oficio": {
      "extends":   "normal",
      "alignment": "right",
      "font_size": 10.0,
      "italic":    true
    },
    "corpo_oficio": {
      "extends":       "normal",
      "line_breaking": "knuth_plass"
    }
  },

  "content": [
    {
      "type":  "paragraph",
      "style": "data_oficio",
      "text":  "Lisboa, {{data_hoje}}"
    },
    {
      "type":  "heading",
      "level": 1,
      "text":  "Assunto: {{assunto}}"
    },
    {
      "type":  "paragraph",
      "style": "corpo_oficio",
      "text":  "Exmo(a). Sr(a). {{destinatario}},"
    },
    {
      "type":  "paragraph",
      "style": "corpo_oficio",
      "text":  "{{corpo_texto}}"
    },
    { "type": "spacer", "height_mm": 4.0 },
    {
      "type":  "paragraph",
      "style": "normal",
      "text":  "Com os melhores cumprimentos,"
    },
    { "type": "spacer", "height_mm": 16.0 },
    {
      "type":  "paragraph",
      "style": "normal",
      "text":  "{{nome_signatario}}"
    },
    {
      "type":  "paragraph",
      "style": "caption",
      "text":  "{{cargo_signatario}}"
    }
  ]
}
```

Equivalente em TOML:

```toml
ndt = "2.0.0"

[meta]
title  = "Ofício n.º {{referencia}}"
entity = "Câmara Municipal de Lisboa"
lang   = "pt-PT"

[output]
standard     = "pdf_a_1b"
compression  = "best"
document_ref = "{{referencia}}"

[styles.data_oficio]
extends   = "normal"
alignment = "right"
font_size = 10.0
italic    = true

[styles.corpo_oficio]
extends       = "normal"
line_breaking = "knuth_plass"

[[content]]
type  = "paragraph"
style = "data_oficio"
text  = "Lisboa, {{data_hoje}}"

[[content]]
type  = "heading"
level = 1
text  = "Assunto: {{assunto}}"

[[content]]
type  = "paragraph"
style = "corpo_oficio"
text  = "Exmo(a). Sr(a). {{destinatario}},"

[[content]]
type  = "paragraph"
style = "corpo_oficio"
text  = "{{corpo_texto}}"

[[content]]
type      = "spacer"
height_mm = 4.0

[[content]]
type  = "paragraph"
style = "normal"
text  = "Com os melhores cumprimentos,"

[[content]]
type      = "spacer"
height_mm = 16.0

[[content]]
type  = "paragraph"
style = "normal"
text  = "{{nome_signatario}}"

[[content]]
type  = "paragraph"
style = "caption"
text  = "{{cargo_signatario}}"
```

---

## Exemplo completo — Acta com tabela e notas de rodapé

```json
{
  "ndt": "2.0.0",

  "meta": {
    "title":  "Acta n.º {{numero}} da Reunião de {{data_reuniao}}",
    "entity": "Câmara Municipal de Lisboa"
  },

  "output": {
    "standard":       "pdf_a_1b",
    "classification": "internal",
    "document_ref":   "ACT/{{ano}}/{{numero}}"
  },

  "footnotes": [
    {
      "number": 1,
      "content": [
        { "type": "paragraph", "style": "footnote",
          "text": "Aprovado nos termos do art.º 42.º do Regime Jurídico das Autarquias Locais." }
      ]
    }
  ],

  "content": [
    { "type": "toc", "title": "Índice", "max_level": 2 },
    { "type": "spacer", "height_mm": 8.0 },

    { "type": "heading", "level": 1, "text": "1. Presenças" },
    {
      "type": "table",
      "table_style": "table_grid",
      "rows": [
        {
          "is_header": true,
          "cells": [
            { "content": "Nome",     "style": "table_header", "col_span": 1 },
            { "content": "Cargo",    "style": "table_header" },
            { "content": "Presença", "style": "table_header", "alignment": "center" }
          ]
        },
        {
          "cells": [
            { "content": "{{membro_1_nome}}" },
            { "content": "{{membro_1_cargo}}" },
            { "content": "✓", "alignment": "center" }
          ]
        }
      ]
    },

    { "type": "heading", "level": 1, "text": "2. Deliberações" },
    {
      "type": "paragraph",
      "style": "normal",
      "children": [
        { "type": "text", "text": "A câmara deliberou, por unanimidade" },
        { "type": "footnote_ref", "number": 1 },
        { "type": "text", "text": ", aprovar o orçamento municipal para {{ano_orcamento}}." }
      ]
    }
  ]
}
```

---

## Versões NDT

| Versão | Principais adições |
|---|---|
| 1.0.0 | Scaffold base — paragraph, heading, table, image, spacer, header, footer |
| 1.1.0 | Watermark, page_sections, campos calculados, RowHeight exact |
| 1.2.0 | col_span/row_span, z-index, letter_spacing, bordas por célula, indent, TextAlign Right |
| 1.3.0 | `styles` (estilos nomeados), tab_stops, cell padding, dual-format TOML+JSON |
| 1.4.0 | opentype_features, line_breaking, decorações de texto, section_break |
| 1.5.0 | footnotes, toc, acroform_field, tabelas aninhadas, LiberationSerif/Mono |
| **2.0.0** | output.standard (PDF/A), classification, signature, rastreabilidade CRA/NIS2 |

Todos os documentos NDT 1.0.0–1.x são aceites pelo engine 2.0.0 sem alterações.

---

## Erros comuns

**`StyleCycleError("nome_estilo")`**  
O campo `extends` cria um ciclo: A → B → A. Verificar a cadeia de herança.

**`UnknownStyle("nome")`**  
O `style` referenciado não existe nos estilos pré-definidos nem na secção `styles`. Verificar o nome.

**`NDT TOML parse error`**  
Erro de sintaxe TOML. Causas comuns: `{{placeholders}}` com caracteres especiais, `=` em vez de `:` em JSON, ou aspas mal fechadas.

**`NDT JSON parse error`**  
Erro de sintaxe JSON. Usar um validator JSON (ex: `jq .` em terminal).

**PDF com marca de água não esperada**  
`output.classification` está definido com valor diferente de `"public"`. Níveis `internal`, `confidential` e `reserved` adicionam marca de água automática.

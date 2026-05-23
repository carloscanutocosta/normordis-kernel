# NCRTF — NORMAXIS Canonical Rich Text Format
### Manual do Developer · v1.3.0

> **Versão actual:** NCRTF 1.3.0 · normaxis-pdf 2.0.0  
> **Formato:** JSON exclusivamente  
> **Contexto de uso:** conteúdo de campos rich text em mini-apps com editor Lexical

---

## O que é o NCRTF

O NCRTF é o formato intermédio entre o editor React (Lexical) e o backend Rust do NORMAXIS. Quando o utilizador edita texto rico numa mini-app, o Lexical serializa o estado do editor para NCRTF JSON. O backend deserializa e renderiza para PDF.

**Escopo e limites.** O NCRTF é conteúdo rico embebível — não define documento completo, metadados jurídicos, numeração, integridade nem cadeia de custódia. Esses pertencem ao NDT (template) e ao NDF (documento gerado). O NCRTF é conteúdo *dentro* de um documento, não o documento em si.

**O NCRTF não é o mesmo que o NDT.** O NDT define um template completo de documento. O NCRTF define o conteúdo de um campo rich text *dentro* de um NDT — por exemplo, o corpo de um ofício editável pelo utilizador.

```
Mini-app frontend                  Backend Rust
────────────────                   ────────────
Lexical editor
  → serializa estado
  → NCRTF JSON string   ─────────► parse_ncrtf()
                                    → NcrtfDocument
                                    → ncrtf_to_elements()
                                    → Vec<Box<dyn Element>>
                                    → render_to_bytes()
                                    → Vec<u8> (PDF)
```

---

## Estrutura raiz

```json
{
  "ncrtf": "1.3.0",
  "meta": {
    "title":      "Título opcional",
    "lang":       "pt",
    "author":     "Nome do autor",
    "created_at": "2026-04-29T10:00:00Z",
    "updated_at": "2026-04-29T10:00:00Z",
    "custom": {
      "reference": "REF/2026/001"
    }
  },
  "blocks": [ ...Block[] ]
}
```

### Sobre o campo meta

O `meta` do NCRTF é auxiliar e editorial — serve o editor e o fluxo de autoria, não o arquivo documental. Não substitui o `meta` do NDT (que define o template) nem o `meta` do NDF (que é imutável e tem valor jurídico-arquivístico). Campos como título, entidade, numeração ou classificação de segurança pertencem sempre ao NDT/NDF.

> **JSON e canonicalização:** o NCRTF usa JSON standard na sua função primária de transportar rich text editável entre frontend e backend. Não requer JSON canónico (RFC 8785 / JCS) neste contexto. Quando um NCRTF é incorporado num NDF como parte do campo `content`, o NDF aplica a canonicalização ao payload completo — o NCRTF é canonicalizado *como parte do NDF*, não independentemente.

---

## Modelo de dados

```
NcrtfDocument
└── blocks: Block[]
    ├── paragraph    → children: Inline[]
    ├── heading      → children: Inline[], level: 1-6
    ├── list         → children: ListItem[]
    ├── table        → head: TableRow[], body: TableRow[]
    ├── blockquote   → children: Inline[]
    ├── code_block   → code: string
    ├── image        → src, alt, caption
    ├── horizontal_rule
    └── page_break

Inline
    ├── text         → text: string, marks: Mark[]
    ├── link         → href, children: Inline[]
    ├── hard_break
    └── footnote_ref → number: u32  (v1.3.0+)

Mark
    ├── "bold"
    ├── "italic"
    ├── "underline"
    ├── "strikethrough"
    ├── "superscript"
    ├── "subscript"
    ├── "code"
    ├── "small_caps"
    ├── { "type": "color",      "value": "#CC0000" }
    ├── { "type": "highlight",  "value": "yellow"  }
    └── { "type": "font_size",  "value": 14.0      }
```

---

## Blocos

### paragraph

O bloco mais comum. Contém uma sequência de nós inline.

```json
{
  "type":      "paragraph",
  "alignment": "justify",
  "indent":    0,
  "style":     "corpo",
  "children": [
    { "type": "text", "text": "Texto normal. ", "marks": [] },
    { "type": "text", "text": "Negrito. ",       "marks": ["bold"] },
    { "type": "text", "text": "Itálico.",         "marks": ["italic"] }
  ]
}
```

| Campo | Tipo | | Descrição |
|---|---|---|---|
| `alignment` | enum | opt | `"left"` `"center"` `"right"` `"justify"`. Default: `"left"`. |
| `indent` | u8 | opt | Nível de indentação 0–6. Default: `0`. |
| `style` | string | opt | Nome de estilo NDT. Ex: `"corpo"`. Disponível a partir de NCRTF 1.1.0. |
| `children` | Inline[] | **req** | Nós inline. Pode estar vazio (`[]`) para parágrafo em branco. |

---

### heading

```json
{
  "type":      "heading",
  "level":     1,
  "alignment": "left",
  "children": [
    { "type": "text", "text": "1. Introdução", "marks": [] }
  ]
}
```

`level`: 1–6 (mapeado para H1–H6 e `Section` do normaxis-pdf).

---

### list

```json
{
  "type":      "list",
  "list_type": "bullet",
  "children": [
    {
      "type":     "list_item",
      "indent":   0,
      "checked":  null,
      "children": [
        { "type": "text", "text": "Primeiro item", "marks": [] }
      ]
    },
    {
      "type":     "list_item",
      "indent":   1,
      "checked":  null,
      "children": [
        { "type": "text", "text": "Item aninhado", "marks": ["italic"] }
      ]
    }
  ]
}
```

`list_type`: `"bullet"` | `"ordered"` | `"checklist"`

Para checklist, `checked` é `true` ou `false`. Para outros tipos, é `null`.

---

### table

```json
{
  "type":       "table",
  "caption":    "Tabela 1 — Resumo",
  "col_widths": [50, 25, 25],
  "head": [
    {
      "type": "table_row",
      "cells": [
        {
          "type":      "table_cell",
          "header":    true,
          "col_span":  1,
          "row_span":  1,
          "alignment": "center",
          "children":  [{ "type": "text", "text": "Descrição", "marks": ["bold"] }]
        },
        {
          "type":      "table_cell",
          "header":    true,
          "alignment": "center",
          "children":  [{ "type": "text", "text": "Estado", "marks": ["bold"] }]
        },
        {
          "type":      "table_cell",
          "header":    true,
          "alignment": "center",
          "children":  [{ "type": "text", "text": "Data", "marks": ["bold"] }]
        }
      ]
    }
  ],
  "body": [
    {
      "type": "table_row",
      "cells": [
        {
          "type":      "table_cell",
          "header":    false,
          "alignment": "left",
          "children":  [{ "type": "text", "text": "Revisão do PDM", "marks": [] }]
        },
        {
          "type":      "table_cell",
          "alignment": "center",
          "children":  [{ "type": "text", "text": "Concluído", "marks": [] }]
        },
        {
          "type":      "table_cell",
          "alignment": "center",
          "children":  [{ "type": "text", "text": "Jun 2026", "marks": [] }]
        }
      ]
    }
  ]
}
```

`col_widths`: percentagens que somam 100. Se omitido, distribuição uniforme.

---

### blockquote

```json
{
  "type":        "blockquote",
  "attribution": "Relatório de Actividades 2025",
  "children": [
    { "type": "text", "text": "Os resultados superaram as expectativas.", "marks": [] }
  ]
}
```

Renderizado como parágrafo com indentação e cor cinza.

---

### code_block

```json
{
  "type":     "code_block",
  "language": "rust",
  "code":     "fn main() {\n    println!(\"Olá NORMAXIS!\");\n}"
}
```

`language` é informativo (sem syntax highlighting no PDF). Renderizado com fonte Liberation Mono.

---

### image

```json
{
  "type":          "image",
  "src":           "data:image/png;base64,iVBOR...",
  "alt":           "Descrição da imagem para acessibilidade",
  "caption":       "Fig. 1 — Legenda da figura",
  "alignment":     "center",
  "width_percent": 80
}
```

| Campo | Tipo | | Descrição |
|---|---|---|---|
| `src` | string | **req** | Data URI base64 ou `"asset:chave"`. |
| `alt` | string | **rec** | Texto alternativo. Obrigatório para PDF/UA. |
| `caption` | string | opt | Legenda apresentada abaixo da imagem. |
| `alignment` | enum | opt | `"left"` `"center"` `"right"`. Default: `"center"`. |
| `width_percent` | f64 | opt | Percentagem da largura útil. Default: `100`. |

---

### horizontal_rule e page_break

```json
{ "type": "horizontal_rule" }
{ "type": "page_break" }
```

---

## Nós Inline

### text

O nó fundamental. Contém texto com zero ou mais marks.

```json
{ "type": "text", "text": "Texto simples",     "marks": [] }
{ "type": "text", "text": "Negrito",            "marks": ["bold"] }
{ "type": "text", "text": "Negrito e itálico",  "marks": ["bold", "italic"] }
{ "type": "text", "text": "Cor azul",
  "marks": [{ "type": "color", "value": "#003399" }] }
{ "type": "text", "text": "Misto",
  "marks": ["bold", { "type": "color", "value": "#003399" }] }
```

**Com OpenType features (NCRTF 1.2.0+):**

```json
{
  "type":  "text",
  "text":  "1.234,56 €",
  "marks": [],
  "opentype_features": { "tnum": true }
}
```

---

### link

```json
{
  "type":     "link",
  "href":     "https://www.cm-lisboa.pt",
  "title":    "Website da Câmara Municipal de Lisboa",
  "target":   "_blank",
  "children": [
    { "type": "text", "text": "cm-lisboa.pt", "marks": [] }
  ]
}
```

---

### hard_break

Quebra de linha forçada dentro de um parágrafo (equivalente ao `<br>` em HTML).

```json
{ "type": "hard_break" }
```

---

### footnote_ref (NCRTF 1.3.0+)

Referência inline a uma nota de rodapé. O número corresponde à entrada em `footnotes` no NDT ou NDF resultante. O NCRTF apenas referencia notas pelo número — a definição do conteúdo das notas e a sua custódia pertencem ao NDT (template) ou ao NDF (documento gerado), não ao NCRTF.

```json
{ "type": "footnote_ref", "number": 1 }
```

Usado dentro de `children[]` de um `paragraph`:

```json
{
  "type": "paragraph",
  "children": [
    { "type": "text",         "text": "Conforme legislação vigente" },
    { "type": "footnote_ref", "number": 1 },
    { "type": "text",         "text": ", aplica-se o seguinte procedimento." }
  ]
}
```

---

## Marks — referência completa

Os marks podem ser **strings simples** ou **objectos**. O campo `marks` aceita mistura dos dois.

### Marks simples (sem parâmetros)

```json
"marks": ["bold"]
"marks": ["italic"]
"marks": ["underline"]
"marks": ["strikethrough"]
"marks": ["superscript"]
"marks": ["subscript"]
"marks": ["code"]
"marks": ["small_caps"]
```

| Mark | Versão | Renderização |
|---|---|---|
| `bold` | 1.0 | Negrito |
| `italic` | 1.0 | Itálico |
| `underline` | 1.0 | Sublinhado (cor do texto por defeito) |
| `strikethrough` | 1.0 | Riscado |
| `superscript` | 1.2 | Sobrescrito — 66% size, +35% Y offset |
| `subscript` | 1.2 | Subscrito — 66% size, -15% Y offset |
| `code` | 1.0 | Monospace inline (Liberation Mono) |
| `small_caps` | 1.2 | Small caps — OpenType `smcp` ou sintético 80% |

### Marks com parâmetros

```json
{ "type": "color",     "value": "#CC0000"  }
{ "type": "highlight", "value": "yellow"   }
{ "type": "font_size", "value": 14.0       }
```

**Underline com cor (NCRTF 1.2.0+):**

```json
{ "type": "underline", "color": "#003399" }
```

**Strikethrough com cor (NCRTF 1.2.0+):**

```json
{ "type": "strikethrough", "color": "#CC0000" }
```

### Cores de highlight

Valores aceites para `highlight.value`:

| Valor | Cor |
|---|---|
| `yellow` | Amarelo #FFFF00 |
| `green` | Verde #00FF00 |
| `cyan` | Ciano #00FFFF |
| `magenta` | Magenta #FF00FF |
| `blue` | Azul #0000FF |
| `red` | Vermelho #FF0000 |
| `dark_blue` | Azul escuro #000080 |
| `dark_cyan` | Ciano escuro #008080 |
| `dark_green` | Verde escuro #008000 |
| `dark_magenta` | Magenta escuro #800080 |
| `dark_red` | Vermelho escuro #800000 |
| `dark_yellow` | Amarelo escuro #808000 |
| `dark_gray` | Cinza escuro #404040 |
| `light_gray` | Cinza claro #C0C0C0 |
| `black` | Preto #000000 |
| `white` | Branco #FFFFFF |

---

## Soft hyphen

O carácter `U+00AD` (soft hyphen) indica um ponto de hifenização preferido. Invisível quando a linha não quebra nesse ponto; renderiza um hífen quando a linha quebra.

```json
{ "type": "text", "text": "im\u00adple\u00admen\u00adta\u00adção", "marks": [] }
```

Útil para palavras longas em texto justificado que causam espaçamentos irregulares.

---

## Integração com Lexical (frontend)

O Lexical não serializa para NCRTF nativamente — é necessário um conversor customizado.

### Mapeamento Lexical → NCRTF

| Nó Lexical | Bloco NCRTF | Notas |
|---|---|---|
| `ParagraphNode` | `paragraph` | `alignment` mapeado directamente |
| `HeadingNode` | `heading` | `tag: "h1"–"h6"` → `level: 1–6` |
| `ListNode` | `list` | `listType: "bullet"/"number"/"check"` → `list_type` |
| `ListItemNode` | `list_item` | `indent`, `checked` mapeados directamente |
| `TableNode` | `table` | `colWidths` → `col_widths` em percentagens |
| `TableRowNode` | `table_row` | `first` row → `head`, resto → `body` |
| `TableCellNode` | `table_cell` | `headerState` → `header: bool` |
| `ImageNode` | `image` | `src` (base64 data URI ou `asset:chave`), `altText` → `alt`. URLs externas devem ser resolvidas pela mini-app para `asset:...` ou data URI antes de enviar ao backend — o NCRTF não aceita URLs HTTP/HTTPS directamente. |
| `HorizontalRuleNode` | `horizontal_rule` | — |

### Mapeamento de marks Lexical → NCRTF

| Format Lexical | Mark NCRTF |
|---|---|
| `bold` | `"bold"` |
| `italic` | `"italic"` |
| `underline` | `"underline"` |
| `strikethrough` | `"strikethrough"` |
| `subscript` | `"subscript"` |
| `superscript` | `"superscript"` |
| `code` | `"code"` |
| TextNode com `style: "color: #..."` | `{"type": "color", "value": "#..."}` |
| TextNode com `style: "background-color: ..."` | `{"type": "highlight", "value": "yellow"}` |
| TextNode com `style: "font-size: 14px"` | `{"type": "font_size", "value": 14.0}` |

---

## Serialização em Rust

```rust
use normaxis_pdf::{parse_ncrtf, ncrtf_to_elements, DocumentStyle};

// Parsear NCRTF
let ncrtf_json = r#"{ "ncrtf": "1.3.0", "blocks": [...] }"#;
let doc = parse_ncrtf(ncrtf_json)?;

// Converter para elementos normaxis-pdf
let style = DocumentStyle::default();
let elements = ncrtf_to_elements(&doc, &style);

// Usar num DocumentBuilder
let pdf = DocumentBuilder::new("Título")
    .push_ncrtf(ncrtf_json)?
    .render_to_bytes()?;
```

---

## Exemplo completo

```json
{
  "ncrtf": "1.3.0",
  "meta": {
    "title":      "Relatório de Actividades 2026",
    "lang":       "pt",
    "author":     "Serviço de Planeamento",
    "created_at": "2026-04-29T09:00:00Z",
    "custom": {
      "reference":  "REF/2026/001",
      "department": "Divisão de Urbanismo"
    }
  },
  "blocks": [
    {
      "type":      "heading",
      "level":     1,
      "alignment": "left",
      "children":  [
        { "type": "text", "text": "Relatório de Actividades 2026", "marks": [] }
      ]
    },
    {
      "type":      "paragraph",
      "alignment": "justify",
      "children": [
        { "type": "text", "text": "O presente relatório descreve as actividades desenvolvidas durante o ano de ", "marks": [] },
        { "type": "text", "text": "2026", "marks": ["bold"] },
        { "type": "text", "text": " pela Divisão de Urbanismo.", "marks": [] }
      ]
    },
    {
      "type":      "heading",
      "level":     2,
      "alignment": "left",
      "children":  [
        { "type": "text", "text": "1.1 Síntese de Resultados", "marks": [] }
      ]
    },
    {
      "type":      "list",
      "list_type": "bullet",
      "children": [
        {
          "type":    "list_item",
          "indent":  0,
          "checked": null,
          "children": [{ "type": "text", "text": "Aprovação de 42 projectos de construção", "marks": [] }]
        },
        {
          "type":    "list_item",
          "indent":  0,
          "checked": null,
          "children": [{ "type": "text", "text": "Revisão do PDM concluída", "marks": [] }]
        },
        {
          "type":    "list_item",
          "indent":  1,
          "checked": null,
          "children": [
            { "type": "text", "text": "Consulta pública: ", "marks": [] },
            { "type": "text", "text": "1.247 participantes", "marks": ["bold"] }
          ]
        }
      ]
    },
    {
      "type":       "table",
      "caption":    "Tabela 1 — Resumo de actividades por trimestre",
      "col_widths": [40, 20, 20, 20],
      "head": [
        {
          "type": "table_row",
          "cells": [
            { "type": "table_cell", "header": true, "alignment": "left",   "children": [{ "type": "text", "text": "Actividade",   "marks": ["bold"] }] },
            { "type": "table_cell", "header": true, "alignment": "center", "children": [{ "type": "text", "text": "T1",           "marks": ["bold"] }] },
            { "type": "table_cell", "header": true, "alignment": "center", "children": [{ "type": "text", "text": "T2",           "marks": ["bold"] }] },
            { "type": "table_cell", "header": true, "alignment": "center", "children": [{ "type": "text", "text": "Total",        "marks": ["bold"] }] }
          ]
        }
      ],
      "body": [
        {
          "type": "table_row",
          "cells": [
            { "type": "table_cell", "alignment": "left",   "children": [{ "type": "text", "text": "Licenças emitidas",  "marks": [] }] },
            { "type": "table_cell", "alignment": "center", "children": [{ "type": "text", "text": "18",                 "marks": [] }] },
            { "type": "table_cell", "alignment": "center", "children": [{ "type": "text", "text": "24",                 "marks": [] }] },
            { "type": "table_cell", "alignment": "center", "children": [{ "type": "text", "text": "42",                 "marks": ["bold"] }] }
          ]
        }
      ]
    },
    { "type": "horizontal_rule" },
    {
      "type":      "paragraph",
      "alignment": "justify",
      "children": [
        { "type": "text", "text": "Para mais informações consultar ", "marks": [] },
        {
          "type":     "link",
          "href":     "https://www.cm-lisboa.pt/urbanismo",
          "children": [{ "type": "text", "text": "cm-lisboa.pt/urbanismo", "marks": [] }]
        },
        { "type": "text", "text": ".", "marks": [] }
      ]
    }
  ]
}
```

---

## Versões NCRTF

| Versão | Adições |
|---|---|
| 1.0 | Formato base — paragraph, heading, list, table, image, blockquote, code_block, horizontal_rule, page_break, text, link, hard_break, marks simples |
| 1.1.0 | Campo `style` em `paragraph` e `heading` — referência a NamedStyle NDT |
| 1.2.0 | Marks com parâmetros: underline/strikethrough com cor, highlight (16 cores), superscript, subscript, small_caps. Campo `opentype_features` em `text`. |
| **1.3.0** | `footnote_ref` inline. Soft hyphen `\u00AD` em `text`. |

---

## Erros comuns

**`ParseError("...")`**  
JSON inválido. Verificar com `jq . ficheiro.json`. Causas comuns: vírgula extra no fim de array, aspas não fechadas, `null` em vez de `[]`.

**Texto rico não aparece no PDF**  
O array `children` está vazio. Um `paragraph` com `"children": []` renderiza uma linha em branco.

**Marca `bold` não funciona**  
Verificar que o valor é a string `"bold"`, não o objecto `{"type":"bold"}`. As marks simples são strings.

**Highlight não reconhecido**  
O valor de `highlight.value` tem de ser um dos 16 nomes da tabela de cores. Valores hex (`"#FFFF00"`) não são aceites em `highlight` — usar `color` para cores arbitrárias.

**Tabela sem cabeçalho**  
`head` pode ser um array vazio `[]`. Nesse caso, todas as linhas ficam em `body`.

**Nó `footnote_ref` não disponível**  
Verificar que a versão do NCRTF no documento é `"1.3.0"` ou superior. Versões anteriores ignoram este nó.

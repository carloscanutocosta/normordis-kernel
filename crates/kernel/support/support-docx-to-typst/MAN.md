# MAN — support-docx-to-typst

## Objectivo

Conversor de DOCX para código Typst. Extrai parágrafos, estilos e formatação básica de ficheiros Word e gera código Typst equivalente para renderização posterior.

---

## Contrato público

```rust
/// Converte um ficheiro DOCX no disco para código Typst.
pub fn convert_docx_file(path: impl AsRef<Path>) -> Result<String, ConvertError>;

/// Converte bytes de um ficheiro DOCX (em memória) para código Typst.
pub fn convert_docx_bytes(bytes: &[u8]) -> Result<String, ConvertError>;

/// Erros de conversão.
pub struct ConvertError(pub String);
impl std::error::Error for ConvertError { ... }
```

### Tipos internos (expostos para extensibilidade)

```rust
pub enum ParaStyle {
    Normal,
    Heading(u8),    // Heading1=1 … Heading6=6
    ListBullet,
    ListNumber,
}

pub struct RunData {
    pub text: String,
    pub bold: bool,
}

/// Extrai blocos de parágrafo do XML DOCX.
pub fn parse_blocks(xml: &str) -> Vec<(ParaStyle, Vec<RunData>)>;

/// Detecta o estilo de um parágrafo a partir do valor do atributo `pStyle`.
pub fn detect_style(val: &str) -> ParaStyle;

/// Converte blocos extraídos para código Typst.
pub fn blocks_to_typst(blocks: &[(ParaStyle, Vec<RunData>)]) -> String;
```

---

## Mapeamento DOCX → Typst

| Estilo DOCX | Typst gerado |
|---|---|
| `Normal` | parágrafo simples |
| `Heading1` | `= Título` |
| `Heading2` | `== Título` |
| `Heading3`–`6` | `===` … `======` |
| `ListBullet` | `- item` |
| `ListNumber` | `+ item` |
| Run com `bold` | `*texto*` |

---

## Como usar

### A partir de ficheiro

```rust
use support_docx_to_typst::convert_docx_file;

let typst_source = convert_docx_file("legacy_template.docx")?;
// Usar como input para render_typst_document
```

### A partir de bytes (upload web, Tauri, etc.)

```rust
use support_docx_to_typst::convert_docx_bytes;

let docx_bytes = std::fs::read("template.docx")?;
let typst_source = convert_docx_bytes(&docx_bytes)?;
```

### Pipeline completo DOCX → PDF

```rust
use support_docx_to_typst::convert_docx_file;
use documentos_pdf::{WarmFonts, render_typst_document};

let fonts = WarmFonts::load()?;
let typst_source = convert_docx_file("template.docx")?;
let pdf_bytes = render_typst_document(&typst_source, &fonts)?;
```

---

## Invariantes

- A conversão é stateless — cada chamada é independente.
- O código Typst gerado é válido para Typst 0.14+.
- Parágrafos vazios são preservados como linhas em branco.
- Runs sem texto são ignorados silenciosamente.

---

## Limites actuais

- Sem suporte a imagens embutidas no DOCX.
- Sem suporte a tabelas (`<w:tbl>`).
- Sem suporte a caixas de texto (`<w:txbxContent>`).
- Sem suporte a notas de rodapé nem de fim.
- Formatação limitada a negrito — itálico, sublinhado e cor são ignorados.
- Sem suporte a listas aninhadas.

---

## ToDo

- [ ] Suporte a itálico e sublinhado.
- [ ] Extracção de tabelas como `#table(...)` Typst.
- [ ] Extracção de imagens como ficheiros externos referenciados.
- [ ] Listas aninhadas com indentação.

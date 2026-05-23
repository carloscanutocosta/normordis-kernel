use serde::Deserialize;
use serde_json::Value;

use super::marks::{MarkValue, OpenTypeFeatures};
use crate::layout::TextAlign;
use crate::{NormaxisPdfError, Result};

// ── Root document ─────────────────────────────────────────────────────────────

/// Root NCRTF document.
#[derive(Debug, Clone, Deserialize)]
pub struct NcrtfDocument {
    /// Format version, e.g. `"1.0"`.
    pub ncrtf: String,
    #[serde(default)]
    pub meta: DocumentMeta,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DocumentMeta {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    /// Arbitrary extra fields (reference, department, etc.).
    pub custom: Option<Value>,
}

// ── Block nodes ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Paragraph(ParagraphBlock),
    Heading(HeadingBlock),
    List(ListBlock),
    Table(TableBlock),
    Blockquote(BlockquoteBlock),
    CodeBlock(CodeBlockNode),
    Image(ImageBlock),
    HorizontalRule,
    PageBreak,
    FixedBox(FixedBoxBlock),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParagraphBlock {
    pub alignment: Option<TextAlign>,
    #[serde(default)]
    pub indent: Option<u8>,
    pub style: Option<String>,
    pub children: Vec<Inline>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeadingBlock {
    /// Heading depth 1–6 (analogous to H1–H6).
    pub level: u8,
    pub alignment: Option<TextAlign>,
    pub children: Vec<Inline>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListBlock {
    pub list_type: ListType,
    pub children: Vec<ListItem>,
}

/// Single item inside a `ListBlock`. The `"type": "list_item"` JSON field is
/// silently ignored by serde (unknown fields are dropped by default).
#[derive(Debug, Clone, Deserialize)]
pub struct ListItem {
    #[serde(default)]
    pub indent: Option<u8>,
    /// `None` = not a checklist item; `Some(true/false)` = checked state.
    pub checked: Option<bool>,
    pub children: Vec<Inline>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TableBlock {
    pub caption: Option<String>,
    pub col_widths: Option<Vec<f64>>,
    pub head: Vec<TableRow>,
    pub body: Vec<TableRow>,
}

/// Table row. The `"type": "table_row"` JSON field is ignored by serde.
#[derive(Debug, Clone, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// Table cell. The `"type": "table_cell"` JSON field is ignored by serde.
#[derive(Debug, Clone, Deserialize)]
pub struct TableCell {
    #[serde(default)]
    pub header: bool,
    #[serde(default)]
    pub col_span: Option<u8>,
    #[serde(default)]
    pub row_span: Option<u8>,
    pub alignment: Option<TextAlign>,
    pub children: Vec<Inline>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockquoteBlock {
    pub attribution: Option<String>,
    pub children: Vec<Inline>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeBlockNode {
    pub language: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageBlock {
    /// Base64 data URI (`"data:image/png;base64,..."`) or asset key (`"asset:logo"`).
    pub src: String,
    pub alt: Option<String>,
    pub caption: Option<String>,
    pub alignment: Option<ImageAlign>,
    pub width_percent: Option<f64>,
}

impl ImageBlock {
    /// Validates that `src` is a data URI or asset reference.
    /// Returns `Err` if `src` is an HTTP/HTTPS URL.
    pub fn validate_src(&self) -> Result<()> {
        if self.src.starts_with("http://") || self.src.starts_with("https://") {
            return Err(NormaxisPdfError::Template(
                "NCRTF image.src does not accept HTTP/HTTPS URLs. \
                 Resolve to data URI (data:...) or asset reference (asset:...) first."
                    .into(),
            ));
        }
        Ok(())
    }
}

// ── Inline nodes ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    Text(TextNode),
    Link(LinkNode),
    HardBreak,
    /// Inline footnote reference mark — renders as a superscript number (NCRTF 1.3.0+).
    FootnoteRef(FootnoteRefNode),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextNode {
    pub text: String,
    pub marks: Option<Vec<MarkValue>>,
    pub opentype_features: Option<OpenTypeFeatures>,
}

/// NCRTF 1.3.0 — inline footnote reference mark.
#[derive(Debug, Clone, Deserialize)]
pub struct FootnoteRefNode {
    /// The footnote number this mark references.
    pub number: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LinkNode {
    pub href: String,
    pub title: Option<String>,
    pub target: Option<String>,
    pub children: Vec<Inline>,
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Bullet,
    Ordered,
    Checklist,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageAlign {
    Left,
    #[default]
    Center,
    Right,
}

// ── Fixed Box block ───────────────────────────────────────────────────────────

/// An NCRTF block that maps to a `FixedTextBox` element.
#[derive(Debug, Clone, Deserialize)]
pub struct FixedBoxBlock {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub overflow: Option<String>,  // "truncate" | "clip" | "shrink" | "overflow"
    pub padding_mm: Option<f64>,
    pub border: Option<BoxBorderSpec>,
    pub background: Option<String>, // hex color string e.g. "#F5F5F5"
    pub alignment: Option<TextAlign>,
    pub children: Vec<Inline>,
}

/// NCRTF representation of a border (color stored as hex string).
#[derive(Debug, Clone, Deserialize)]
pub struct BoxBorderSpec {
    pub width_mm: f64,
    pub color: String,
    pub style: Option<String>,  // "solid" | "dashed" | "dotted"
}

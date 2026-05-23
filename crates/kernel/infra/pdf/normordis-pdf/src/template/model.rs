use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_none_or_null(v: &Option<Value>) -> bool {
    matches!(v, None | Some(Value::Null))
}

// ── Root ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtDocument {
    pub ndt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<NdtMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<NdtStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fonts: Option<NdtFonts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<NdtPage>,
    /// NDT 2.0.0 — output-level options (PDF standard, compression, classification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<NdtOutput>,
    /// NDT 2.0.0 — signature field metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<NdtSignature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholders: Option<HashMap<String, PlaceholderDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zones: Option<HashMap<String, ZoneDef>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<BodyElement>,
}

// ── NDT 2.0.0 output / signature ─────────────────────────────────────────────

/// Output-level options for NDT 2.0.0 / 2.1.0.
///
/// Controls PDF standard, compression, document classification, and reference.
/// Applied automatically by `DocumentBuilder::push_ndt()`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtOutput {
    /// PDF conformance standard: `"pdf_a_1b"`, `"pdf_a_2b"`, `"pdf_ua2"`, or `"pdf17"` (default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// Compression level: `"none"`, `"fast"`, `"default"`, or `"best"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
    /// Document security classification: `"publico"`, `"interno"`, `"confidencial"`, `"reservado"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// Document reference (e.g. `"REF/2026/001"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ref: Option<String>,
    /// NDT 2.1.0 — granular accessibility configuration.
    /// When standard = "pdf_ua2", this is applied automatically with defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<crate::compliance::ua::AccessibilityConfig>,
}

/// Signature metadata for NDT 2.0.0.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtSignature {
    /// Visual signature field position on the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<NdtSignatureField>,
    /// Reason for signing (embedded in PDF signature metadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Location (embedded in PDF signature metadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Visual position of a signature field on the page.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtSignatureField {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Word compatibility mode extracted from word/settings.xml.
    /// 12=Word2007, 14=Word2010, 15=Word2013, 16=Word2016+
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compat_mode: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_top_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_bottom_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_left_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_right_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size_body: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtFonts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub families: Option<Vec<FontFamilyDef>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontFamilyDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regular: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold_italic: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NdtPage {
    #[serde(skip_serializing_if = "is_none_or_null")]
    pub header: Option<Value>,
    #[serde(skip_serializing_if = "is_none_or_null")]
    pub footer: Option<Value>,
}

// ── Placeholder definitions ───────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaceholderDef {
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "is_none_or_null")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

// ── Zone ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZoneDef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<BodyElement>,
}

// ── Body elements ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BodyElement {
    Paragraph(ParagraphElement),
    Heading(HeadingElement),
    RichText(RichTextElement),
    Table(TableElement),
    List(ListElement),
    Image(ImageElement),
    Spacer(SpacerElement),
    HorizontalRule,
    PageBreak,
    FixedText(FixedTextElement),
    FixedImage(FixedImageElement),
    FixedLine(FixedLineElement),
    FixedBox(FixedBoxElement),
    ZoneRef(ZoneRefElement),
    Conditional(ConditionalElement),
    Repeat(RepeatElement),
    Include(IncludeElement),
    // v1.5.0
    FootnoteRef(FootnoteRefElement),
    Toc(TocElement),
    AcroformField(AcroformFieldElement),
}

// ── v1.5.0 element models ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FootnoteRefElement {
    pub number: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark_style: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TocElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_char: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcroformFieldElement {
    pub field_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_by_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    pub rect: AcroformRect,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcroformRect {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParagraphElement {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeadingElement {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RichTextElement {
    /// Inline NCRTF JSON string OR `"{{placeholder}}"` pointing to data.
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TableElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_widths: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripe: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_type: Option<String>,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageElement {
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpacerElement {
    pub height_mm: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedTextElement {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedImageElement {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedLineElement {
    pub x1_mm: f64,
    pub y1_mm: f64,
    pub x2_mm: f64,
    pub y2_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedBoxElement {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZoneRefElement {
    pub zone: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionalElement {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    #[serde(skip_serializing_if = "is_none_or_null")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<BodyElement>,
    #[serde(rename = "else", default, skip_serializing_if = "Vec::is_empty")]
    pub else_branch: Vec<BodyElement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepeatElement {
    pub items: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_var: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<BodyElement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncludeElement {
    pub path: String,
    #[serde(skip_serializing_if = "is_none_or_null")]
    pub data: Option<Value>,
}

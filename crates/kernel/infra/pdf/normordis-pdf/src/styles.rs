use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{error::NormaxisPdfError, layout::TextAlign};

/// Diagonal text watermark rendered on every page.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::{Watermark, RgbColor};
///
/// let wm = Watermark::new("RASCUNHO")
///     .opacity(0.12)
///     .color(RgbColor { r: 0.8, g: 0.0, b: 0.0 })
///     .font_size(72.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watermark {
    /// Text to display (e.g. "RASCUNHO", "CÓPIA NÃO CERTIFICADA").
    pub text: String,
    /// Opacity from 0.0 (invisible) to 1.0 (opaque). Default: 0.10.
    pub opacity: f64,
    /// Text color. Default: light grey (0.7, 0.7, 0.7).
    pub color: RgbColor,
    /// Font size in points. Default: 72.0.
    pub font_size: f64,
    /// Rotation angle in degrees (counter-clockwise). Default: 45.0.
    pub angle_deg: f64,
}

impl Watermark {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
    pub fn opacity(mut self, v: f64) -> Self {
        self.opacity = v;
        self
    }
    pub fn color(mut self, c: RgbColor) -> Self {
        self.color = c;
        self
    }
    pub fn font_size(mut self, pt: f64) -> Self {
        self.font_size = pt;
        self
    }
    pub fn angle_deg(mut self, deg: f64) -> Self {
        self.angle_deg = deg;
        self
    }
}

impl Default for Watermark {
    fn default() -> Self {
        Self {
            text: "RASCUNHO".into(),
            opacity: 0.10,
            color: RgbColor {
                r: 0.7,
                g: 0.7,
                b: 0.7,
            },
            font_size: 72.0,
            angle_deg: 45.0,
        }
    }
}

/// Page orientation.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Full styling configuration for a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStyle {
    pub page_size: PageSize,
    pub orientation: Orientation,
    pub margin_top_mm: f64,
    pub margin_bottom_mm: f64,
    pub margin_left_mm: f64,
    pub margin_right_mm: f64,
    pub font_size_body: f64,
    pub font_size_title: f64,
    pub font_size_section: f64,
    pub font_size_small: f64,
    /// Line height multiplier (e.g. 1.4 = 140% of font size).
    pub line_height: f64,
    /// Primary brand colour — used for section headings and table headers.
    pub primary_color: RgbColor,
    /// Default body text colour.
    pub text_color: RgbColor,
    /// Named paragraph/table styles. Built-in defaults are always available.
    #[serde(default)]
    pub named_styles: HashMap<String, NamedStyle>,
    /// Minimum lines of a paragraph that must appear at the bottom of a page
    /// before a page break (orphan control). 0 = disabled. Default: 2.
    #[serde(default = "default_orphan_lines")]
    pub min_orphan_lines: u8,
    /// Minimum lines of a paragraph that must appear at the top of a page
    /// after a page break (widow control). 0 = disabled. Default: 2.
    #[serde(default = "default_widow_lines")]
    pub min_widow_lines: u8,
}

fn default_orphan_lines() -> u8 {
    2
}
fn default_widow_lines() -> u8 {
    2
}

impl Default for DocumentStyle {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            orientation: Orientation::Portrait,
            margin_top_mm: 20.0,
            margin_bottom_mm: 20.0,
            margin_left_mm: 25.0,
            margin_right_mm: 20.0,
            font_size_body: 11.0,
            font_size_title: 16.0,
            font_size_section: 13.0,
            font_size_small: 9.0,
            line_height: 1.4,
            // Institutional blue #003399
            primary_color: RgbColor {
                r: 0.0,
                g: 0.2,
                b: 0.6,
            },
            // Near-black #1A1A1A
            text_color: RgbColor {
                r: 0.102,
                g: 0.102,
                b: 0.102,
            },
            named_styles: HashMap::new(),
            min_orphan_lines: 2,
            min_widow_lines: 2,
        }
    }
}

/// Supported paper sizes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PageSize {
    #[default]
    A4,
    A3,
    Letter,
}

impl PageSize {
    /// Returns `(width_mm, height_mm)` for the page size.
    pub fn dimensions_mm(&self) -> (f64, f64) {
        match self {
            PageSize::A4 => (210.0, 297.0),
            PageSize::A3 => (297.0, 420.0),
            PageSize::Letter => (215.9, 279.4),
        }
    }
}

/// An RGB colour with components in the range [0.0, 1.0].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl RgbColor {
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Parse a CSS-style hex colour (e.g. `"#003399"` or `"003399"`).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
        })
    }
}

// ── Named Styles ─────────────────────────────────────────────────────────────

/// A named paragraph style (equivalent to a Word Paragraph Style).
///
/// All fields are `Option<T>` — `None` means "inherit from parent or document defaults".
/// Styles form an inheritance chain via the `extends` field.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamedStyle {
    /// Parent style name. Resolution walks the chain until it reaches a style
    /// with no `extends`, then fills remaining `None` fields from `DocumentStyle` defaults.
    pub extends: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub alignment: Option<TextAlign>,
    pub space_before_mm: Option<f64>,
    pub space_after_mm: Option<f64>,
    pub indent_left_mm: Option<f64>,
    pub indent_right_mm: Option<f64>,
    pub indent_first_line_mm: Option<f64>,
    /// Explicit text colour override. `None` inherits from document.
    pub color: Option<RgbColor>,
    /// Font family name (e.g. "LiberationSerif", "LiberationMono").
    /// `None` inherits from parent or document default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
}

/// A fully-resolved paragraph style — no `Option` fields.
///
/// Produced by `StyleResolver::resolve`. Callers read directly from these fields.
#[derive(Debug, Clone)]
pub struct ResolvedStyle {
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub alignment: TextAlign,
    pub space_before_mm: f64,
    pub space_after_mm: f64,
    pub indent_left_mm: f64,
    pub indent_right_mm: f64,
    pub indent_first_line_mm: f64,
    pub color: Option<RgbColor>,
    /// Resolved font family name — never empty.
    pub font_family: String,
}

/// Resolves named styles against a `DocumentStyle`, with cycle detection.
pub struct StyleResolver<'a> {
    styles: &'a HashMap<String, NamedStyle>,
    doc: &'a DocumentStyle,
}

impl<'a> StyleResolver<'a> {
    pub fn new(styles: &'a HashMap<String, NamedStyle>, doc: &'a DocumentStyle) -> Self {
        Self { styles, doc }
    }

    /// Resolves `name` to a `ResolvedStyle`, traversing the inheritance chain.
    ///
    /// Returns `Err(StyleCycleError)` if a cycle is detected, or
    /// `Err(UnknownStyle)` if the name is not found in the registry or built-ins.
    pub fn resolve(&self, name: &str) -> crate::Result<ResolvedStyle> {
        let mut visited = HashSet::new();
        // Merge built-ins into a combined lookup; user styles override built-ins.
        let builtins = default_named_styles(self.doc);
        self.resolve_chain(name, &builtins, &mut visited)
    }

    fn resolve_chain(
        &self,
        name: &str,
        builtins: &HashMap<String, NamedStyle>,
        visited: &mut HashSet<String>,
    ) -> crate::Result<ResolvedStyle> {
        if !visited.insert(name.to_string()) {
            return Err(NormaxisPdfError::StyleCycleError(name.to_string()));
        }

        // User styles take priority over built-ins.
        let style = self
            .styles
            .get(name)
            .or_else(|| builtins.get(name))
            .ok_or_else(|| NormaxisPdfError::UnknownStyle(name.to_string()))?;

        // If this style extends another, resolve the parent first, then overlay.
        let base = if let Some(ref parent) = style.extends {
            self.resolve_chain(parent, builtins, visited)?
        } else {
            self.doc_defaults()
        };

        Ok(ResolvedStyle {
            font_size: style.font_size.unwrap_or(base.font_size),
            bold: style.bold.unwrap_or(base.bold),
            italic: style.italic.unwrap_or(base.italic),
            alignment: style.alignment.unwrap_or(base.alignment),
            space_before_mm: style.space_before_mm.unwrap_or(base.space_before_mm),
            space_after_mm: style.space_after_mm.unwrap_or(base.space_after_mm),
            indent_left_mm: style.indent_left_mm.unwrap_or(base.indent_left_mm),
            indent_right_mm: style.indent_right_mm.unwrap_or(base.indent_right_mm),
            indent_first_line_mm: style
                .indent_first_line_mm
                .unwrap_or(base.indent_first_line_mm),
            color: style.color.clone().or(base.color),
            font_family: style.font_family.clone().unwrap_or(base.font_family),
        })
    }

    /// Document-level defaults used as the ultimate fallback in the chain.
    fn doc_defaults(&self) -> ResolvedStyle {
        let space_after = self.doc.font_size_body * 0.3 * 25.4 / 72.0;
        ResolvedStyle {
            font_size: self.doc.font_size_body,
            bold: false,
            italic: false,
            alignment: TextAlign::Justify,
            space_before_mm: 0.0,
            space_after_mm: space_after,
            indent_left_mm: 0.0,
            indent_right_mm: 0.0,
            indent_first_line_mm: 0.0,
            color: None,
            font_family: "LiberationSans".to_string(),
        }
    }
}

/// Returns the 7 built-in named styles computed from `doc` defaults.
///
/// These are always available without declaring them in `DocumentStyle.named_styles`.
/// User-defined styles with the same name override these.
pub fn default_named_styles(doc: &DocumentStyle) -> HashMap<String, NamedStyle> {
    let pt_to_mm = |pt: f64| pt * 25.4 / 72.0;

    let mut m = HashMap::new();

    // normal — body text baseline
    m.insert(
        "normal".into(),
        NamedStyle {
            font_size: Some(doc.font_size_body),
            bold: Some(false),
            italic: Some(false),
            alignment: Some(TextAlign::Justify),
            space_after_mm: Some(pt_to_mm(doc.font_size_body) * 0.3),
            ..Default::default()
        },
    );

    // heading_1 — document title level
    m.insert(
        "heading_1".into(),
        NamedStyle {
            font_size: Some(doc.font_size_title),
            bold: Some(true),
            alignment: Some(TextAlign::Left),
            space_before_mm: Some(8.0),
            space_after_mm: Some(4.0),
            color: Some(doc.primary_color.clone()),
            ..Default::default()
        },
    );

    // heading_2 — section level
    m.insert(
        "heading_2".into(),
        NamedStyle {
            extends: Some("heading_1".into()),
            font_size: Some(doc.font_size_section),
            space_before_mm: Some(6.0),
            space_after_mm: Some(3.0),
            color: Some(doc.text_color.clone()),
            ..Default::default()
        },
    );

    // heading_3 — sub-section level
    m.insert(
        "heading_3".into(),
        NamedStyle {
            extends: Some("heading_2".into()),
            font_size: Some(doc.font_size_body),
            space_before_mm: Some(4.0),
            space_after_mm: Some(2.0),
            ..Default::default()
        },
    );

    // caption — figure/table captions
    m.insert(
        "caption".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_size: Some(doc.font_size_small),
            italic: Some(true),
            alignment: Some(TextAlign::Center),
            space_before_mm: Some(2.0),
            space_after_mm: Some(4.0),
            ..Default::default()
        },
    );

    // table_header — bold, left-aligned header cells
    m.insert(
        "table_header".into(),
        NamedStyle {
            extends: Some("normal".into()),
            bold: Some(true),
            alignment: Some(TextAlign::Left),
            space_before_mm: Some(0.0),
            space_after_mm: Some(0.0),
            ..Default::default()
        },
    );

    // table_body — standard body cell text
    m.insert(
        "table_body".into(),
        NamedStyle {
            extends: Some("normal".into()),
            alignment: Some(TextAlign::Left),
            space_before_mm: Some(0.0),
            space_after_mm: Some(0.0),
            ..Default::default()
        },
    );

    // footnote — small text at bottom of page
    m.insert(
        "footnote".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_size: Some(9.0),
            space_before_mm: Some(0.5),
            space_after_mm: Some(0.5),
            ..Default::default()
        },
    );

    // TOC entry styles — indentation increases with level
    m.insert(
        "toc_1".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_size: Some(11.0),
            bold: Some(true),
            space_after_mm: Some(1.0),
            ..Default::default()
        },
    );

    m.insert(
        "toc_2".into(),
        NamedStyle {
            extends: Some("toc_1".into()),
            bold: Some(false),
            indent_left_mm: Some(8.0),
            ..Default::default()
        },
    );

    m.insert(
        "toc_3".into(),
        NamedStyle {
            extends: Some("toc_2".into()),
            indent_left_mm: Some(16.0),
            font_size: Some(10.0),
            ..Default::default()
        },
    );

    m
}

// ── SecurityClassification ────────────────────────────────────────────────────

/// Document security classification level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClassification {
    #[default]
    Public,
    Internal,
    Confidential,
    Reserved,
}

impl SecurityClassification {
    /// Portuguese label for this classification level.
    pub fn label_pt(self) -> &'static str {
        match self {
            Self::Public => "Público",
            Self::Internal => "Interno",
            Self::Confidential => "Confidencial",
            Self::Reserved => "Reservado",
        }
    }

    /// Watermark colour for non-public documents.
    pub fn watermark_color(self) -> RgbColor {
        match self {
            Self::Internal => RgbColor {
                r: 0.0,
                g: 0.0,
                b: 0.5,
            },
            Self::Confidential => RgbColor {
                r: 0.8,
                g: 0.0,
                b: 0.0,
            },
            Self::Reserved => RgbColor {
                r: 0.5,
                g: 0.0,
                b: 0.0,
            },
            Self::Public => RgbColor {
                r: 0.7,
                g: 0.7,
                b: 0.7,
            },
        }
    }
}

// ── TraceabilityMetadata ──────────────────────────────────────────────────────

/// Traceability metadata for CRA/NIS2 compliance.
///
/// When set on a [`DocumentBuilder`], this is embedded in the document context
/// and — if classification is non-public — automatically applies a classification
/// watermark to every page.
///
/// [`DocumentBuilder`]: crate::DocumentBuilder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilityMetadata {
    /// normordis-pdf version that generated this document.
    pub engine_version: String,
    /// NORMAXIS framework version.
    pub framework_version: Option<String>,
    /// Generating entity identifier (e.g. `"cm-lisboa"`).
    pub entity_id: String,
    /// Document reference (e.g. `"REF/2026/001"`).
    pub document_ref: Option<String>,
    /// Document security classification.
    pub classification: SecurityClassification,
    /// Generation timestamp (ISO 8601).
    pub generated_at: String,
    /// NDT template version used.
    pub ndt_version: String,
}

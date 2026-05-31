use serde::{Deserialize, Serialize};

use crate::styles::RgbColor;

/// A formatting mark applied to a `TextNode`.
///
/// Marks can be simple strings (`"bold"`) or parameterised objects
/// (`{"type": "color", "value": "#CC0000"}`).  A custom `Deserialize`
/// implementation handles both forms.
#[derive(Debug, Clone)]
pub enum MarkValue {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    Code,
    SmallCaps,
    Color(String),
    Highlight(String),
    FontSize(f64),
    UnderlineColor(String),
    StrikethroughColor(String),
}

impl<'de> Deserialize<'de> for MarkValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct MarkValueVisitor;

        impl<'de> Visitor<'de> for MarkValueVisitor {
            type Value = MarkValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(r##"a mark string ("bold") or a mark object ({"type":"color","value":"#FF0000"})"##)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<MarkValue, E> {
                match v {
                    "bold" => Ok(MarkValue::Bold),
                    "italic" => Ok(MarkValue::Italic),
                    "underline" => Ok(MarkValue::Underline),
                    "strikethrough" => Ok(MarkValue::Strikethrough),
                    "superscript" => Ok(MarkValue::Superscript),
                    "subscript" => Ok(MarkValue::Subscript),
                    "code" => Ok(MarkValue::Code),
                    "small_caps" => Ok(MarkValue::SmallCaps),
                    other => Err(de::Error::unknown_variant(
                        other,
                        &[
                            "bold",
                            "italic",
                            "underline",
                            "strikethrough",
                            "superscript",
                            "subscript",
                            "code",
                            "small_caps",
                        ],
                    )),
                }
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<MarkValue, A::Error> {
                let mut type_field: Option<String> = None;
                let mut value_field: Option<serde_json::Value> = None;
                let mut color_field: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => type_field = Some(map.next_value()?),
                        "value" => value_field = Some(map.next_value()?),
                        "color" => color_field = Some(map.next_value()?),
                        _ => {
                            let _: serde_json::Value = map.next_value()?;
                        }
                    }
                }

                let type_str = type_field.ok_or_else(|| de::Error::missing_field("type"))?;

                match type_str.as_str() {
                    "color" => {
                        let val = value_field
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(MarkValue::Color(val))
                    }
                    "highlight" => {
                        let val = value_field
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(MarkValue::Highlight(val))
                    }
                    "font_size" => {
                        let val = value_field
                            .as_ref()
                            .and_then(|v| v.as_f64())
                            .ok_or_else(|| de::Error::missing_field("value"))?;
                        Ok(MarkValue::FontSize(val))
                    }
                    "underline" => {
                        let val = color_field.ok_or_else(|| de::Error::missing_field("color"))?;
                        Ok(MarkValue::UnderlineColor(val))
                    }
                    "strikethrough" => {
                        let val = color_field.ok_or_else(|| de::Error::missing_field("color"))?;
                        Ok(MarkValue::StrikethroughColor(val))
                    }
                    other => Err(de::Error::unknown_variant(
                        other,
                        &[
                            "color",
                            "highlight",
                            "font_size",
                            "underline",
                            "strikethrough",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_any(MarkValueVisitor)
    }
}

impl Serialize for MarkValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            MarkValue::Bold => s.serialize_str("bold"),
            MarkValue::Italic => s.serialize_str("italic"),
            MarkValue::Underline => s.serialize_str("underline"),
            MarkValue::Strikethrough => s.serialize_str("strikethrough"),
            MarkValue::Superscript => s.serialize_str("superscript"),
            MarkValue::Subscript => s.serialize_str("subscript"),
            MarkValue::Code => s.serialize_str("code"),
            MarkValue::SmallCaps => s.serialize_str("small_caps"),
            MarkValue::Color(c) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("type", "color")?;
                m.serialize_entry("value", c)?;
                m.end()
            }
            MarkValue::Highlight(c) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("type", "highlight")?;
                m.serialize_entry("value", c)?;
                m.end()
            }
            MarkValue::FontSize(f) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("type", "font_size")?;
                m.serialize_entry("value", f)?;
                m.end()
            }
            MarkValue::UnderlineColor(c) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("type", "underline")?;
                m.serialize_entry("color", c)?;
                m.end()
            }
            MarkValue::StrikethroughColor(c) => {
                let mut m = s.serialize_map(Some(2))?;
                m.serialize_entry("type", "strikethrough")?;
                m.serialize_entry("color", c)?;
                m.end()
            }
        }
    }
}

// ── AppliedStyle ──────────────────────────────────────────────────────────────

/// Resolved set of active formatting marks for a single text run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppliedStyle {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub superscript: bool,
    #[serde(default)]
    pub subscript: bool,
    #[serde(default)]
    pub code: bool,
    #[serde(default)]
    pub small_caps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size_override: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough_color: Option<String>,
}

impl<'a> From<&'a [MarkValue]> for AppliedStyle {
    fn from(marks: &'a [MarkValue]) -> Self {
        let mut style = AppliedStyle::default();
        for mark in marks {
            match mark {
                MarkValue::Bold => style.bold = true,
                MarkValue::Italic => style.italic = true,
                MarkValue::Underline => style.underline = true,
                MarkValue::Strikethrough => style.strikethrough = true,
                MarkValue::Superscript => style.superscript = true,
                MarkValue::Subscript => style.subscript = true,
                MarkValue::Code => style.code = true,
                MarkValue::SmallCaps => style.small_caps = true,
                MarkValue::Color(c) => style.color = Some(c.clone()),
                MarkValue::Highlight(h) => style.highlight = Some(h.clone()),
                MarkValue::FontSize(f) => style.font_size_override = Some(*f),
                MarkValue::UnderlineColor(c) => {
                    style.underline = true;
                    style.underline_color = Some(c.clone());
                }
                MarkValue::StrikethroughColor(c) => {
                    style.strikethrough = true;
                    style.strikethrough_color = Some(c.clone());
                }
            }
        }
        style
    }
}

// ── HighlightColor ────────────────────────────────────────────────────────────

/// 16 Word-standard highlight colours (w:highlight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HighlightColor {
    Black,
    Blue,
    Cyan,
    DarkBlue,
    DarkCyan,
    DarkGray,
    DarkGreen,
    DarkMagenta,
    DarkRed,
    DarkYellow,
    Green,
    LightGray,
    Magenta,
    Red,
    White,
    Yellow,
}

impl MarkValue {
    /// Returns the mark type name as a string.
    pub fn mark_type(&self) -> &str {
        match self {
            MarkValue::Bold => "bold",
            MarkValue::Italic => "italic",
            MarkValue::Underline => "underline",
            MarkValue::Strikethrough => "strikethrough",
            MarkValue::Superscript => "superscript",
            MarkValue::Subscript => "subscript",
            MarkValue::Code => "code",
            MarkValue::SmallCaps => "small_caps",
            MarkValue::Color(_) => "color",
            MarkValue::Highlight(_) => "highlight",
            MarkValue::FontSize(_) => "font_size",
            MarkValue::UnderlineColor(_) => "underline",
            MarkValue::StrikethroughColor(_) => "strikethrough",
        }
    }
}

impl HighlightColor {
    /// Returns the sRGB colour for this Word highlight.
    pub fn to_rgb(self) -> RgbColor {
        let (r, g, b) = match self {
            Self::Black => (0.000, 0.000, 0.000),
            Self::Blue => (0.000, 0.000, 1.000),
            Self::Cyan => (0.000, 1.000, 1.000),
            Self::DarkBlue => (0.000, 0.000, 0.502),
            Self::DarkCyan => (0.000, 0.502, 0.502),
            Self::DarkGray => (0.663, 0.663, 0.663),
            Self::DarkGreen => (0.000, 0.502, 0.000),
            Self::DarkMagenta => (0.502, 0.000, 0.502),
            Self::DarkRed => (0.502, 0.000, 0.000),
            Self::DarkYellow => (0.502, 0.502, 0.000),
            Self::Green => (0.000, 1.000, 0.000),
            Self::LightGray => (0.827, 0.827, 0.827),
            Self::Magenta => (1.000, 0.000, 1.000),
            Self::Red => (1.000, 0.000, 0.000),
            Self::White => (1.000, 1.000, 1.000),
            Self::Yellow => (1.000, 1.000, 0.000),
        };
        RgbColor { r, g, b }
    }
}

// ── DecorationLine ────────────────────────────────────────────────────────────

/// A drawn line used for underline, double-underline, or strikethrough.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationLine {
    /// Override colour; `None` inherits the text colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<RgbColor>,
    /// Line thickness in mm.  Default: 0.25 mm (≈ 0.7 pt).
    #[serde(default = "DecorationLine::default_thickness")]
    pub thickness_mm: f64,
}

impl DecorationLine {
    fn default_thickness() -> f64 {
        0.25
    }

    pub fn simple() -> Self {
        Self {
            color: None,
            thickness_mm: 0.25,
        }
    }

    pub fn with_color(color: RgbColor) -> Self {
        Self {
            color: Some(color),
            thickness_mm: 0.25,
        }
    }
}

impl Default for DecorationLine {
    fn default() -> Self {
        Self::simple()
    }
}

// ── TextDecoration ────────────────────────────────────────────────────────────

/// Rich per-TextRun decoration properties (v1.4.0+).
///
/// Complements [`AppliedStyle`] with higher-fidelity options: colour-aware
/// underlines, double underlines, highlights (background rect), and
/// superscript/subscript/small-caps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextDecoration {
    /// Single underline drawn below the baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<DecorationLine>,
    /// Double underline (two parallel lines below baseline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double_underline: Option<DecorationLine>,
    /// Strikethrough line drawn at mid-character height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<DecorationLine>,
    /// Background highlight rectangle (Word `w:highlight`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<HighlightColor>,
    /// Raise text to superscript position (66% font size, raised baseline).
    #[serde(default)]
    pub superscript: bool,
    /// Lower text to subscript position (66% font size, lowered baseline).
    #[serde(default)]
    pub subscript: bool,
    /// Render as small capitals (uppercase at 80% font size, w:smallCaps).
    #[serde(default)]
    pub small_caps: bool,
}

// ── OpenTypeFeatures ──────────────────────────────────────────────────────────

/// Per-TextRun OpenType feature overrides.
///
/// All features default to `false` (use the font's built-in defaults).
/// Setting a feature to `true` explicitly enables it for this run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenTypeFeatures {
    /// Kerning (GPOS `kern`).
    #[serde(default)]
    pub kern: bool,
    /// Standard ligatures (`liga`): fi, fl, ff, ffi, ffl.
    #[serde(default)]
    pub liga: bool,
    /// Tabular (monospaced) numerals (`tnum`).
    #[serde(default)]
    pub tnum: bool,
    /// Small capitals (`smcp`).
    #[serde(default)]
    pub smcp: bool,
    /// Superscript glyphs (`sups`).
    #[serde(default)]
    pub sups: bool,
    /// Subscript glyphs (`subs`).
    #[serde(default)]
    pub subs: bool,
}

impl OpenTypeFeatures {
    /// Converts enabled features to a `rustybuzz::Feature` slice for shaping.
    pub fn to_rustybuzz_features(&self) -> Vec<rustybuzz::Feature> {
        let mut out = Vec::new();
        let mut add = |tag: [u8; 4]| {
            out.push(rustybuzz::Feature {
                tag: ttf_parser::Tag::from_bytes(&tag),
                value: 1,
                start: 0,
                end: u32::MAX,
            });
        };
        if self.kern {
            add(*b"kern");
        }
        if self.liga {
            add(*b"liga");
        }
        if self.tnum {
            add(*b"tnum");
        }
        if self.smcp {
            add(*b"smcp");
        }
        if self.sups {
            add(*b"sups");
        }
        if self.subs {
            add(*b"subs");
        }
        out
    }
}

// ── LineBreakingMode ──────────────────────────────────────────────────────────

/// Line-breaking algorithm to use for a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineBreakingMode {
    /// Greedy first-fit algorithm (fast, default).
    #[default]
    Greedy,
    /// Knuth-Plass optimal algorithm (slower, better inter-word spacing).
    /// Requires the `optimal_wrap` feature flag; falls back to `Greedy` when
    /// the feature is not compiled.
    KnuthPlass,
}

// ── GlyphUsageTracker ─────────────────────────────────────────────────────────

/// Collects glyph IDs used during rendering, keyed by `"family::variant"`.
///
/// Preparatory infrastructure for v2.0 glyph subsetting (PDF/A-1b).
/// Updated by element renderers as text is emitted.
#[derive(Debug, Clone, Default)]
pub struct GlyphUsageTracker {
    pub used: std::collections::HashMap<String, std::collections::HashSet<u16>>,
}

impl GlyphUsageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `glyph_ids` were used for the given font key.
    pub fn record(&mut self, font_key: &str, glyph_ids: impl Iterator<Item = u16>) {
        let set = self.used.entry(font_key.to_string()).or_default();
        set.extend(glyph_ids);
    }
}

// ── TextRun ───────────────────────────────────────────────────────────────────

/// A single formatted run of text — a string paired with its applied style.
///
/// Lives here (rather than in `elements::paragraph`) so that `layout::engine`
/// can depend on it without creating a circular module dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub style: AppliedStyle,
    /// Extra space between each character in mm (CSS `letter-spacing`).
    #[serde(default)]
    pub letter_spacing_mm: f64,
    /// Rich decoration properties (underline colour, highlight, small-caps, etc.).
    #[serde(default)]
    pub decoration: TextDecoration,
    /// OpenType feature overrides for this run.
    #[serde(default)]
    pub opentype: OpenTypeFeatures,
}

impl Default for TextRun {
    fn default() -> Self {
        Self {
            text: String::new(),
            style: AppliedStyle::default(),
            letter_spacing_mm: 0.0,
            decoration: TextDecoration::default(),
            opentype: OpenTypeFeatures::default(),
        }
    }
}

impl TextRun {
    /// Create a plain (unstyled) text run.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Create a superscript run for inline footnote references.
    ///
    /// The run renders as a raised, smaller number marking a footnote.
    pub fn footnote_ref(number: u32) -> Self {
        Self {
            text: number.to_string(),
            style: AppliedStyle {
                superscript: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

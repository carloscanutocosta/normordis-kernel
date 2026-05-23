use serde::{Deserialize, Serialize};

use crate::{compliance::ua::StructTag, styles::RgbColor};

/// Defines a fixed rectangular area on a PDF page.
///
/// Content is composed within the box boundaries.  The `PageFlow` cursor is
/// **not** affected — fixed elements are placed at absolute coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedBox {
    /// X position from left page edge in mm.
    pub x_mm: f64,
    /// Y position from bottom page edge in mm (printpdf bottom-left convention).
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    /// What to do when content exceeds the box height.
    pub overflow: OverflowPolicy,
    /// Optional visible border.
    pub border: Option<BoxBorder>,
    /// Optional background fill.
    pub background: Option<RgbColor>,
    /// Internal padding applied to all four sides in mm.
    pub padding_mm: f64,
    /// Z-order for overlapping fixed elements. Higher values render on top.
    /// Default: 0. Negative values render below the default layer.
    #[serde(default)]
    pub z_index: i32,
    /// Accessibility role for PDF/UA-2. None = Artifact (decorative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ua_role: Option<StructTag>,
    /// Alternative text when ua_role = Some(Figure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ua_alt: Option<String>,
}

impl Default for FixedBox {
    fn default() -> Self {
        Self {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 50.0,
            height_mm: 10.0,
            overflow: OverflowPolicy::Truncate,
            border: None,
            background: None,
            padding_mm: 2.0,
            z_index: 0,
            ua_role: None,
            ua_alt: None,
        }
    }
}

impl FixedBox {
    /// Sets the accessibility role for this Fixed Box (PDF/UA-2).
    pub fn role(mut self, tag: StructTag) -> Self {
        self.ua_role = Some(tag);
        self
    }

    /// Sets the alt text (requires role = Figure).
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.ua_alt = Some(text.into());
        self
    }

    /// Usable inner width after subtracting horizontal padding.
    pub fn inner_width_mm(&self) -> f64 {
        (self.width_mm - self.padding_mm * 2.0).max(0.0)
    }

    /// Usable inner height after subtracting vertical padding.
    pub fn inner_height_mm(&self) -> f64 {
        (self.height_mm - self.padding_mm * 2.0).max(0.0)
    }

    /// X coordinate of the inner content area (from left page edge).
    pub fn inner_x_mm(&self) -> f64 {
        self.x_mm + self.padding_mm
    }

    /// Y coordinate of the top of the inner content area (from bottom page edge).
    pub fn inner_y_top_mm(&self) -> f64 {
        self.y_mm + self.height_mm - self.padding_mm
    }
}

/// Policy for content that exceeds the fixed box height.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    /// Silently stop rendering lines that would fall below the box bottom.
    #[default]
    Truncate,
    /// Render with a PDF clipping path — content cut at the box boundary.
    Clip,
    /// Reduce font size until all content fits — useful for labels/badges.
    Shrink,
    /// Allow content to exceed the box — useful during development/debug.
    Overflow,
}

/// Border style for a fixed box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxBorder {
    pub width_mm: f64,
    pub color: RgbColor,
    pub style: BorderStyle,
}

/// Stroke pattern for borders and lines.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

use crate::richtext::marks::AppliedStyle;

use super::TextAlign;

/// A single styled text segment within a rendered line.
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub text: String,
    /// Horizontal offset from the left edge of the content area (mm).
    pub x_offset_mm: f64,
    pub style: AppliedStyle,
    pub font_size: f64,
    /// Extra space between each character in mm (from `TextRun::letter_spacing_mm`).
    pub letter_spacing_mm: f64,
}

/// A single rendered line, ready to be drawn on a PDF page.
#[derive(Debug, Clone)]
pub struct LineBox {
    pub segments: Vec<LineSegment>,
    /// Line height in mm (advance to next baseline).
    pub height_mm: f64,
    /// Total text width in mm (words + spaces, before alignment offset).
    pub width_mm: f64,
    pub alignment: TextAlign,
}

impl LineBox {
    pub fn total_width_mm(&self) -> f64 {
        self.width_mm
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.segments.iter().all(|s| s.text.trim().is_empty())
    }
}

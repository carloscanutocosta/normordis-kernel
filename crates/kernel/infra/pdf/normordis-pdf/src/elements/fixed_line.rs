use super::{Element, LayoutMode, RenderContext};
use crate::{
    layout::{BorderStyle, FixedBox, OverflowPolicy},
    styles::RgbColor,
};

/// A horizontal or vertical decorative line at a fixed position.
///
/// Useful for separators in form templates, certidões, and similar layouts.
/// Does not participate in `PageFlow`.
#[derive(Debug, Clone)]
pub struct FixedLineElement {
    pub x1_mm: f64,
    pub y1_mm: f64,
    pub x2_mm: f64,
    pub y2_mm: f64,
    /// Stroke width in mm.
    pub width_mm: f64,
    pub color: RgbColor,
    pub style: BorderStyle,
}

impl FixedLineElement {
    pub fn new(x1_mm: f64, y1_mm: f64, x2_mm: f64, y2_mm: f64, color: RgbColor) -> Self {
        Self {
            x1_mm,
            y1_mm,
            x2_mm,
            y2_mm,
            width_mm: 0.3,
            color,
            style: BorderStyle::Solid,
        }
    }
}

impl Element for FixedLineElement {
    fn layout_mode(&self) -> LayoutMode {
        let x = self.x1_mm.min(self.x2_mm);
        let y = self.y1_mm.min(self.y2_mm);
        let w = (self.x2_mm - self.x1_mm).abs().max(self.width_mm);
        let h = (self.y2_mm - self.y1_mm).abs().max(self.width_mm);

        LayoutMode::Fixed(FixedBox {
            x_mm: x,
            y_mm: y,
            width_mm: w,
            height_mm: h,
            overflow: OverflowPolicy::Overflow,
            border: None,
            background: None,
            padding_mm: 0.0,
            z_index: 0,
            ua_role: None,
            ua_alt: None,
        })
    }

    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        let width_pt = (self.width_mm * 72.0 / 25.4) as f32;
        ctx.backend.draw_line(
            self.x1_mm,
            self.y1_mm,
            self.x2_mm,
            self.y2_mm,
            width_pt,
            &self.color,
        )?;
        Ok(super::RenderResult::done())
    }
}

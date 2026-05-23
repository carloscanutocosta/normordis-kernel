use serde::{Deserialize, Serialize};

use crate::styles::RgbColor;

use super::{Element, RenderContext};

/// A blank vertical gap — the simplest element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacer {
    pub height_mm: f64,
}

impl Spacer {
    pub fn new(height_mm: f64) -> Self {
        Self { height_mm }
    }
}

impl Element for Spacer {
    fn estimated_height_mm(&self) -> f64 {
        self.height_mm
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        ctx.flow.advance(self.height_mm);
        Ok(super::RenderResult::done())
    }
}

/// Full-width horizontal rule — 3mm gap above and below, 0.3pt grey line.
pub struct HorizontalRuleElement;

impl Element for HorizontalRuleElement {
    fn estimated_height_mm(&self) -> f64 {
        6.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        ctx.flow.advance(3.0);
        let y = ctx.flow.cursor_y_mm;
        let x0 = ctx.layout.content_x_mm;
        let x1 = x0 + ctx.layout.content_width_mm;
        let gray = RgbColor::new(0.6, 0.6, 0.6);
        if ctx.ua_enabled() {
            ctx.backend.begin_artifact_content();
        }
        ctx.draw_hline(x0, x1, y, 0.5, &gray)?;
        if ctx.ua_enabled() {
            ctx.backend.end_tagged_content();
        }
        ctx.flow.advance(3.0);
        Ok(super::RenderResult::done())
    }
}

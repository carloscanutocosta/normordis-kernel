use super::{Element, RenderContext, RenderResult};
use crate::styles::RgbColor;

// Height consumed by the separator line + padding.
pub(crate) const FOOTNOTE_SEPARATOR_HEIGHT_MM: f64 = 2.5;
/// Footnote separator line thickness in mm (0.25 mm ≈ 0.7 pt).
pub const FOOTNOTE_SEPARATOR_THICKNESS_MM: f64 = 0.25;

// ── FootnoteMarkStyle ─────────────────────────────────────────────────────────

/// Superscript style for footnote reference marks.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FootnoteMarkStyle {
    /// Superscript arabic numeral (default).
    #[default]
    Numeric,
    /// Superscript letter: a b c …
    Alpha,
    /// Traditional symbols: * † ‡ §
    Symbol,
}

impl FootnoteMarkStyle {
    pub fn mark_text(self, number: u32) -> String {
        match self {
            FootnoteMarkStyle::Numeric => number.to_string(),
            FootnoteMarkStyle::Alpha => {
                let idx = ((number.saturating_sub(1)) % 26) as u8;
                char::from(b'a' + idx).to_string()
            }
            FootnoteMarkStyle::Symbol => {
                let symbols = ['*', '†', '‡', '§', '¶', '#'];
                let idx = ((number.saturating_sub(1)) % 6) as usize;
                symbols[idx].to_string()
            }
        }
    }
}

// ── FootnoteRef ───────────────────────────────────────────────────────────────

/// Inline footnote reference — renders as a superscript mark.
///
/// Use [`TextRun::footnote_ref`] to embed a reference inside paragraph text.
/// As a standalone flow element, it advances the cursor by a negligible amount.
#[derive(Debug, Clone)]
pub struct FootnoteRef {
    pub number: u32,
    pub mark_style: FootnoteMarkStyle,
}

impl FootnoteRef {
    pub fn new(number: u32) -> Self {
        Self { number, mark_style: FootnoteMarkStyle::Numeric }
    }

    pub fn with_style(mut self, style: FootnoteMarkStyle) -> Self {
        self.mark_style = style;
        self
    }

    pub fn mark_text(&self) -> String {
        self.mark_style.mark_text(self.number)
    }
}

impl Element for FootnoteRef {
    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let mark = self.mark_text();
        let p = crate::elements::paragraph::Paragraph::new(mark)
            .font_size(ctx.style.font_size_small * 0.75);
        p.render(ctx)
    }
}

// ── FootnoteAccumulator ───────────────────────────────────────────────────────

/// Collects footnote content to be rendered at the bottom of each page.
#[derive(Default)]
pub struct FootnoteAccumulator {
    /// (number, content_texts) pairs waiting to be rendered.
    pub pending: Vec<(u32, Vec<String>)>,
    /// Total height reserved at the bottom for all pending footnotes (mm).
    pub reserved_height_mm: f64,
}

impl FootnoteAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve space for a footnote. Returns the height reserved.
    pub fn reserve(&mut self, number: u32, texts: Vec<String>, line_height_mm: f64) -> f64 {
        let n_lines = texts.iter().map(|t| {
            ((t.len() as f64 / 60.0).ceil() as usize).max(1)
        }).sum::<usize>();
        let text_h = n_lines as f64 * line_height_mm;
        let extra = if self.pending.is_empty() { FOOTNOTE_SEPARATOR_HEIGHT_MM } else { 0.0 };
        let height = extra + text_h;
        self.reserved_height_mm += height;
        self.pending.push((number, texts));
        height
    }

    /// Render all pending footnotes at the reserved area at the bottom of the page.
    pub fn render_pending(&mut self, ctx: &mut RenderContext) -> crate::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }

        let footnote_top_y = ctx.layout.margin_bottom_mm + self.reserved_height_mm;
        ctx.flow.cursor_y_mm = footnote_top_y;

        render_separator(ctx);

        ctx.flow.cursor_y_mm -= FOOTNOTE_SEPARATOR_HEIGHT_MM;

        let pending = std::mem::take(&mut self.pending);
        for (number, texts) in &pending {
            for (i, text) in texts.iter().enumerate() {
                let line = if i == 0 {
                    format!("{number}. {text}")
                } else {
                    format!("    {text}")
                };
                let p = crate::elements::paragraph::Paragraph::new(line)
                    .style("footnote");
                p.render(ctx)?;
            }
        }

        self.pending.clear();
        self.reserved_height_mm = 0.0;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.reserved_height_mm = 0.0;
    }
}

fn render_separator(ctx: &mut RenderContext) {
    let x0 = ctx.layout.content_x_mm;
    let x1 = x0 + ctx.layout.content_width_mm / 3.0;
    let y = ctx.flow.cursor_y_mm - FOOTNOTE_SEPARATOR_HEIGHT_MM * 0.5;
    let width_pt = (FOOTNOTE_SEPARATOR_THICKNESS_MM / 25.4 * 72.0) as f32;
    let color = RgbColor { r: 0.4, g: 0.4, b: 0.4 };
    if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
    let _ = ctx.backend.draw_line(x0, y, x1, y, width_pt, &color);
    if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
}

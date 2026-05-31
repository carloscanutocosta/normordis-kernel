use super::{paragraph::ParagraphContent, Element, LayoutMode, RenderContext};
use crate::{
    layout::{FixedBox, OverflowPolicy, TextAlign},
    richtext::marks::TextRun,
};

/// Vertical alignment of content within a `FixedTextBox` or table cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

/// A text element rendered inside a fixed rectangular area.
///
/// Does not participate in `PageFlow` — `layout_mode()` returns
/// `LayoutMode::Fixed`.
#[derive(Debug, Clone)]
pub struct FixedTextBox {
    pub text_box: FixedBox,
    pub content: ParagraphContent,
    pub alignment: TextAlign,
    /// Override the body font size.  `None` uses `DocumentStyle.font_size_body`.
    pub font_size: Option<f64>,
    pub vertical_align: VerticalAlign,
}

impl FixedTextBox {
    /// Y starting position of the first text line given the rendered content height.
    pub fn content_y_start_mm(&self, content_height_mm: f64) -> f64 {
        let inner_h = self.text_box.inner_height_mm();
        match self.vertical_align {
            VerticalAlign::Top => self.text_box.inner_y_top_mm(),
            VerticalAlign::Middle => {
                self.text_box.inner_y_top_mm() - ((inner_h - content_height_mm) / 2.0).max(0.0)
            }
            VerticalAlign::Bottom => {
                self.text_box.y_mm + self.text_box.padding_mm + content_height_mm
            }
        }
    }

    /// Effective font size after applying `Shrink` overflow policy.
    pub fn effective_font_size(&self, ctx: &RenderContext) -> f64 {
        let base_fs = self.font_size.unwrap_or(ctx.style.font_size_body);
        if self.text_box.overflow != OverflowPolicy::Shrink {
            return base_fs;
        }
        let inner_w = self.text_box.inner_width_mm().max(1.0);
        let inner_h = self.text_box.inner_height_mm();
        let runs = self.content_runs();
        let mut fs = base_fs;
        loop {
            let r =
                ctx.layout_engine
                    .layout_runs(&ctx.fonts, &runs, inner_w, self.alignment, fs, &[]);
            if r.total_height_mm <= inner_h || fs <= 6.0 {
                return fs;
            }
            fs = (fs - 0.5).max(6.0);
        }
    }

    fn content_runs(&self) -> Vec<TextRun> {
        match &self.content {
            ParagraphContent::Plain(text) => vec![TextRun::plain(text)],
            ParagraphContent::Runs(runs) => runs.clone(),
        }
    }
}

impl Element for FixedTextBox {
    fn layout_mode(&self) -> LayoutMode {
        LayoutMode::Fixed(self.text_box.clone())
    }

    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        let inner_w = self.text_box.inner_width_mm().max(1.0);
        let runs = self.content_runs();
        let effective_fs = self.effective_font_size(ctx);
        let ua = ctx.ua_config.enabled;

        // UA-2: tag or mark as Artifact based on ua_role
        if ua {
            match &self.text_box.ua_role {
                Some(tag) => {
                    let mcid = ctx.ua_tag_element(tag.clone(), self.text_box.ua_alt.clone());
                    ctx.backend
                        .begin_tagged_content(tag.pdf_name().as_bytes(), mcid);
                }
                None => {
                    ctx.backend.begin_artifact_content();
                }
            }
        }

        let result = ctx.layout_engine.layout_runs(
            &ctx.fonts,
            &runs,
            inner_w,
            self.alignment,
            effective_fs,
            &[],
        );
        let y_start = self.content_y_start_mm(result.total_height_mm);

        let tc = ctx.style.text_color.clone();
        let line_h = ctx.layout_engine.line_height_mm(&ctx.fonts, effective_fs);
        let bottom_y = self.text_box.y_mm + self.text_box.padding_mm;
        let mut y = y_start;

        for line in &result.lines {
            if self.text_box.overflow == OverflowPolicy::Truncate && y - line_h < bottom_y {
                break;
            }
            for seg in &line.segments {
                if seg.text.is_empty() {
                    continue;
                }
                let Some(font_ref) = ctx.get_font_ref(seg.style.bold, seg.style.italic) else {
                    continue;
                };
                let x = self.text_box.inner_x_mm() + seg.x_offset_mm;
                ctx.draw_text(&seg.text, x, y, effective_fs, font_ref, &tc)?;
            }
            y -= line_h;
        }

        if ua {
            ctx.backend.end_tagged_content();
        }
        Ok(super::RenderResult::done())
    }
}

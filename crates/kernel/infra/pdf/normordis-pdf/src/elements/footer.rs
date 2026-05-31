use serde::{Deserialize, Serialize};

use super::{Element, RenderContext};

/// Page footer rendered at the bottom of every page.
///
/// Supports left, center, and right text columns. All text fields accept
/// runtime fields: `{{page}}`, `{{total_pages}}`, `{{today}}`, `{{now}}`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageFooter {
    /// Left column — typically the document reference.
    pub left_text: Option<String>,
    /// Centre column — typically the entity name.
    pub center_text: Option<String>,
    /// Right column — typically page number or date.
    pub right_text: Option<String>,
    /// Deprecated: use `right_text = "{{page}} / {{total_pages}}"`.
    pub show_page_number: bool,
    /// Deprecated: use `left_text = "{{today}}"`.
    pub show_date: bool,
}

impl PageFooter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience constructor — shows page number only.
    pub fn with_page_numbers() -> Self {
        Self {
            show_page_number: true,
            ..Self::new()
        }
    }

    pub fn left(mut self, text: impl Into<String>) -> Self {
        self.left_text = Some(text.into());
        self
    }

    pub fn center(mut self, text: impl Into<String>) -> Self {
        self.center_text = Some(text.into());
        self
    }

    pub fn right(mut self, text: impl Into<String>) -> Self {
        self.right_text = Some(text.into());
        self
    }
}

impl Element for PageFooter {
    fn estimated_height_mm(&self) -> f64 {
        // separator line (0.5mm) + gap (1.5mm) + text row (~5mm)
        7.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        use crate::template::resolver::{resolve_runtime_fields, RuntimeContext};

        let ua = ctx.ua_config.enabled;
        if ua {
            ctx.backend.begin_artifact_content();
        }

        let y = ctx.flow.cursor_y_mm;
        let x1 = ctx.layout.content_x_mm;
        let x2 = x1 + ctx.layout.content_width_mm;
        let tc = ctx.style.text_color.clone();
        let rt = RuntimeContext::new(ctx.page_number, ctx.total_pages);
        let fs = ctx.style.font_size_small;
        let content_width = ctx.layout.content_width_mm;

        // Separator line
        ctx.draw_hline(x1, x2, y, 0.4, &tc)?;

        let Some(font_ref) = ctx.get_font_ref(false, false) else {
            ctx.flow.advance(self.estimated_height_mm());
            return Ok(super::RenderResult::done());
        };

        let text_y = y - 2.5;

        if let Some(ref txt) = self.left_text {
            let resolved = resolve_runtime_fields(txt, &rt);
            ctx.draw_text(&resolved, x1, text_y, fs, font_ref, &tc)?;
        }

        if let Some(ref txt) = self.center_text {
            let resolved = resolve_runtime_fields(txt, &rt);
            let w = ctx
                .fonts
                .get_default()
                .measure_text_mm(&resolved, fs, false, false);
            let cx = x1 + content_width / 2.0 - w / 2.0;
            ctx.draw_text(&resolved, cx, text_y, fs, font_ref, &tc)?;
        }

        if let Some(ref txt) = self.right_text {
            let resolved = resolve_runtime_fields(txt, &rt);
            let w = ctx
                .fonts
                .get_default()
                .measure_text_mm(&resolved, fs, false, false);
            let rx = x1 + content_width - w;
            ctx.draw_text(&resolved, rx, text_y, fs, font_ref, &tc)?;
        } else if self.show_page_number {
            let num = ctx.page_number.to_string();
            let w = ctx
                .fonts
                .get_default()
                .measure_text_mm(&num, fs, false, false);
            let rx = x1 + content_width - w;
            ctx.draw_text(&num, rx, text_y, fs, font_ref, &tc)?;
        }

        ctx.flow.advance(self.estimated_height_mm());
        if ua {
            ctx.backend.end_tagged_content();
        }
        Ok(super::RenderResult::done())
    }
}

// ── SectionedFooter ───────────────────────────────────────────────────────────

/// Footer configuration with per-section variants.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::{PageFooter, SectionedFooter};
///
/// let footer = SectionedFooter::new()
///     .all_pages(
///         PageFooter::new()
///             .left("REF/2026/042")
///             .right("{{page}} / {{total_pages}}"),
///     );
/// ```
#[derive(Debug, Clone, Default)]
pub struct SectionedFooter {
    pub first_page: Option<PageFooter>,
    pub odd_pages: Option<PageFooter>,
    pub even_pages: Option<PageFooter>,
}

impl SectionedFooter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn first_page(mut self, f: PageFooter) -> Self {
        self.first_page = Some(f);
        self
    }

    pub fn odd_pages(mut self, f: PageFooter) -> Self {
        self.odd_pages = Some(f);
        self
    }

    pub fn even_pages(mut self, f: PageFooter) -> Self {
        self.even_pages = Some(f);
        self
    }

    /// Sets the same footer for all pages (odd, even, and first).
    pub fn all_pages(mut self, f: PageFooter) -> Self {
        self.first_page = Some(f.clone());
        self.odd_pages = Some(f.clone());
        self.even_pages = Some(f);
        self
    }

    /// Resolves which footer to render for a given page number (1-based).
    pub fn resolve(&self, page_number: u32) -> Option<&PageFooter> {
        if page_number == 1 {
            if let Some(ref f) = self.first_page {
                return Some(f);
            }
        }
        if page_number.is_multiple_of(2) {
            if let Some(ref f) = self.even_pages {
                return Some(f);
            }
        }
        self.odd_pages.as_ref()
    }
}

use serde::{Deserialize, Serialize};

use super::{Element, RenderContext};
use crate::styles::{RgbColor, StyleResolver};

/// Institutional document header rendered at the top of the first page.
///
/// Includes entity name, document title, optional subtitle/logo/reference/date,
/// and a horizontal separator line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionalHeader {
    /// Full name of the issuing entity (e.g. "Câmara Municipal de Lisboa").
    pub entity_name: String,
    pub document_title: String,
    pub document_subtitle: Option<String>,
    /// Raw PNG or JPEG bytes of the entity logo.
    pub logo: Option<Vec<u8>>,
    /// Document reference number or code.
    pub reference: Option<String>,
    /// Issue date string.
    pub date: Option<String>,
}

impl InstitutionalHeader {
    pub fn new(
        entity_name: impl Into<String>,
        document_title: impl Into<String>,
    ) -> Self {
        Self {
            entity_name: entity_name.into(),
            document_title: document_title.into(),
            document_subtitle: None,
            logo: None,
            reference: None,
            date: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.document_subtitle = Some(subtitle.into());
        self
    }

    pub fn with_logo(mut self, bytes: Vec<u8>) -> Self {
        self.logo = Some(bytes);
        self
    }

    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    pub fn with_date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }
}

impl Element for InstitutionalHeader {
    fn estimated_height_mm(&self) -> f64 {
        if self.logo.is_some() { 35.0 } else { 25.0 }
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        let ua = ctx.ua_config.enabled;
        if ua { ctx.backend.begin_artifact_content(); }

        let text_color = ctx.style.text_color.clone();
        let muted_color = RgbColor { r: 0.45, g: 0.45, b: 0.45 };
        let sep_color = ctx.style.primary_color.clone();
        let resolver = StyleResolver::new(&ctx.style.named_styles, &ctx.style);
        let font_family = resolver.resolve("normal").map(|r| r.font_family).unwrap_or_else(|_| "LiberationSans".to_string());

        let content_x = ctx.layout.content_x_mm;
        let content_w = ctx.layout.content_width_mm;
        let right_edge = content_x + content_w;

        // ── Row 1: entity name (bold 13pt, left) + date (9pt, right) ────────
        {
            let fs_main = 13.0_f64;
            let fs_meta = 9.0_f64;
            let y = ctx.flow.cursor_y_mm;
            if let Some(fref) = ctx.get_font_ref(true, false) {
                ctx.draw_text(&self.entity_name, content_x, y, fs_main, fref, &text_color)?;
            }
            if let Some(ref date) = self.date {
                let dw = ctx.fonts.get_family(&font_family)
                    .measure_text_mm(date, fs_meta, false, false);
                if let Some(fref) = ctx.get_font_ref(false, false) {
                    ctx.draw_text(date, right_edge - dw, y, fs_meta, fref, &muted_color)?;
                }
            }
            let lh = ctx.layout_engine.line_height_mm(&ctx.fonts, fs_main);
            ctx.flow.advance(lh + 1.5);
        }

        // ── Row 2: document title (bold 11pt, left) + reference (9pt, right) ─
        {
            let fs_main = 11.0_f64;
            let fs_meta = 9.0_f64;
            let y = ctx.flow.cursor_y_mm;
            if let Some(fref) = ctx.get_font_ref(true, false) {
                ctx.draw_text(&self.document_title, content_x, y, fs_main, fref, &text_color)?;
            }
            if let Some(ref reference) = self.reference {
                let rw = ctx.fonts.get_family(&font_family)
                    .measure_text_mm(reference, fs_meta, false, false);
                if let Some(fref) = ctx.get_font_ref(false, false) {
                    ctx.draw_text(reference, right_edge - rw, y, fs_meta, fref, &muted_color)?;
                }
            }
            let lh = ctx.layout_engine.line_height_mm(&ctx.fonts, fs_main);
            ctx.flow.advance(lh + 1.5);
        }

        // ── Row 3: subtitle (italic 9pt, left) ──────────────────────────────
        if let Some(ref subtitle) = self.document_subtitle {
            let fs = 9.0_f64;
            let y = ctx.flow.cursor_y_mm;
            if let Some(fref) = ctx.get_font_ref(false, true) {
                ctx.draw_text(subtitle, content_x, y, fs, fref, &muted_color)?;
            }
            let lh = ctx.layout_engine.line_height_mm(&ctx.fonts, fs);
            ctx.flow.advance(lh + 1.5);
        }

        // ── Separator line ───────────────────────────────────────────────────
        ctx.flow.advance(1.0);
        let sep_y = ctx.flow.cursor_y_mm;
        ctx.backend.draw_line(content_x, sep_y, right_edge, sep_y, 0.75, &sep_color)?;
        ctx.flow.advance(3.0);

        if ua { ctx.backend.end_tagged_content(); }
        Ok(super::RenderResult::done())
    }
}

// ── SectionedHeader ───────────────────────────────────────────────────────────

/// Header configuration with per-section variants.
///
/// Allows different headers for the first page, odd pages, and even pages,
/// matching the Word document model.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::{SectionedHeader, InstitutionalHeader};
///
/// let header = SectionedHeader::new()
///     .first_page(
///         InstitutionalHeader::new("Câmara Municipal", "Ofício")
///             .with_reference("REF/2026/001")
///     )
///     .odd_pages(
///         InstitutionalHeader::new("Câmara Municipal", "Ofício — continuação")
///     );
/// ```
#[derive(Debug, Clone, Default)]
pub struct SectionedHeader {
    pub first_page: Option<InstitutionalHeader>,
    pub odd_pages: Option<InstitutionalHeader>,
    pub even_pages: Option<InstitutionalHeader>,
}

impl SectionedHeader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the header for the first page only.
    pub fn first_page(mut self, h: InstitutionalHeader) -> Self {
        self.first_page = Some(h);
        self
    }

    /// Sets the header for odd pages (1, 3, 5, …).
    /// Also used as fallback when `even_pages` is not set.
    pub fn odd_pages(mut self, h: InstitutionalHeader) -> Self {
        self.odd_pages = Some(h);
        self
    }

    /// Sets the header for even pages (2, 4, 6, …).
    pub fn even_pages(mut self, h: InstitutionalHeader) -> Self {
        self.even_pages = Some(h);
        self
    }

    /// Resolves which header to render for a given page number (1-based).
    ///
    /// Resolution order:
    /// - page == 1 AND first_page.is_some() → first_page
    /// - page is even AND even_pages.is_some() → even_pages
    /// - odd_pages.is_some() → odd_pages
    /// - None (no header for this page)
    pub fn resolve(&self, page_number: u32) -> Option<&InstitutionalHeader> {
        if page_number == 1 {
            if let Some(ref h) = self.first_page {
                return Some(h);
            }
        }
        if page_number.is_multiple_of(2) {
            if let Some(ref h) = self.even_pages {
                return Some(h);
            }
        }
        self.odd_pages.as_ref()
    }
}

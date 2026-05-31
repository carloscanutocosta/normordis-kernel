pub mod fixed;
pub mod fixed_image;
pub mod fixed_line;
pub mod fixed_text;
pub mod footer;
pub mod footnote;
pub mod form;
pub mod header;
pub mod image;
pub mod list;
pub mod page_break;
pub mod paragraph;
pub mod section;
pub mod section_break;
pub mod spacer;
pub mod table;
pub mod toc;

use std::collections::HashMap;

use crate::{
    backend::{FontRef, PdfBackend},
    compliance::ua::{AccessibilityConfig, StructTag, StructureTree},
    fonts::FontRegistry,
    layout::{FixedBox, GlyphUsageTracker, PageFlow, TextLayoutEngine},
    page::PageLayout,
    styles::{DocumentStyle, RgbColor},
};

/// Determines how an element interacts with the page flow.
#[derive(Debug, Clone)]
pub enum LayoutMode {
    /// Element participates in vertical flow.
    Flow,
    /// Element is placed at absolute coordinates; does not affect the cursor.
    Fixed(FixedBox),
}

/// Result of a single render call on an element.
#[derive(Debug, Clone)]
pub struct RenderResult {
    pub has_more: bool,
}

impl RenderResult {
    pub fn done() -> Self {
        Self { has_more: false }
    }
    pub fn more() -> Self {
        Self { has_more: true }
    }
}

/// Context passed to every element during rendering.
///
/// Elements call `ctx.backend.draw_*()` to emit drawing operations and advance
/// `ctx.flow.cursor_y_mm` via `ctx.flow.advance()`.
pub struct RenderContext {
    /// PDF backend — all drawing calls go through this.
    pub backend: Box<dyn PdfBackend>,
    /// Font map: `"family::variant"` → `FontRef`.
    pub font_map: HashMap<String, FontRef>,
    /// Vertical cursor and page-break state.
    pub flow: PageFlow,
    /// Calculated page layout for the current document style.
    pub layout: PageLayout,
    /// Text measurement and line-wrapping engine.
    pub layout_engine: TextLayoutEngine,
    /// Document-wide style settings.
    pub style: DocumentStyle,
    /// Loaded font registry.
    pub fonts: FontRegistry,
    /// Set to `true` by `PageBreakElement`.
    pub force_page_break: bool,
    /// Name of the default font family.
    pub default_font_family: String,
    /// Current page number (1-based).
    pub page_number: u32,
    /// Total pages (set after first-pass page count).
    pub total_pages: u32,
    /// Index of the first item to render on continuation pages.
    pub resume_index: usize,
    /// Glyph usage collector for Eixo B subsetting.
    pub glyph_tracker: GlyphUsageTracker,
    /// Vertical space (mm) reserved at the bottom for footnotes.
    pub reserved_footnotes_mm: f64,
    /// Accessibility configuration for PDF/UA-2.
    pub ua_config: AccessibilityConfig,
    /// Structure tree events accumulated during rendering.
    pub ua_events: StructureTree,
    /// Current marked content identifier (resets on each page).
    pub mcid_counter: u32,
    /// Last heading level rendered (for UA-2 level-skip detection).
    pub last_heading_level: Option<u8>,
}

impl RenderContext {
    pub fn reset_resume(&mut self) {
        self.resume_index = 0;
    }

    /// Returns true if PDF/UA-2 accessibility mode is active.
    pub fn ua_enabled(&self) -> bool {
        self.ua_config.enabled
    }

    /// Allocates the next MCID for the current page and returns it.
    pub fn next_mcid(&mut self) -> u32 {
        let mcid = self.mcid_counter;
        self.mcid_counter += 1;
        mcid
    }

    /// Records a single-element structure event (BeginGroup + ContentRef + EndGroup).
    pub fn ua_tag_element(&mut self, tag: StructTag, alt: Option<String>) -> u32 {
        let mcid = self.next_mcid();
        let page_idx = self.page_number as usize - 1;
        self.ua_events.begin_group(tag, alt);
        self.ua_events.add_content_ref(mcid, page_idx);
        self.ua_events.end_group();
        mcid
    }

    /// Opens a structure element group (must be closed with ua_end_group).
    pub fn ua_begin_group(&mut self, tag: StructTag, alt: Option<String>) {
        self.ua_events.begin_group(tag, alt);
    }

    /// Adds a content ref to the current structure element group.
    pub fn ua_content_ref(&mut self, mcid: u32) {
        let page_idx = self.page_number as usize - 1;
        self.ua_events.add_content_ref(mcid, page_idx);
    }

    /// Closes the current structure element group.
    pub fn ua_end_group(&mut self) {
        self.ua_events.end_group();
    }

    /// Returns the `FontRef` for the default family with the given style.
    pub fn get_font_ref(&self, bold: bool, italic: bool) -> Option<FontRef> {
        self.get_font_ref_for(&self.default_font_family.clone(), bold, italic)
    }

    /// Returns the `FontRef` for a specific family with the given style.
    pub fn get_font_ref_for(&self, family: &str, bold: bool, italic: bool) -> Option<FontRef> {
        if bold && italic {
            self.font_map
                .get(&format!("{family}::bold_italic"))
                .or_else(|| self.font_map.get(&format!("{family}::bold")))
                .or_else(|| self.font_map.get(&format!("{family}::italic")))
                .or_else(|| self.font_map.get(&format!("{family}::regular")))
                .copied()
        } else if bold {
            self.font_map
                .get(&format!("{family}::bold"))
                .or_else(|| self.font_map.get(&format!("{family}::regular")))
                .copied()
        } else if italic {
            self.font_map
                .get(&format!("{family}::italic"))
                .or_else(|| self.font_map.get(&format!("{family}::regular")))
                .copied()
        } else {
            self.font_map.get(&format!("{family}::regular")).copied()
        }
    }

    /// Convenience: draw text, forwarding to the backend.
    pub fn draw_text(
        &mut self,
        text: &str,
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
    ) -> crate::Result<()> {
        self.backend
            .draw_text(text, x_mm, y_mm, font_size_pt, font_ref, color, 0.0)
    }

    /// Convenience: draw text with letter spacing.
    pub fn draw_text_spaced(
        &mut self,
        text: &str,
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
        letter_spacing_pt: f32,
    ) -> crate::Result<()> {
        self.backend.draw_text(
            text,
            x_mm,
            y_mm,
            font_size_pt,
            font_ref,
            color,
            letter_spacing_pt,
        )
    }

    /// Convenience: draw a horizontal line.
    pub fn draw_hline(
        &mut self,
        x0_mm: f64,
        x1_mm: f64,
        y_mm: f64,
        width_pt: f32,
        color: &RgbColor,
    ) -> crate::Result<()> {
        self.backend
            .draw_line(x0_mm, y_mm, x1_mm, y_mm, width_pt, color)
    }

    /// Convenience: draw a vertical line.
    pub fn draw_vline(
        &mut self,
        x_mm: f64,
        y0_mm: f64,
        y1_mm: f64,
        width_pt: f32,
        color: &RgbColor,
    ) -> crate::Result<()> {
        self.backend
            .draw_line(x_mm, y0_mm, x_mm, y1_mm, width_pt, color)
    }
}

/// Trait implemented by every document element.
pub trait Element {
    fn layout_mode(&self) -> LayoutMode {
        LayoutMode::Flow
    }

    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn as_section_info(&self) -> Option<(u8, &str)> {
        None
    }

    fn inject_toc_entries(&mut self, _entries: &[crate::elements::toc::TocEntry]) {}

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult>;
}

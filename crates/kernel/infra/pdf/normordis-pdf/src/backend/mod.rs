pub mod pdf_writer_backend;

use crate::{fonts::ShapedGlyph, styles::RgbColor};

/// Abstraction layer over the PDF backend.
///
/// Elements call methods on PdfBackend and never interact with pdf-writer
/// directly. This allows future backend changes without touching element code.
pub trait PdfBackend {
    /// Draw text at an absolute position (bottom-left origin, mm).
    fn draw_text(
        &mut self,
        text: &str,
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
        letter_spacing_pt: f32,
    ) -> crate::Result<()>;

    /// Draw text using pre-shaped glyph IDs (for GPOS-aware rendering).
    fn draw_shaped_glyphs(
        &mut self,
        glyphs: &[ShapedGlyph],
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
    ) -> crate::Result<()>;

    /// Draw a line between two points. `width_pt` is stroke width in points.
    fn draw_line(
        &mut self,
        x1_mm: f64,
        y1_mm: f64,
        x2_mm: f64,
        y2_mm: f64,
        width_pt: f32,
        color: &RgbColor,
    ) -> crate::Result<()>;

    /// Draw a filled rectangle. `(x_mm, y_mm)` is the bottom-left corner.
    fn draw_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        fill: &RgbColor,
    ) -> crate::Result<()>;

    /// Draw a filled+stroked rectangle (for form field placeholders).
    fn draw_rect_stroked(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        fill: &RgbColor,
        stroke: &RgbColor,
        stroke_pt: f32,
    ) -> crate::Result<()>;

    /// Draw rotated text centered at `(cx_mm, cy_mm)`.
    /// `half_width_mm` is used to offset the text so it centers horizontally.
    fn draw_text_rotated(
        &mut self,
        text: &str,
        cx_mm: f64,
        cy_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
        angle_deg: f64,
        half_width_mm: f64,
    ) -> crate::Result<()>;

    /// Flush current page and start a new one with given dimensions.
    fn new_page(&mut self, width_mm: f64, height_mm: f64) -> crate::Result<()>;

    /// Finalise document and return PDF bytes.
    fn finish(&mut self) -> crate::Result<Vec<u8>>;

    /// Save graphics state (`q`).
    fn save_state(&mut self);

    /// Restore graphics state (`Q`).
    fn restore_state(&mut self);

    /// Set fill and stroke opacity via ExtGState (0.0 = transparent, 1.0 = opaque).
    ///
    /// The backend creates or reuses a cached `ExtGState` for the given level
    /// and emits the `gs` operator in the content stream.
    fn set_opacity(&mut self, opacity: f64) -> crate::Result<()>;

    /// Reset fill and stroke opacity to fully opaque (1.0).
    fn reset_opacity(&mut self);

    // ── PDF/UA-2 tagged content ──────────────────────────────────────────────

    /// Begin a tagged content sequence (`/tag <</MCID n>> BDC`). No-op by default.
    fn begin_tagged_content(&mut self, _tag_name: &[u8], _mcid: u32) {}

    /// End a tagged content sequence (`EMC`). No-op by default.
    fn end_tagged_content(&mut self) {}

    /// Begin an artifact content sequence (`/Artifact BMC`). No-op by default.
    fn begin_artifact_content(&mut self) {}

    /// Return the 0-based index of the current page. Used for structure tree refs.
    fn current_page_idx(&self) -> usize { 0 }

    /// Write the structure tree (called after rendering, before finish()).
    ///
    /// Takes the flat event list and the document language tag (BCP 47).
    fn write_structure_tree(
        &mut self,
        _events: &[crate::compliance::ua::StructEvent],
        _lang: &str,
    ) {}

    /// Record a section heading for the PDF outline (bookmarks panel).
    ///
    /// `level` is 1–3; `page_idx` is 0-based; `y_mm` is the baseline y
    /// coordinate from the page bottom (PDF coordinate space, in mm).
    fn add_outline_entry(&mut self, _title: &str, _level: u8, _page_idx: usize, _y_mm: f64) {}

    /// Queue an invisible GoTo link annotation for the current page.
    ///
    /// The hit-rect `(x1_mm, y1_mm, x2_mm, y2_mm)` is in mm, PDF coordinates
    /// (bottom-left origin, y1 < y2). `dest_title` is the heading text to look
    /// up in the outline table; `dest_page_estimate` (1-based) is used as a
    /// fallback if the title is not found when the doc is finalised.
    fn add_link_annotation(
        &mut self,
        _x1_mm: f64,
        _y1_mm: f64,
        _x2_mm: f64,
        _y2_mm: f64,
        _dest_title: &str,
        _dest_page_estimate: u32,
    ) {}
}

/// Opaque reference to an embedded font variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontRef(pub u32);

/// Opaque reference to an embedded image XObject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageRef(pub u32);

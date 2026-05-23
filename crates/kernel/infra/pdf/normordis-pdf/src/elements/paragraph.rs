use serde::{Deserialize, Serialize};

use super::{Element, RenderContext, RenderResult};
use crate::{
    compliance::ua::StructTag,
    layout::{DecorationLine, TabStop, TextAlign},
    richtext::marks::AppliedStyle,
    styles::{RgbColor, StyleResolver},
};

// Re-export TextRun from richtext::marks so that existing callers that import
// it via `elements::paragraph::TextRun` continue to work.
pub use crate::richtext::marks::TextRun;

// ── ParagraphBorder ───────────────────────────────────────────────────────────

/// Border drawn around a paragraph block (w:pBdr equivalent).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParagraphBorder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<DecorationLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<DecorationLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<DecorationLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<DecorationLine>,
    /// Inner padding between the text and the border lines in mm.
    #[serde(default)]
    pub padding_mm: f64,
}

impl ParagraphBorder {
    /// Box border: all four sides use the same decoration line.
    pub fn box_border(line: DecorationLine) -> Self {
        Self {
            top: Some(line.clone()),
            bottom: Some(line.clone()),
            left: Some(line.clone()),
            right: Some(line),
            padding_mm: 1.0,
        }
    }
}

// ── ParagraphContent ──────────────────────────────────────────────────────────

/// Content of a paragraph — either a single plain string or a sequence of
/// formatted runs (the normal case when converting from NCRTF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParagraphContent {
    Plain(String),
    Runs(Vec<TextRun>),
}

impl ParagraphContent {
    pub fn as_plain_text(&self) -> String {
        match self {
            ParagraphContent::Plain(s) => s.clone(),
            ParagraphContent::Runs(runs) => runs.iter().map(|r| r.text.as_str()).collect(),
        }
    }
}

/// A block of body text with optional formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub content: ParagraphContent,
    /// Override the body font size; `None` uses `DocumentStyle.font_size_body` or resolved style.
    pub font_size: Option<f64>,
    /// Bold override for `Plain` content variant.
    pub bold: bool,
    /// Italic override for `Plain` content variant.
    pub italic: bool,
    pub alignment: TextAlign,
    /// Left indent in mm. Positive values indent from the left margin.
    #[serde(default)]
    pub indent_left_mm: f64,
    /// Right indent in mm. Positive values indent from the right margin.
    #[serde(default)]
    pub indent_right_mm: f64,
    /// First-line indent in mm. Positive = indent in; negative = hanging indent.
    #[serde(default)]
    pub indent_first_line_mm: f64,
    /// Named style reference. When set, resolved style provides defaults for
    /// any field not explicitly overridden on this `Paragraph`.
    #[serde(default)]
    pub style_ref: Option<String>,
    /// Explicit spacing before this paragraph in mm. Overrides the named style value.
    #[serde(default)]
    pub space_before_mm: Option<f64>,
    /// Explicit spacing after this paragraph in mm. Overrides the named style value.
    #[serde(default)]
    pub space_after_mm: Option<f64>,
    /// Tab stops for this paragraph. Empty = no custom tab stops.
    #[serde(default)]
    pub tab_stops: Vec<TabStop>,
    /// Optional border drawn around the paragraph block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<ParagraphBorder>,
    /// Optional background fill colour for the paragraph block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<RgbColor>,
    /// Keep this paragraph on the same page as the next one.
    #[serde(default)]
    pub keep_next: bool,
    /// Keep all lines of this paragraph together on the same page.
    #[serde(default)]
    pub keep_lines: bool,
}

impl Paragraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            content: ParagraphContent::Plain(text.into()),
            font_size: None,
            bold: false,
            italic: false,
            alignment: TextAlign::Justify,
            indent_left_mm: 0.0,
            indent_right_mm: 0.0,
            indent_first_line_mm: 0.0,
            style_ref: None,
            space_before_mm: None,
            space_after_mm: None,
            tab_stops: Vec::new(),
            border: None,
            background: None,
            keep_next: false,
            keep_lines: false,
        }
    }

    pub fn from_runs(runs: Vec<TextRun>, alignment: TextAlign, font_size: Option<f64>) -> Self {
        Self {
            content: ParagraphContent::Runs(runs),
            font_size,
            bold: false,
            italic: false,
            alignment,
            indent_left_mm: 0.0,
            indent_right_mm: 0.0,
            indent_first_line_mm: 0.0,
            style_ref: None,
            space_before_mm: None,
            space_after_mm: None,
            tab_stops: Vec::new(),
            border: None,
            background: None,
            keep_next: false,
            keep_lines: false,
        }
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn align(mut self, alignment: TextAlign) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn indent_left(mut self, mm: f64) -> Self {
        self.indent_left_mm = mm;
        self
    }

    pub fn indent_right(mut self, mm: f64) -> Self {
        self.indent_right_mm = mm;
        self
    }

    pub fn indent_first_line(mut self, mm: f64) -> Self {
        self.indent_first_line_mm = mm;
        self
    }

    /// Apply a named style. The style provides defaults for any fields not
    /// explicitly set on this `Paragraph`.
    pub fn style(mut self, name: impl Into<String>) -> Self {
        self.style_ref = Some(name.into());
        self
    }

    /// Override space before this paragraph (mm). Suppressed at top of page.
    pub fn space_before(mut self, mm: f64) -> Self {
        self.space_before_mm = Some(mm);
        self
    }

    /// Override space after this paragraph (mm).
    pub fn space_after(mut self, mm: f64) -> Self {
        self.space_after_mm = Some(mm);
        self
    }

    /// Add a tab stop.
    pub fn tab_stop(mut self, stop: TabStop) -> Self {
        self.tab_stops.push(stop);
        self
    }

    /// Add a border around the paragraph.
    pub fn border(mut self, border: ParagraphBorder) -> Self {
        self.border = Some(border);
        self
    }

    /// Set the paragraph background fill colour.
    pub fn background(mut self, color: RgbColor) -> Self {
        self.background = Some(color);
        self
    }

    /// Keep this paragraph on the same page as the next.
    pub fn keep_next(mut self) -> Self {
        self.keep_next = true;
        self
    }

    /// Keep all lines of this paragraph together on the same page.
    pub fn keep_lines(mut self) -> Self {
        self.keep_lines = true;
        self
    }
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn parse_hex_color(hex: &str) -> Option<RgbColor> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 { return None; }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(RgbColor { r: r as f64 / 255.0, g: g as f64 / 255.0, b: b as f64 / 255.0 })
}

/// Estimates line count without a full FontRegistry (character-width heuristic).
fn estimate_line_count(runs: &[TextRun], max_width_mm: f64, font_size: f64) -> usize {
    let char_width_mm = font_size * 0.5 * 25.4 / 72.0;
    let chars_per_line = (max_width_mm / char_width_mm).floor() as usize;
    let total_chars: usize = runs.iter().map(|r| r.text.len()).sum();
    if chars_per_line == 0 { return 1; }
    total_chars.div_ceil(chars_per_line).max(1)
}

impl Element for Paragraph {
    fn estimated_height_mm(&self) -> f64 {
        let approx_width_mm = (165.0 - self.indent_left_mm - self.indent_right_mm).max(10.0);
        let font_size = self.font_size.unwrap_or(11.0);

        let runs: Vec<TextRun> = match &self.content {
            ParagraphContent::Plain(text) => vec![TextRun {
                text: text.clone(),
                style: AppliedStyle { bold: self.bold, italic: self.italic, ..Default::default() },
                letter_spacing_mm: 0.0,
                ..Default::default()
            }],
            ParagraphContent::Runs(runs) => runs.clone(),
        };

        let line_count = estimate_line_count(&runs, approx_width_mm, font_size);
        let line_h = font_size * 1.4 * 25.4 / 72.0;
        let space_after = self.space_after_mm.unwrap_or(font_size * 0.3 * 25.4 / 72.0);
        let space_before = self.space_before_mm.unwrap_or(0.0);
        space_before + (line_count as f64) * line_h + space_after
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let start_line = ctx.resume_index;
        let is_fresh = start_line == 0;

        // ── Resolve named style (if any) ──────────────────────────────────────
        let resolved = if let Some(ref name) = self.style_ref {
            let resolver = StyleResolver::new(&ctx.style.named_styles, &ctx.style);
            Some(resolver.resolve(name)?)
        } else {
            None
        };

        // ── Effective values (explicit > resolved style > document defaults) ──
        let font_size = self.font_size
            .or_else(|| resolved.as_ref().map(|r| r.font_size))
            .unwrap_or(ctx.style.font_size_body);

        let effective_alignment = if self.alignment != TextAlign::Justify {
            self.alignment
        } else if let Some(ref r) = resolved {
            r.alignment
        } else {
            self.alignment
        };

        let space_after = self.space_after_mm
            .or_else(|| resolved.as_ref().map(|r| r.space_after_mm))
            .unwrap_or(font_size * 0.3 * 25.4 / 72.0);

        let space_before = self.space_before_mm
            .or_else(|| resolved.as_ref().map(|r| r.space_before_mm))
            .unwrap_or(0.0);

        let indent_left = if self.indent_left_mm != 0.0 {
            self.indent_left_mm
        } else {
            resolved.as_ref().map(|r| r.indent_left_mm).unwrap_or(0.0)
        };

        let indent_right = if self.indent_right_mm != 0.0 {
            self.indent_right_mm
        } else {
            resolved.as_ref().map(|r| r.indent_right_mm).unwrap_or(0.0)
        };

        let indent_first = if self.indent_first_line_mm != 0.0 {
            self.indent_first_line_mm
        } else {
            resolved.as_ref().map(|r| r.indent_first_line_mm).unwrap_or(0.0)
        };

        if is_fresh && space_before > 0.0 && !ctx.flow.is_top_of_page() {
            ctx.flow.advance(space_before);
        }

        // ── Build TextRuns and lay out lines ──────────────────────────────────
        // (Moved here so orphan/widow can inspect line count before the loop.)
        let runs: Vec<TextRun> = match &self.content {
            ParagraphContent::Plain(text) => {
                let bold = self.bold || resolved.as_ref().map(|r| r.bold).unwrap_or(false);
                let italic = self.italic || resolved.as_ref().map(|r| r.italic).unwrap_or(false);
                vec![TextRun {
                    text: text.clone(),
                    style: AppliedStyle { bold, italic, ..Default::default() },
                    letter_spacing_mm: 0.0,
                    ..Default::default()
                }]
            }
            ParagraphContent::Runs(runs) => runs.clone(),
        };

        let max_width = (ctx.layout.content_width_mm - indent_left - indent_right).max(1.0);

        let result = ctx.layout_engine.layout_runs(
            &ctx.fonts, &runs, max_width, effective_alignment, font_size, &self.tab_stops,
        );

        // ── Orphan / widow pre-calculation ────────────────────────────────────
        // Simulate how many lines fit from current cursor.
        // natural_break = index of first line that would overflow.
        let natural_break: Option<usize> = {
            let mut sim_y = ctx.flow.cursor_y_mm;
            let mut found = None;
            for (idx, line) in result.lines.iter().enumerate().skip(start_line) {
                if sim_y - line.height_mm < ctx.flow.margin_bottom_mm {
                    found = Some(idx);
                    break;
                }
                sim_y -= line.height_mm;
            }
            found
        };

        // Orphan control: if fresh and a split would leave too few lines at the
        // bottom, push the entire paragraph to the next page instead.
        if is_fresh && !ctx.flow.is_top_of_page() {
            if let Some(nb) = natural_break {
                let lines_on_page = nb - start_line;
                let min_orphan = ctx.style.min_orphan_lines as usize;
                if min_orphan > 0 && lines_on_page > 0 && lines_on_page < min_orphan {
                    ctx.resume_index = 0;
                    return Ok(RenderResult::more());
                }
            }
        }

        // Widow control: if too few lines would land on the NEXT page, move the
        // break point earlier so at least min_widow_lines go to the next page.
        let break_at: Option<usize> = if let Some(nb) = natural_break {
            let min_widow = ctx.style.min_widow_lines as usize;
            let total = result.lines.len();
            let lines_after = total - nb;
            if min_widow > 0 && lines_after > 0 && lines_after < min_widow {
                let earlier = total.saturating_sub(min_widow);
                // Only move break earlier if it leaves at least one line on current page.
                if earlier > start_line { Some(earlier) } else { Some(nb) }
            } else {
                Some(nb)
            }
        } else {
            None
        };

        // UA-2 marked content
        let ua_mcid = if ctx.ua_enabled() {
            let mcid = ctx.ua_tag_element(StructTag::P, None);
            ctx.backend.begin_tagged_content(b"P", mcid);
            Some(mcid)
        } else {
            None
        };

        // Clone text color now to avoid borrow conflicts during drawing.
        let text_color: RgbColor = resolved.as_ref()
            .and_then(|r| r.color.clone())
            .unwrap_or_else(|| ctx.style.text_color.clone());

        // ── Background rect (only on fresh start — can't span pages) ────────
        let block_y_top = ctx.flow.cursor_y_mm;
        let block_x = ctx.layout.content_x_mm + indent_left;
        let block_w = max_width;
        let block_h = result.total_height_mm;

        if is_fresh {
            if let Some(ref bg) = self.background {
                let pad = self.border.as_ref().map(|b| b.padding_mm).unwrap_or(0.0);
                if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
                ctx.backend.draw_rect(
                    block_x - pad,
                    block_y_top - block_h - pad,
                    block_w + pad * 2.0,
                    block_h + pad * 2.0,
                    bg,
                )?;
                if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
            }
        }

        // ── Emit lines ────────────────────────────────────────────────────────
        for (line_idx, line) in result.lines.iter().enumerate().skip(start_line) {
            // Break at the pre-calculated point (includes widow/orphan adjustments).
            if Some(line_idx) == break_at {
                ctx.resume_index = line_idx;
                if ua_mcid.is_some() { ctx.backend.end_tagged_content(); }
                return Ok(RenderResult::more());
            }

            let y = ctx.flow.cursor_y_mm;
            ctx.flow.advance(line.height_mm);

            let first_line_extra = if line_idx == 0 { indent_first } else { 0.0 };

            for seg in &line.segments {
                if seg.text.is_empty() {
                    continue;
                }
                let Some(font_ref) = ctx.get_font_ref(seg.style.bold, seg.style.italic) else {
                    continue;
                };
                let x = ctx.layout.content_x_mm + indent_left + first_line_extra + seg.x_offset_mm;
                let seg_w = ctx.fonts.measure_text_mm(
                    &seg.text, &ctx.default_font_family, seg.font_size, seg.style.bold, seg.style.italic,
                );

                // Highlight pre-pass: filled rect behind the text (Artifact).
                if let Some(ref h) = seg.style.highlight {
                    let hl_color = parse_hex_color(h)
                        .unwrap_or(RgbColor { r: 1.0, g: 1.0, b: 0.0 });
                    if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
                    ctx.backend.draw_rect(x, y - line.height_mm, seg_w, line.height_mm, &hl_color)?;
                    if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
                }

                // Text.
                if seg.letter_spacing_mm > 0.0 {
                    let ls_pt = (seg.letter_spacing_mm * 72.0 / 25.4) as f32;
                    ctx.draw_text_spaced(&seg.text, x, y, font_size, font_ref, &text_color, ls_pt)?;
                } else {
                    ctx.draw_text(&seg.text, x, y, font_size, font_ref, &text_color)?;
                }

                // Decoration post-pass: underline / strikethrough (Artifact).
                let underline = seg.style.underline;
                let strikethrough = seg.style.strikethrough;
                if underline || strikethrough {
                    if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
                    if underline {
                        let ul_y = y - seg.font_size * 0.15 * 25.4 / 72.0;
                        ctx.draw_hline(x, x + seg_w, ul_y, 0.5, &text_color)?;
                    }
                    if strikethrough {
                        let st_y = y + seg.font_size * 0.25 * 25.4 / 72.0;
                        ctx.draw_hline(x, x + seg_w, st_y, 0.5, &text_color)?;
                    }
                    if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
                }
            }
        }

        // ── Paragraph borders (only on fresh start — can't span pages) ────────
        if is_fresh { if let Some(ref brd) = self.border {
            let pad = brd.padding_mm;
            let x0 = block_x - pad;
            let x1 = block_x + block_w + pad;
            let y_top = block_y_top + pad;
            let y_bot = block_y_top - block_h - pad;
            let has_border = brd.top.is_some() || brd.bottom.is_some()
                || brd.left.is_some() || brd.right.is_some();
            if ctx.ua_config.enabled && has_border { ctx.backend.begin_artifact_content(); }
            if let Some(ref dl) = brd.top {
                let col = dl.color.as_ref().cloned()
                    .unwrap_or(RgbColor { r: 0.0, g: 0.0, b: 0.0 });
                let pt = (dl.thickness_mm * 72.0 / 25.4) as f32;
                ctx.draw_hline(x0, x1, y_top, pt, &col)?;
            }
            if let Some(ref dl) = brd.bottom {
                let col = dl.color.as_ref().cloned()
                    .unwrap_or(RgbColor { r: 0.0, g: 0.0, b: 0.0 });
                let pt = (dl.thickness_mm * 72.0 / 25.4) as f32;
                ctx.draw_hline(x0, x1, y_bot, pt, &col)?;
            }
            if let Some(ref dl) = brd.left {
                let col = dl.color.as_ref().cloned()
                    .unwrap_or(RgbColor { r: 0.0, g: 0.0, b: 0.0 });
                let pt = (dl.thickness_mm * 72.0 / 25.4) as f32;
                ctx.draw_vline(x0, y_bot, y_top, pt, &col)?;
            }
            if let Some(ref dl) = brd.right {
                let col = dl.color.as_ref().cloned()
                    .unwrap_or(RgbColor { r: 0.0, g: 0.0, b: 0.0 });
                let pt = (dl.thickness_mm * 72.0 / 25.4) as f32;
                ctx.draw_vline(x1, y_bot, y_top, pt, &col)?;
            }
            if ctx.ua_config.enabled && has_border { ctx.backend.end_tagged_content(); }
        } } // end is_fresh border block

        if ua_mcid.is_some() {
            ctx.backend.end_tagged_content();
        }

        // ── space_after ───────────────────────────────────────────────────────
        ctx.flow.advance(space_after);

        Ok(RenderResult::done())
    }
}

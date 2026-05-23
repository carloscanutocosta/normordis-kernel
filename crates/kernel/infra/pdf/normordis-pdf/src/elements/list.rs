use serde::{Deserialize, Serialize};

use super::{paragraph::TextRun, Element, RenderContext, RenderResult};
use crate::{
    compliance::ua::StructTag,
    layout::LayoutResult,
    richtext::marks::AppliedStyle,
};

/// A single item in an unordered or ordered list.
///
/// `ListItem` is a stable alias for this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemElement {
    /// Nesting depth (0 = top level).
    pub indent: u8,
    pub runs: Vec<TextRun>,
}

/// A single item in a checklist, with a checked state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckListItem {
    pub checked: bool,
    pub indent: u8,
    pub runs: Vec<TextRun>,
}

/// Stable public alias for [`ListItemElement`].
pub type ListItem = ListItemElement;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Indentation per nesting level in mm.
const INDENT_MM: f64 = 5.0;

/// Renders a list item with PDF/UA-2 structure: LI > Lbl (prefix) + LBody (text).
/// When UA is disabled, falls back to rendering everything in a single block.
fn render_item_ua_or_plain(
    ctx: &mut RenderContext,
    indent: u8,
    prefix: &str,
    text_x: f64,
    fs: f64,
    result: &LayoutResult,
) {
    let ua = ctx.ua_config.enabled;
    let indent_mm = f64::from(indent) * INDENT_MM;
    let tc = ctx.style.text_color.clone();

    if ua {
        let mcid_lbl = ctx.next_mcid();
        let mcid_body = ctx.next_mcid();

        // Structure tree: LI > Lbl + LBody
        ctx.ua_begin_group(StructTag::Lbl, None);
        ctx.ua_content_ref(mcid_lbl);
        ctx.ua_end_group();
        ctx.ua_begin_group(StructTag::LBody, None);
        ctx.ua_content_ref(mcid_body);
        ctx.ua_end_group();

        // Lbl content stream: draw prefix at first-line Y without advancing
        let y_first = ctx.flow.cursor_y_mm;
        ctx.backend.begin_tagged_content(b"Lbl", mcid_lbl);
        if !prefix.is_empty() {
            let px = ctx.layout.content_x_mm + indent_mm;
            if let Some(font_ref) = ctx.get_font_ref(false, false) {
                let _ = ctx.draw_text(prefix, px, y_first, fs, font_ref, &tc);
            }
        }
        ctx.backend.end_tagged_content();

        // LBody content stream: advance cursor and draw text lines
        ctx.backend.begin_tagged_content(b"LBody", mcid_body);
        for line in &result.lines {
            let y = ctx.flow.cursor_y_mm;
            ctx.flow.advance(line.height_mm);
            for seg in &line.segments {
                if seg.text.is_empty() { continue; }
                let Some(font_ref) = ctx.get_font_ref(seg.style.bold, seg.style.italic) else { continue };
                let x = text_x + seg.x_offset_mm;
                let _ = ctx.draw_text(&seg.text, x, y, fs, font_ref, &tc);
            }
        }
        ctx.flow.advance(fs * 0.20 * 25.4 / 72.0);
        ctx.backend.end_tagged_content();
    } else {
        render_list_item_with_layout(ctx, indent, prefix, text_x, fs, result);
    }
}

/// Computes layout for a list item, returning (text_x_mm, layout_result).
fn layout_item(
    ctx: &RenderContext,
    indent: u8,
    prefix: &str,
    runs: &[TextRun],
    fs: f64,
) -> (f64, LayoutResult) {
    let indent_mm = f64::from(indent) * INDENT_MM;
    let prefix_w = ctx.fonts.get_default().measure_text_mm(prefix, fs, false, false);
    let text_x = ctx.layout.content_x_mm + indent_mm + prefix_w;
    let max_w = (ctx.layout.content_width_mm - indent_mm - prefix_w).max(1.0);
    let result = ctx.layout_engine.layout_runs(
        &ctx.fonts,
        runs,
        max_w,
        crate::layout::TextAlign::Justify,
        fs,
        &[],
    );
    (text_x, result)
}

fn render_list_item_with_layout(
    ctx: &mut RenderContext,
    indent: u8,
    prefix: &str,
    text_x: f64,
    fs: f64,
    result: &LayoutResult,
) {
    let indent_mm = f64::from(indent) * INDENT_MM;
    let tc = ctx.style.text_color.clone();

    let mut first_line = true;
    for line in &result.lines {
        let y = ctx.flow.cursor_y_mm;
        ctx.flow.advance(line.height_mm);

        if first_line && !prefix.is_empty() {
            first_line = false;
            let px = ctx.layout.content_x_mm + indent_mm;
            if let Some(font_ref) = ctx.get_font_ref(false, false) {
                let _ = ctx.draw_text(prefix, px, y, fs, font_ref, &tc);
            }
        } else {
            first_line = false;
        }

        for seg in &line.segments {
            if seg.text.is_empty() {
                continue;
            }
            let Some(font_ref) = ctx.get_font_ref(seg.style.bold, seg.style.italic) else {
                continue;
            };
            let x = text_x + seg.x_offset_mm;
            let _ = ctx.draw_text(&seg.text, x, y, fs, font_ref, &tc);
        }
    }

    ctx.flow.advance(fs * 0.20 * 25.4 / 72.0);
}

// ── BulletList ────────────────────────────────────────────────────────────────

/// An unordered (bullet) list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletList {
    pub items: Vec<ListItemElement>,
}

impl BulletList {
    pub fn new(items: Vec<ListItemElement>) -> Self {
        Self { items }
    }
}

impl Element for BulletList {
    fn estimated_height_mm(&self) -> f64 {
        self.items.len() as f64 * 6.5
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let start = ctx.resume_index;
        let fs = ctx.style.font_size_body;
        let ua = ctx.ua_config.enabled;

        if ua && start == 0 { ctx.ua_begin_group(StructTag::L, None); }

        for (i, item) in self.items.iter().enumerate().skip(start) {
            let (text_x, layout) = layout_item(ctx, item.indent, "• ", &item.runs, fs);
            let item_h = layout.total_height_mm + fs * 0.20 * 25.4 / 72.0;

            if ctx.flow.would_overflow(item_h) && i > start {
                ctx.resume_index = i;
                if ua { ctx.ua_end_group(); }
                return Ok(RenderResult::more());
            }

            if ua { ctx.ua_begin_group(StructTag::LI, None); }
            render_item_ua_or_plain(ctx, item.indent, "• ", text_x, fs, &layout);
            if ua { ctx.ua_end_group(); }
        }

        if ua { ctx.ua_end_group(); }
        Ok(RenderResult::done())
    }
}

// ── OrderedList ───────────────────────────────────────────────────────────────

/// An ordered (numbered) list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedList {
    /// Starting counter value (usually 1).
    pub start: u32,
    pub items: Vec<ListItemElement>,
}

impl OrderedList {
    pub fn new(items: Vec<ListItemElement>) -> Self {
        Self { start: 1, items }
    }
}

impl Element for OrderedList {
    fn estimated_height_mm(&self) -> f64 {
        self.items.len() as f64 * 6.5
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let start = ctx.resume_index;
        let fs = ctx.style.font_size_body;
        let ua = ctx.ua_config.enabled;

        if ua && start == 0 { ctx.ua_begin_group(StructTag::L, None); }

        for (i, item) in self.items.iter().enumerate().skip(start) {
            let prefix = format!("{}. ", self.start + i as u32);
            let (text_x, layout) = layout_item(ctx, item.indent, &prefix, &item.runs, fs);
            let item_h = layout.total_height_mm + fs * 0.20 * 25.4 / 72.0;

            if ctx.flow.would_overflow(item_h) && i > start {
                ctx.resume_index = i;
                if ua { ctx.ua_end_group(); }
                return Ok(RenderResult::more());
            }

            if ua { ctx.ua_begin_group(StructTag::LI, None); }
            render_item_ua_or_plain(ctx, item.indent, &prefix, text_x, fs, &layout);
            if ua { ctx.ua_end_group(); }
        }

        if ua { ctx.ua_end_group(); }
        Ok(RenderResult::done())
    }
}

// ── CheckList ─────────────────────────────────────────────────────────────────

/// A checklist with checkbox indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckList {
    pub items: Vec<CheckListItem>,
}

impl CheckList {
    pub fn new(items: Vec<CheckListItem>) -> Self {
        Self { items }
    }
}

impl Element for CheckList {
    fn estimated_height_mm(&self) -> f64 {
        self.items.len() as f64 * 6.5
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let start = ctx.resume_index;
        let fs = ctx.style.font_size_body;
        let ua = ctx.ua_config.enabled;

        if ua && start == 0 { ctx.ua_begin_group(StructTag::L, None); }

        for (i, item) in self.items.iter().enumerate().skip(start) {
            let prefix = if item.checked { "[x] " } else { "[ ] " };
            let runs: Vec<TextRun> = item.runs.iter().map(|r| TextRun {
                text: r.text.clone(),
                style: r.style.clone(),
                letter_spacing_mm: r.letter_spacing_mm,
                ..Default::default()
            }).collect();
            let (text_x, layout) = layout_item(ctx, item.indent, prefix, &runs, fs);
            let item_h = layout.total_height_mm + fs * 0.20 * 25.4 / 72.0;

            if ctx.flow.would_overflow(item_h) && i > start {
                ctx.resume_index = i;
                if ua { ctx.ua_end_group(); }
                return Ok(RenderResult::more());
            }

            if ua { ctx.ua_begin_group(StructTag::LI, None); }
            render_item_ua_or_plain(ctx, item.indent, prefix, text_x, fs, &layout);
            if ua { ctx.ua_end_group(); }
        }

        if ua { ctx.ua_end_group(); }
        Ok(RenderResult::done())
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

impl ListItemElement {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            indent: 0,
            runs: vec![TextRun {
                text: text.into(),
                style: AppliedStyle::default(),
                letter_spacing_mm: 0.0,
                ..Default::default()
            }],
        }
    }

    pub fn with_indent(mut self, indent: u8) -> Self {
        self.indent = indent;
        self
    }
}

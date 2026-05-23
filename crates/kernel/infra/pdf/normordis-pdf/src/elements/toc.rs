use super::{Element, RenderContext, RenderResult};
use crate::{
    layout::TextAlign,
    styles::StyleResolver,
};

// ── TocEntry ──────────────────────────────────────────────────────────────────

/// A heading entry collected during the first pass for TOC rendering.
#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub page_number: u32,
}

// ── TableOfContents ───────────────────────────────────────────────────────────

/// Automatic Table of Contents element.
///
/// Placed in the document flow where the TOC should appear. Rendered in two
/// passes: the first pass (by the document loop) collects section headings and
/// page numbers; the second pass renders entries with dot leaders and
/// right-aligned page numbers.
///
/// # Example
/// ```rust
/// use normordis_pdf::*;
///
/// let pdf = DocumentBuilder::new("Relatório")
///     .push(TableOfContents::new().title("Índice").max_level(3))
///     .push(Section::new("1. Introdução", 1))
///     .render_to_bytes()?;
/// # Ok::<(), normordis_pdf::NormaxisPdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct TableOfContents {
    /// Title rendered above the TOC entries. `None` = no title.
    pub title: Option<String>,
    /// Maximum heading level to include (1–6). Default: 3.
    pub max_level: u8,
    /// Fill character for the leader between title and page number. Default: `'.'`.
    pub leader_char: char,
    /// Style name for the TOC title. Default: `"heading_1"`.
    pub title_style: String,
    /// Style names for TOC entry levels. Index 0 = level 1.
    pub entry_styles: Vec<String>,
    /// Entries injected by the document loop in the second pass. `None` = first pass.
    pub(crate) entries: Option<Vec<TocEntry>>,
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self {
            title: Some("Índice".to_string()),
            max_level: 3,
            leader_char: '.',
            title_style: "heading_1".to_string(),
            entry_styles: vec!["toc_1".to_string(), "toc_2".to_string(), "toc_3".to_string()],
            entries: None,
        }
    }
}

impl TableOfContents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn no_title(mut self) -> Self {
        self.title = None;
        self
    }

    pub fn max_level(mut self, level: u8) -> Self {
        self.max_level = level.clamp(1, 6);
        self
    }

    pub fn dot_leader(mut self, c: char) -> Self {
        self.leader_char = c;
        self
    }

    /// Returns the style name for a given heading level (1-based).
    pub fn entry_style_for(&self, level: u8) -> &str {
        let idx = (level as usize).saturating_sub(1);
        self.entry_styles
            .get(idx)
            .map(|s| s.as_str())
            .unwrap_or("normal")
    }
}

impl Element for TableOfContents {
    fn inject_toc_entries(&mut self, entries: &[TocEntry]) {
        self.entries = Some(entries.to_vec());
    }

    fn estimated_height_mm(&self) -> f64 {
        match &self.entries {
            Some(entries) => {
                let title_h = if self.title.is_some() { 12.0 } else { 0.0 };
                let entries_h = entries.len() as f64 * 6.0;
                title_h + entries_h
            }
            None => 0.0,
        }
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let entries = match &self.entries {
            Some(e) => e,
            // First pass — nothing to render yet.
            None => return Ok(RenderResult::done()),
        };

        // Render optional title.
        if let Some(ref title) = self.title {
            let p = crate::elements::paragraph::Paragraph::new(title.clone())
                .style(self.title_style.as_str());
            p.render(ctx)?;
        }

        // Render each TOC entry.
        for entry in entries {
            if entry.level > self.max_level {
                continue;
            }
            render_toc_entry(entry, self.leader_char, self.entry_style_for(entry.level), ctx)?;
        }

        Ok(RenderResult::done())
    }
}

fn render_toc_entry(
    entry: &TocEntry,
    leader_char: char,
    style_name: &str,
    ctx: &mut RenderContext,
) -> crate::Result<()> {
    let resolver = StyleResolver::new(&ctx.style.named_styles, &ctx.style);
    let resolved = resolver.resolve(style_name)?;

    let font_size = resolved.font_size;
    let indent = resolved.indent_left_mm;
    let usable_w = ctx.layout.content_width_mm - indent;

    // Measure title and page number.
    let page_str = entry.page_number.to_string();
    let page_w = ctx.fonts.get_family(&resolved.font_family)
        .measure_text_mm(&page_str, font_size, true, false);
    let title_w = ctx.fonts.get_family(&resolved.font_family)
        .measure_text_mm(&entry.title, font_size, resolved.bold, resolved.italic);

    // Calculate how many leader characters fit in the gap.
    let leader_str = leader_char.to_string();
    let leader_char_w = ctx.fonts.get_family(&resolved.font_family)
        .measure_text_mm(&leader_str, font_size, false, false);
    let gap = usable_w - title_w - page_w - 4.0; // 4mm breathing room
    let n_leaders = if leader_char_w > 0.0 {
        ((gap / leader_char_w).floor() as usize).saturating_sub(1)
    } else {
        0
    };
    let leaders: String = std::iter::repeat(leader_char).take(n_leaders).collect();

    // Build a single paragraph: title + leaders + page number.
    let line_text = format!("{}{} {}", entry.title, leaders, page_str);

    let p = crate::elements::paragraph::Paragraph::new(line_text)
        .style(style_name)
        .align(TextAlign::Left);

    let y_top_mm = ctx.flow.cursor_y_mm;
    p.render(ctx)?;
    let y_bot_mm = ctx.flow.cursor_y_mm;

    // Invisible GoTo link annotation covering the full entry row.
    let x1 = ctx.layout.content_x_mm;
    let x2 = x1 + ctx.layout.content_width_mm;
    ctx.backend.add_link_annotation(x1, y_bot_mm, x2, y_top_mm, &entry.title, entry.page_number);

    Ok(())
}

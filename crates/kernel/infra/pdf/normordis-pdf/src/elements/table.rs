use serde::{Deserialize, Serialize};

use super::{fixed_text::VerticalAlign, Element, RenderContext, RenderResult};
use crate::{
    compliance::ua::StructTag,
    layout::TextAlign,
    richtext::marks::AppliedStyle,
    styles::RgbColor,
};

// ── RowHeight ─────────────────────────────────────────────────────────────────

/// Controls how row height is determined.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RowHeight {
    /// Height is determined by content (default).
    #[default]
    Auto,
    /// Minimum height in mm — row can grow if content requires it.
    AtLeast(f64),
    /// Exact height in mm — content is clipped if it exceeds this height.
    Exact(f64),
}

// ── CellBorders ───────────────────────────────────────────────────────────────

/// Per-edge border configuration for a table cell.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellBorders {
    pub top: Option<CellBorder>,
    pub bottom: Option<CellBorder>,
    pub left: Option<CellBorder>,
    pub right: Option<CellBorder>,
}

impl CellBorders {
    pub fn is_empty(&self) -> bool {
        self.top.is_none() && self.bottom.is_none()
            && self.left.is_none() && self.right.is_none()
    }
}

/// A single edge border for a table cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellBorder {
    pub width_mm: f64,
    pub color: RgbColor,
    pub style: BorderLineStyle,
}

impl Default for CellBorder {
    fn default() -> Self {
        Self {
            width_mm: 0.3,
            color: RgbColor { r: 0.8, g: 0.8, b: 0.8 },
            style: BorderLineStyle::Solid,
        }
    }
}

/// Stroke pattern for cell borders.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderLineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    None,
}

// ── CellPadding ───────────────────────────────────────────────────────────────

/// Per-edge insets for a table cell in mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellPadding {
    pub top_mm: f64,
    pub bottom_mm: f64,
    pub left_mm: f64,
    pub right_mm: f64,
}

impl Default for CellPadding {
    fn default() -> Self {
        Self { top_mm: 1.0, bottom_mm: 1.0, left_mm: 2.0, right_mm: 2.0 }
    }
}

impl CellPadding {
    pub fn uniform(mm: f64) -> Self {
        Self { top_mm: mm, bottom_mm: mm, left_mm: mm, right_mm: mm }
    }

    pub fn horizontal_vertical(h_mm: f64, v_mm: f64) -> Self {
        Self { top_mm: v_mm, bottom_mm: v_mm, left_mm: h_mm, right_mm: h_mm }
    }
}

// ── TableStyle ────────────────────────────────────────────────────────────────

/// Named table style — controls border, header background, and stripe settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStyle {
    /// Border drawn around the outside of the whole table.
    pub outer_border: Option<CellBorder>,
    /// Border drawn between cells (inner grid lines).
    pub inner_border: Option<CellBorder>,
    /// Background colour for header rows. `None` = no header background.
    pub header_background: Option<RgbColor>,
    /// Stripe colour for alternate body rows. `None` = no stripes.
    pub stripe_color: Option<RgbColor>,
}

impl TableStyle {
    /// Grid style — thin borders everywhere, light header background, no stripes.
    pub fn grid() -> Self {
        Self {
            outer_border: Some(CellBorder::default()),
            inner_border: Some(CellBorder::default()),
            header_background: Some(RgbColor { r: 0.85, g: 0.88, b: 0.95 }),
            stripe_color: None,
        }
    }

    /// Bordered style — outer border only, header background, stripes.
    pub fn bordered() -> Self {
        Self {
            outer_border: Some(CellBorder { width_mm: 0.5, ..CellBorder::default() }),
            inner_border: None,
            header_background: Some(RgbColor { r: 0.85, g: 0.88, b: 0.95 }),
            stripe_color: Some(RgbColor { r: 0.96, g: 0.96, b: 0.96 }),
        }
    }

    /// Striped style — no borders, alternating row background.
    pub fn striped() -> Self {
        Self {
            outer_border: None,
            inner_border: None,
            header_background: Some(RgbColor { r: 0.85, g: 0.88, b: 0.95 }),
            stripe_color: Some(RgbColor { r: 0.96, g: 0.96, b: 0.96 }),
        }
    }

    /// Plain style — no borders, no background.
    pub fn plain() -> Self {
        Self {
            outer_border: None,
            inner_border: None,
            header_background: None,
            stripe_color: None,
        }
    }
}

// ── TableCell ─────────────────────────────────────────────────────────────────

/// A single cell in a table row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub text: String,
    /// Number of columns this cell spans (default: 1).
    #[serde(default = "default_span")]
    pub col_span: u16,
    /// Number of rows this cell spans (default: 1).
    #[serde(default = "default_span")]
    pub row_span: u16,
    /// Text alignment within this cell (default: Left).
    #[serde(default)]
    pub alignment: TextAlign,
    /// Per-edge borders. If empty, uses the table-level default borders.
    #[serde(default)]
    pub borders: CellBorders,
    /// Background fill override for this cell.
    #[serde(default)]
    pub background: Option<RgbColor>,
    /// Vertical alignment of text within the cell.
    #[serde(default)]
    pub vertical_align: VerticalAlign,
    /// Cell padding (insets). Defaults to 1 mm top/bottom, 2 mm left/right.
    #[serde(default)]
    pub padding: CellPadding,
    /// Named paragraph style for the cell text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_ref: Option<String>,
    /// Optional nested table. When set, renders a sub-table instead of `text`.
    #[serde(skip)]
    pub nested_table: Option<Box<Table>>,
}

fn default_span() -> u16 { 1 }

impl TableCell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            col_span: 1,
            row_span: 1,
            alignment: TextAlign::Left,
            borders: CellBorders::default(),
            background: None,
            vertical_align: VerticalAlign::Top,
            padding: CellPadding::default(),
            style_ref: None,
            nested_table: None,
        }
    }

    /// Sets a nested sub-table as the cell content (instead of text).
    pub fn nested_table(mut self, table: Table) -> Self {
        self.nested_table = Some(Box::new(table));
        self
    }

    /// Apply a named style to this cell's text.
    pub fn style(mut self, name: impl Into<String>) -> Self {
        self.style_ref = Some(name.into());
        self
    }

    pub fn padding(mut self, padding: CellPadding) -> Self {
        self.padding = padding;
        self
    }

    pub fn col_span(mut self, n: u16) -> Self {
        self.col_span = n.max(1);
        self
    }

    pub fn row_span(mut self, n: u16) -> Self {
        self.row_span = n.max(1);
        self
    }

    pub fn align(mut self, alignment: TextAlign) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn background(mut self, color: RgbColor) -> Self {
        self.background = Some(color);
        self
    }
}

impl From<String> for TableCell {
    fn from(s: String) -> Self { Self::new(s) }
}

impl From<&str> for TableCell {
    fn from(s: &str) -> Self { Self::new(s) }
}

// ── TableRow ──────────────────────────────────────────────────────────────────

/// A row in a table with optional exact/minimum height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub height: RowHeight,
    pub is_header: bool,
}

impl TableRow {
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self { cells, height: RowHeight::Auto, is_header: false }
    }

    /// Constructs a row from plain strings — convenience for simple tables.
    pub fn plain(cells: Vec<String>) -> Self {
        Self::new(cells.into_iter().map(TableCell::from).collect())
    }

    /// Sets an exact height for this row in mm.
    pub fn height_exact(mut self, mm: f64) -> Self {
        self.height = RowHeight::Exact(mm);
        self
    }

    /// Sets a minimum height for this row in mm.
    pub fn height_at_least(mut self, mm: f64) -> Self {
        self.height = RowHeight::AtLeast(mm);
        self
    }
}

// ── TableBuilder ──────────────────────────────────────────────────────────────

/// Fluent builder for tables with complex headers (`col_span`/`row_span`).
pub struct TableBuilder {
    header_rows: Vec<TableRow>,
    body_rows: Vec<TableRow>,
    col_widths: Option<Vec<f64>>,
    show_header_background: bool,
    stripe_rows: bool,
    table_style: Option<TableStyle>,
}

impl TableBuilder {
    pub fn header_row(mut self, cells: Vec<TableCell>) -> Self {
        let mut row = TableRow::new(cells);
        row.is_header = true;
        self.header_rows.push(row);
        self
    }

    pub fn row(mut self, cells: Vec<TableCell>) -> Self {
        self.body_rows.push(TableRow::new(cells));
        self
    }

    pub fn col_widths(mut self, pcts: Vec<f64>) -> Self {
        self.col_widths = Some(pcts);
        self
    }

    pub fn stripe(mut self) -> Self {
        self.stripe_rows = true;
        self
    }

    /// Apply a named table style (e.g. `TableStyle::grid()`).
    pub fn table_style(mut self, style: TableStyle) -> Self {
        self.table_style = Some(style);
        self
    }

    pub fn build(self) -> Table {
        Table {
            headers: Vec::new(),
            header_rows: self.header_rows,
            rows: self.body_rows,
            col_widths: self.col_widths,
            show_header_background: self.show_header_background,
            stripe_rows: self.stripe_rows,
            table_style: self.table_style,
        }
    }
}

// ── Table ─────────────────────────────────────────────────────────────────────

/// A data table with optional header background and alternating row stripes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Simple string headers (backward compat). Converted to a header row on render.
    pub headers: Vec<String>,
    /// Rich header rows supporting `col_span` (set via `Table::builder()`).
    #[serde(default)]
    pub header_rows: Vec<TableRow>,
    pub rows: Vec<TableRow>,
    /// Column widths as percentages of content width. `None` = equal distribution.
    pub col_widths: Option<Vec<f64>>,
    /// Fill header row with `primary_color` at reduced opacity.
    pub show_header_background: bool,
    /// Alternate row backgrounds for readability.
    pub stripe_rows: bool,
    /// Optional named table style. When set, overrides `show_header_background` and
    /// `stripe_rows`, and controls border drawing.
    #[serde(default)]
    pub table_style: Option<TableStyle>,
}

impl Table {
    pub fn new(headers: Vec<String>, rows: Vec<TableRow>) -> Self {
        Self {
            headers,
            header_rows: Vec::new(),
            rows,
            col_widths: None,
            show_header_background: true,
            stripe_rows: true,
            table_style: None,
        }
    }

    pub fn builder() -> TableBuilder {
        TableBuilder {
            header_rows: Vec::new(),
            body_rows: Vec::new(),
            col_widths: None,
            show_header_background: true,
            stripe_rows: false,
            table_style: None,
        }
    }

    pub fn col_widths(mut self, widths: Vec<f64>) -> Self {
        self.col_widths = Some(widths);
        self
    }

    /// Apply a named table style.
    pub fn with_table_style(mut self, style: TableStyle) -> Self {
        self.table_style = Some(style);
        self
    }

    /// Enables alternating row stripes (already on by default for `Table::new`).
    pub fn stripe(self) -> Self {
        self
    }

    fn min_row_height_mm() -> f64 { 6.5 }

    fn effective_row_height(row: &TableRow, measured: f64) -> f64 {
        match row.height {
            RowHeight::Auto => measured.max(Self::min_row_height_mm()),
            RowHeight::AtLeast(h) => measured.max(h),
            RowHeight::Exact(h) => h,
        }
    }

    /// Computes column widths from percentages or equal distribution.
    fn col_widths_mm(&self, usable_width: f64, col_count: usize) -> Vec<f64> {
        if col_count == 0 { return Vec::new(); }
        match &self.col_widths {
            Some(pcts) => pcts.iter().map(|p| p / 100.0 * usable_width).collect(),
            None => vec![usable_width / col_count as f64; col_count],
        }
    }

    /// Computes the column count by scanning header rows and body rows.
    fn effective_col_count(&self) -> usize {
        let mut max = self.headers.len();
        for r in &self.header_rows {
            let span_sum: usize = r.cells.iter().map(|c| c.col_span as usize).sum();
            max = max.max(span_sum);
        }
        for r in &self.rows {
            let span_sum: usize = r.cells.iter().map(|c| c.col_span as usize).sum();
            max = max.max(span_sum);
        }
        max
    }

    /// Measures the text height of a row and returns the effective row height in mm.
    fn measure_row_height(
        &self,
        row: &TableRow,
        col_widths: &[f64],
        ctx: &RenderContext,
    ) -> f64 {
        let fs = ctx.style.font_size_body;
        let mut measured = Self::min_row_height_mm();
        let col_count = col_widths.len();
        let mut col_idx = 0;

        for cell in &row.cells {
            let span = (cell.col_span as usize).min(col_count.saturating_sub(col_idx));
            if span == 0 { break; }
            let w: f64 = col_widths[col_idx..col_idx + span].iter().sum();
            let h_pad = cell.padding.left_mm + cell.padding.right_mm;
            let v_pad = cell.padding.top_mm + cell.padding.bottom_mm;
            let inner_w = (w - h_pad).max(1.0);
            let r = ctx.layout_engine.layout_plain(
                &ctx.fonts, &cell.text, inner_w, cell.alignment, fs,
                AppliedStyle::default(),
            );
            measured = measured.max(r.total_height_mm + v_pad);
            col_idx += span;
        }

        Self::effective_row_height(row, measured)
    }

    /// Renders one row: background, borders, text. Does NOT advance the cursor.
    #[allow(clippy::too_many_arguments)]
    fn render_row(
        &self,
        row: &TableRow,
        col_widths: &[f64],
        x_base: f64,
        row_h: f64,
        body_row_idx: usize,
        is_header: bool,
        ctx: &mut RenderContext,
    ) {
        let y_top = ctx.flow.cursor_y_mm;
        let y_bottom = y_top - row_h;
        let tc = ctx.style.text_color.clone();
        let col_count = col_widths.len();
        let total_w: f64 = col_widths.iter().sum();

        // Row background
        let bg: Option<RgbColor> = if let Some(ref ts) = self.table_style {
            if is_header {
                ts.header_background.clone()
            } else if body_row_idx % 2 == 1 {
                ts.stripe_color.clone()
            } else {
                None
            }
        } else if is_header && self.show_header_background {
            let pc = &ctx.style.primary_color;
            Some(RgbColor {
                r: pc.r * 0.85 + 0.15,
                g: pc.g * 0.85 + 0.15,
                b: pc.b * 0.85 + 0.15,
            })
        } else if !is_header && self.stripe_rows && body_row_idx % 2 == 1 {
            Some(RgbColor { r: 0.96, g: 0.96, b: 0.96 })
        } else {
            None
        };

        if let Some(bg_col) = bg {
            if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
            let _ = ctx.backend.draw_rect(x_base, y_bottom, total_w, row_h, &bg_col);
            if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
        }

        let fs = ctx.style.font_size_body;
        let mut col_x = x_base;
        let mut col_idx = 0;

        for cell in &row.cells {
            let span = (cell.col_span as usize).min(col_count.saturating_sub(col_idx));
            if span == 0 { break; }
            let cell_w: f64 = col_widths[col_idx..col_idx + span].iter().sum();

            // Per-cell background override (Artifact)
            if let Some(ref cell_bg) = cell.background {
                if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
                let _ = ctx.backend.draw_rect(col_x, y_bottom, cell_w, row_h, cell_bg);
                if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
            }

            let h_pad = cell.padding.left_mm + cell.padding.right_mm;
            let inner_w = (cell_w - h_pad).max(1.0);

            if let Some(ref nested) = cell.nested_table {
                let saved_x = ctx.layout.content_x_mm;
                let saved_w = ctx.layout.content_width_mm;
                let saved_cursor = ctx.flow.cursor_y_mm;

                ctx.layout.content_x_mm = col_x + cell.padding.left_mm;
                ctx.layout.content_width_mm = inner_w;
                ctx.flow.cursor_y_mm = y_top - cell.padding.top_mm;
                ctx.resume_index = 0;

                let _ = nested.render(ctx);

                ctx.layout.content_x_mm = saved_x;
                ctx.layout.content_width_mm = saved_w;
                ctx.flow.cursor_y_mm = saved_cursor;
            } else {
                let result = ctx.layout_engine.layout_plain(
                    &ctx.fonts, &cell.text, inner_w, cell.alignment, fs,
                    AppliedStyle::default(),
                );

                let content_h = result.total_height_mm;
                let text_y_start = match cell.vertical_align {
                    VerticalAlign::Top => y_top - cell.padding.top_mm,
                    VerticalAlign::Middle => {
                        let inner_h = row_h - cell.padding.top_mm - cell.padding.bottom_mm;
                        y_top - cell.padding.top_mm - ((inner_h - content_h) / 2.0).max(0.0)
                    }
                    VerticalAlign::Bottom => y_bottom + cell.padding.bottom_mm + content_h,
                };

                let mut line_y = text_y_start;
                for line in &result.lines {
                    if line_y - line.height_mm < y_bottom + cell.padding.bottom_mm {
                        break;
                    }
                    for seg in &line.segments {
                        if seg.text.is_empty() { continue; }
                        let Some(font_ref) = ctx.get_font_ref(seg.style.bold, seg.style.italic) else { continue };
                        let x = col_x + cell.padding.left_mm + seg.x_offset_mm;
                        let _ = ctx.draw_text(&seg.text, x, line_y, fs, font_ref, &tc);
                    }
                    line_y -= line.height_mm;
                }
            }

            // Per-cell borders
            if !cell.borders.is_empty() {
                draw_cell_borders(ctx, &cell.borders, col_x, y_bottom, cell_w, row_h);
            }

            col_x += cell_w;
            col_idx += span;
        }

        // Row bottom border — only when the table style defines inner_border,
        // or when no table_style is set (legacy default: light separator).
        let row_border: Option<(f32, RgbColor)> = match &self.table_style {
            Some(ts) => ts.inner_border.as_ref().map(|b| {
                ((b.width_mm * 72.0 / 25.4) as f32, b.color.clone())
            }),
            None => Some((0.3_f32, RgbColor { r: 0.75, g: 0.75, b: 0.75 })),
        };
        if let Some((pt, color)) = row_border {
            if ctx.ua_config.enabled { ctx.backend.begin_artifact_content(); }
            let _ = ctx.backend.draw_line(x_base, y_bottom, x_base + total_w, y_bottom, pt, &color);
            if ctx.ua_config.enabled { ctx.backend.end_tagged_content(); }
        }
    }
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn draw_cell_borders(
    ctx: &mut RenderContext,
    borders: &CellBorders,
    x: f64, y: f64, w: f64, h: f64,
) {
    let ua = ctx.ua_config.enabled;
    if ua { ctx.backend.begin_artifact_content(); }
    let draw_h = |ctx: &mut RenderContext, border: &CellBorder, bx: f64, by: f64, len: f64| {
        let pt = (border.width_mm * 72.0 / 25.4) as f32;
        let _ = ctx.backend.draw_line(bx, by, bx + len, by, pt, &border.color);
    };
    let draw_v = |ctx: &mut RenderContext, border: &CellBorder, bx: f64, by: f64, len: f64| {
        let pt = (border.width_mm * 72.0 / 25.4) as f32;
        let _ = ctx.backend.draw_line(bx, by, bx, by + len, pt, &border.color);
    };
    if let Some(ref b) = borders.top    { draw_h(ctx, b, x,     y + h, w); }
    if let Some(ref b) = borders.bottom { draw_h(ctx, b, x,     y,     w); }
    if let Some(ref b) = borders.left   { draw_v(ctx, b, x,     y,     h); }
    if let Some(ref b) = borders.right  { draw_v(ctx, b, x + w, y,     h); }
    if ua { ctx.backend.end_tagged_content(); }
}

// ── UA helpers ────────────────────────────────────────────────────────────────

fn ua_tag_row(ctx: &mut RenderContext, is_header: bool) {
    let mcid = ctx.next_mcid();
    let cell_tag = if is_header { StructTag::TH } else { StructTag::TD };
    ctx.ua_begin_group(StructTag::TR, None);
    ctx.ua_begin_group(cell_tag, None);
    ctx.ua_content_ref(mcid);
    ctx.ua_end_group();
    ctx.ua_end_group();
    ctx.backend.begin_tagged_content(b"TR", mcid);
}

// ── Element impl ──────────────────────────────────────────────────────────────

impl Element for Table {
    fn estimated_height_mm(&self) -> f64 {
        let header_h = if self.headers.is_empty() && self.header_rows.is_empty() {
            0.0
        } else {
            Self::min_row_height_mm()
        };
        let rows_h: f64 = self.rows.iter().map(|r| match r.height {
            RowHeight::Exact(h) => h,
            RowHeight::AtLeast(h) => h.max(Self::min_row_height_mm()),
            RowHeight::Auto => Self::min_row_height_mm(),
        }).sum();
        header_h + rows_h
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let col_count = self.effective_col_count();
        if col_count == 0 {
            return Ok(RenderResult::done());
        }

        let usable_w = ctx.layout.content_width_mm;
        let x_base = ctx.layout.content_x_mm;
        let col_widths = self.col_widths_mm(usable_w, col_count);
        let start = ctx.resume_index;
        let ua = ctx.ua_config.enabled;

        let header_rows: Vec<&TableRow> = if !self.header_rows.is_empty() {
            self.header_rows.iter().collect()
        } else {
            Vec::new()
        };
        let simple_headers = self.header_rows.is_empty() && !self.headers.is_empty();
        let hdr_count = if simple_headers { 1 } else { header_rows.len() };

        let has_headers = simple_headers || !header_rows.is_empty();

        if ua && start == 0 {
            ctx.ua_begin_group(StructTag::Table, None);
            if has_headers { ctx.ua_begin_group(StructTag::THead, None); }
        }

        // On continuation pages: re-render headers
        if start > 0 {
            if simple_headers {
                let hdr_row = TableRow {
                    cells: self.headers.iter().map(TableCell::new).collect(),
                    height: RowHeight::Auto,
                    is_header: true,
                };
                let row_h = self.measure_row_height(&hdr_row, &col_widths, ctx);
                if ua { ua_tag_row(ctx, true); }
                self.render_row(&hdr_row, &col_widths, x_base, row_h, 0, true, ctx);
                if ua { ctx.backend.end_tagged_content(); }
                ctx.flow.advance(row_h);
            } else {
                for hdr in &header_rows {
                    let row_h = self.measure_row_height(hdr, &col_widths, ctx);
                    if ua { ua_tag_row(ctx, true); }
                    self.render_row(hdr, &col_widths, x_base, row_h, 0, true, ctx);
                    if ua { ctx.backend.end_tagged_content(); }
                    ctx.flow.advance(row_h);
                }
            }
        }

        // Render header rows on the first call (start == 0)
        if start == 0 {
            if simple_headers {
                let hdr_row = TableRow {
                    cells: self.headers.iter().map(TableCell::new).collect(),
                    height: RowHeight::Auto,
                    is_header: true,
                };
                let row_h = self.measure_row_height(&hdr_row, &col_widths, ctx);
                if ua { ua_tag_row(ctx, true); }
                self.render_row(&hdr_row, &col_widths, x_base, row_h, 0, true, ctx);
                if ua { ctx.backend.end_tagged_content(); }
                ctx.flow.advance(row_h);
            } else {
                for hdr in &header_rows {
                    let row_h = self.measure_row_height(hdr, &col_widths, ctx);
                    if ua { ua_tag_row(ctx, true); }
                    self.render_row(hdr, &col_widths, x_base, row_h, 0, true, ctx);
                    if ua { ctx.backend.end_tagged_content(); }
                    ctx.flow.advance(row_h);
                }
            }
        }

        if ua && start == 0 {
            if has_headers { ctx.ua_end_group(); } // THead
            ctx.ua_begin_group(StructTag::TBody, None);
        }

        // Body rows — resumable
        let body_start = start.saturating_sub(hdr_count);

        for (i, row) in self.rows.iter().enumerate().skip(body_start) {
            let row_h = self.measure_row_height(row, &col_widths, ctx);

            if ctx.flow.would_overflow(row_h) && i > body_start {
                ctx.resume_index = hdr_count + i;
                if ua {
                    ctx.ua_end_group(); // TBody
                    ctx.ua_end_group(); // Table
                }
                return Ok(RenderResult::more());
            }

            if ua { ua_tag_row(ctx, false); }
            self.render_row(row, &col_widths, x_base, row_h, i, false, ctx);
            if ua { ctx.backend.end_tagged_content(); }
            ctx.flow.advance(row_h);
        }

        if ua {
            ctx.ua_end_group(); // TBody
            ctx.ua_end_group(); // Table
        }
        Ok(RenderResult::done())
    }
}

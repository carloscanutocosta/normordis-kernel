use crate::styles::DocumentStyle;

/// Tracks the vertical cursor and page transitions during document rendering.
///
/// printpdf uses a bottom-left origin: Y increases upward.  The cursor starts
/// near the top of the page and decreases as content is added.
pub struct PageFlow {
    /// Current Y position in mm (bottom-left origin).
    pub cursor_y_mm: f64,
    pub page_height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_bottom_mm: f64,
    /// 1-based page counter.
    pub page_number: u32,
}

impl PageFlow {
    pub fn new(style: &DocumentStyle) -> Self {
        let (_, ph) = style.page_size.dimensions_mm();
        Self {
            cursor_y_mm: ph - style.margin_top_mm,
            page_height_mm: ph,
            margin_top_mm: style.margin_top_mm,
            margin_bottom_mm: style.margin_bottom_mm,
            page_number: 1,
        }
    }

    /// Returns `true` if placing `height_mm` content would cross the bottom margin.
    pub fn would_overflow(&self, height_mm: f64) -> bool {
        self.cursor_y_mm - height_mm < self.margin_bottom_mm
    }

    /// Moves the cursor down by `height_mm`. Caller must check `would_overflow` first.
    pub fn advance(&mut self, height_mm: f64) {
        self.cursor_y_mm -= height_mm;
    }

    /// Resets the cursor to the top of a new page and increments the page counter.
    pub fn new_page(&mut self) {
        self.cursor_y_mm = self.page_height_mm - self.margin_top_mm;
        self.page_number += 1;
    }

    /// Remaining vertical space on the current page in mm.
    pub fn remaining_mm(&self) -> f64 {
        self.cursor_y_mm - self.margin_bottom_mm
    }

    /// Returns `true` if placing `height_mm` content would overflow, accounting for
    /// footnote space reserved at the bottom of the current page.
    pub fn would_overflow_with_footnotes(
        &self,
        height_mm: f64,
        reserved_footnotes_mm: f64,
    ) -> bool {
        self.cursor_y_mm - height_mm < self.margin_bottom_mm + reserved_footnotes_mm
    }

    /// Returns `true` when the cursor is at (or within 0.5 mm of) the top margin.
    ///
    /// Used to suppress `space_before_mm` on the very first element of a page,
    /// matching Word's "suppress space before at top of page" behaviour.
    pub fn is_top_of_page(&self) -> bool {
        (self.page_height_mm - self.margin_top_mm - self.cursor_y_mm).abs() < 0.5
    }
}

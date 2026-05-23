use crate::styles::DocumentStyle;

/// Calculated printable area after subtracting margins from the page.
///
/// printpdf uses a bottom-left origin, so Y=0 is the bottom of the page and
/// Y=page_height_mm is the top. The flow cursor starts at `top_y_mm()` and
/// decreases as content is added.
#[derive(Debug, Clone)]
pub struct PageLayout {
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    /// Left edge of the content area (= left margin).
    pub content_x_mm: f64,
    pub content_width_mm: f64,
    pub content_height_mm: f64,
    pub margin_top_mm: f64,
    pub margin_bottom_mm: f64,
}

impl PageLayout {
    pub fn from_style(style: &DocumentStyle) -> Self {
        let (pw, ph) = style.page_size.dimensions_mm();
        Self {
            page_width_mm: pw,
            page_height_mm: ph,
            content_x_mm: style.margin_left_mm,
            content_width_mm: pw - style.margin_left_mm - style.margin_right_mm,
            content_height_mm: ph - style.margin_top_mm - style.margin_bottom_mm,
            margin_top_mm: style.margin_top_mm,
            margin_bottom_mm: style.margin_bottom_mm,
        }
    }

    /// Y coordinate of the top of the content area (cursor starting point).
    pub fn top_y_mm(&self) -> f64 {
        self.page_height_mm - self.margin_top_mm
    }

    /// Y coordinate of the bottom of the content area (page-break threshold).
    pub fn bottom_y_mm(&self) -> f64 {
        self.margin_bottom_mm
    }

    /// True when `cursor_y` has gone below the bottom margin.
    pub fn needs_new_page(&self, cursor_y: f64) -> bool {
        cursor_y < self.bottom_y_mm()
    }
}

use super::{Element, RenderContext};

/// Forces a page break before the next element.
///
/// Sets `ctx.force_page_break = true`; the document loop in `Document::render_to_bytes`
/// will detect this flag and start a new page.
#[derive(Debug, Clone)]
pub struct PageBreakElement;

impl Element for PageBreakElement {
    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        ctx.force_page_break = true;
        Ok(super::RenderResult::done())
    }

}

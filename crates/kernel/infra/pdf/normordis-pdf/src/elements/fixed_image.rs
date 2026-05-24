use super::{Element, LayoutMode, RenderContext};
use crate::layout::FixedBox;

/// Scaling / fitting mode for an image inside a `FixedImageBox`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ImageFit {
    /// Scale to fit entirely within the box, preserving aspect ratio.
    #[default]
    Contain,
    /// Scale to fill the box, preserving aspect ratio (may crop).
    Cover,
    /// Stretch to fill the box exactly (ignores aspect ratio).
    Stretch,
    /// Render at original pixel size; apply `OverflowPolicy` if it exceeds the box.
    Original,
}

/// An image element placed at a fixed position on the page.
///
/// Does not participate in `PageFlow`.
#[derive(Debug, Clone)]
pub struct FixedImageBox {
    pub image_box: FixedBox,
    /// Raw PNG or JPEG bytes.
    pub data: Vec<u8>,
    pub fit: ImageFit,
}

impl FixedImageBox {
    pub fn new(image_box: FixedBox, data: Vec<u8>) -> Self {
        Self {
            image_box,
            data,
            fit: ImageFit::Contain,
        }
    }

    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
}

impl Element for FixedImageBox {
    fn layout_mode(&self) -> LayoutMode {
        LayoutMode::Fixed(self.image_box.clone())
    }

    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        let ua = ctx.ua_config.enabled;
        if ua {
            match &self.image_box.ua_role {
                Some(tag) => {
                    let mcid = ctx.ua_tag_element(tag.clone(), self.image_box.ua_alt.clone());
                    ctx.backend
                        .begin_tagged_content(tag.pdf_name().as_bytes(), mcid);
                }
                None => {
                    ctx.backend.begin_artifact_content();
                }
            }
        }
        // TODO: decode image bytes, compute rendered dimensions per ImageFit,
        // create printpdf image op, place at image_box coordinates.
        if ua {
            ctx.backend.end_tagged_content();
        }
        Ok(super::RenderResult::done())
    }
}

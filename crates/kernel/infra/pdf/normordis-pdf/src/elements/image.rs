use serde::{Deserialize, Serialize};

use super::{Element, RenderContext};
use crate::compliance::ua::StructTag;

/// Horizontal alignment of an image within the content column.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ImageAlignment {
    Left,
    #[default]
    Center,
    Right,
}

/// An inline image with optional width/height constraints and a caption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageElement {
    /// Raw PNG or JPEG bytes.
    pub data: Vec<u8>,
    /// Desired rendered width in mm. `None` = use full content width.
    pub width_mm: Option<f64>,
    /// Desired rendered height in mm. `None` = calculate from aspect ratio.
    pub height_mm: Option<f64>,
    pub alignment: ImageAlignment,
    pub caption: Option<String>,
    /// Alternative text for PDF/UA-2 accessibility.
    /// None = image is treated as decorative Artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Width as a percentage of the content column (1–100). Resolved at render
    /// time using `ctx.layout.content_width_mm`. `None` = full content width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_percent: Option<f64>,
}

impl ImageElement {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            width_mm: None,
            height_mm: None,
            alignment: ImageAlignment::Center,
            caption: None,
            alt: None,
            width_percent: None,
        }
    }

    pub fn width(mut self, mm: f64) -> Self {
        self.width_mm = Some(mm);
        self
    }

    pub fn width_mm(mut self, mm: f64) -> Self {
        self.width_mm = Some(mm);
        self
    }

    pub fn height(mut self, mm: f64) -> Self {
        self.height_mm = Some(mm);
        self
    }

    pub fn align(mut self, alignment: ImageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Set alternative text for PDF/UA-2 accessibility.
    ///
    /// Required for meaningful images when PDF/UA-2 is active.
    /// Images without alt text are marked as decorative Artifact.
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.alt = Some(text.into());
        self
    }
}

impl Element for ImageElement {
    fn estimated_height_mm(&self) -> f64 {
        // Use explicit height or a sensible default; caption adds ~5mm
        let img_h = self.height_mm.unwrap_or(50.0);
        img_h + if self.caption.is_some() { 5.0 } else { 0.0 }
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        if ctx.ua_enabled() {
            match &self.alt {
                Some(alt_text) => {
                    let mcid = ctx.ua_tag_element(StructTag::Figure, Some(alt_text.clone()));
                    ctx.backend.begin_tagged_content(b"Figure", mcid);
                }
                None => {
                    if ctx.ua_config.warn_missing_alt {
                        eprintln!(
                            "PDF/UA-2 WARNING: ImageElement without alt text — \
                             marking as Artifact. Add .alt() for accessible images."
                        );
                    }
                    ctx.backend.begin_artifact_content();
                }
            }
        }

        // TODO: decode image with the `image` crate, determine aspect ratio,
        // create image XObject, place at correct x/y with alignment offset
        ctx.flow.advance(self.estimated_height_mm());

        if ctx.ua_enabled() {
            ctx.backend.end_tagged_content();
        }

        Ok(super::RenderResult::done())
    }
}

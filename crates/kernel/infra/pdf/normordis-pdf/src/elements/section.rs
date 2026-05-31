use serde::{Deserialize, Serialize};

use super::{Element, RenderContext};
use crate::{compliance::ua::StructTag, styles::StyleResolver};

/// A section heading at one of three nesting levels.
///
/// Level 1 uses `DocumentStyle.font_size_title` and `primary_color`.
/// Level 2 uses `font_size_section`.
/// Level 3 uses `font_size_body` with bold weight.
///
/// A `style_ref` can point to a named style (e.g. `"heading_1"`) to override
/// the default level-based sizing and spacing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    /// 1 = main title, 2 = subtitle, 3 = sub-section.
    pub level: u8,
    /// Optional named style override. When set, resolved style provides font size,
    /// spacing, and colour. Falls back to the built-in heading_1/2/3 when `None`.
    #[serde(default)]
    pub style_ref: Option<String>,
}

impl Section {
    pub fn new(title: impl Into<String>, level: u8) -> Self {
        Self {
            title: title.into(),
            level: level.clamp(1, 3),
            style_ref: None,
        }
    }

    /// Apply a named style reference.
    pub fn style(mut self, name: impl Into<String>) -> Self {
        self.style_ref = Some(name.into());
        self
    }

    fn default_style_name(&self) -> &'static str {
        match self.level {
            1 => "heading_1",
            2 => "heading_2",
            _ => "heading_3",
        }
    }
}

impl Section {
    /// Returns the heading level (1–3).
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Returns the heading text.
    pub fn heading_text(&self) -> &str {
        &self.title
    }
}

impl Element for Section {
    fn as_section_info(&self) -> Option<(u8, &str)> {
        Some((self.level, &self.title))
    }

    fn estimated_height_mm(&self) -> f64 {
        let pre = match self.level {
            1 => 8.0,
            2 => 6.0,
            _ => 4.0,
        };
        let post = match self.level {
            1 => 4.0,
            2 => 3.0,
            _ => 2.0,
        };
        pre + 7.0 + post
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<super::RenderResult> {
        let style_name = self
            .style_ref
            .as_deref()
            .unwrap_or_else(|| self.default_style_name());

        let resolver = StyleResolver::new(&ctx.style.named_styles, &ctx.style);
        let resolved = resolver.resolve(style_name)?;

        let fs = resolved.font_size;

        if resolved.space_before_mm > 0.0 && !ctx.flow.is_top_of_page() {
            ctx.flow.advance(resolved.space_before_mm);
        }

        let color = resolved
            .color
            .clone()
            .unwrap_or_else(|| ctx.style.text_color.clone());

        let Some(font_ref) = ctx.get_font_ref(resolved.bold, resolved.italic) else {
            ctx.flow
                .advance(self.estimated_height_mm() - resolved.space_before_mm);
            return Ok(super::RenderResult::done());
        };

        // UA-2 heading tag
        let ua_tag = match self.level {
            1 => StructTag::H1,
            2 => StructTag::H2,
            3 => StructTag::H3,
            4 => StructTag::H4,
            5 => StructTag::H5,
            _ => StructTag::H6,
        };
        if ctx.ua_enabled() {
            if let Some(prev) = ctx.last_heading_level {
                if self.level > prev + 1 {
                    eprintln!(
                        "PDF/UA-2 WARNING: heading level skipped from H{} to H{}",
                        prev, self.level
                    );
                }
            }
            ctx.last_heading_level = Some(self.level);
        }

        let mcid_opt = if ctx.ua_enabled() {
            let mcid = ctx.ua_tag_element(ua_tag, None);
            ctx.backend.begin_tagged_content(
                match self.level {
                    1 => b"H1",
                    2 => b"H2",
                    3 => b"H3",
                    4 => b"H4",
                    5 => b"H5",
                    _ => b"H6",
                },
                mcid,
            );
            Some(mcid)
        } else {
            None
        };

        let x = ctx.layout.content_x_mm;
        let y = ctx.flow.cursor_y_mm;
        ctx.draw_text(&self.title, x, y, fs, font_ref, &color)?;

        // Record outline entry so the bookmarks panel navigates here.
        let page_idx = ctx.backend.current_page_idx();
        ctx.backend
            .add_outline_entry(&self.title, self.level, page_idx, y);

        let line_h = ctx.layout_engine.line_height_mm(&ctx.fonts, fs);
        ctx.flow.advance(line_h + resolved.space_after_mm);

        if mcid_opt.is_some() {
            ctx.backend.end_tagged_content();
        }

        Ok(super::RenderResult::done())
    }
}

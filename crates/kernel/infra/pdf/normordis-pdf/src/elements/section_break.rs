use serde::{Deserialize, Serialize};

use super::{Element, RenderContext, RenderResult};

/// Page orientation for a document section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Per-section page margins in mm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMargins {
    pub top_mm: f64,
    pub bottom_mm: f64,
    pub left_mm: f64,
    pub right_mm: f64,
}

impl SectionMargins {
    pub fn uniform(mm: f64) -> Self {
        Self {
            top_mm: mm,
            bottom_mm: mm,
            left_mm: mm,
            right_mm: mm,
        }
    }

    pub fn symmetric(vertical_mm: f64, horizontal_mm: f64) -> Self {
        Self {
            top_mm: vertical_mm,
            bottom_mm: vertical_mm,
            left_mm: horizontal_mm,
            right_mm: horizontal_mm,
        }
    }
}

impl Default for SectionMargins {
    fn default() -> Self {
        Self::symmetric(25.0, 25.0)
    }
}

/// Forces a section break: starts a new page with a different orientation
/// or margin set.
///
/// This element forces a page break (via `force_page_break`) and stores the
/// new orientation and margins so the document loop can apply them when it
/// creates the next page.  Because the document loop controls page creation,
/// the actual size change is handled there; this element merely signals the
/// intent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectionBreak {
    /// New page orientation for the section that follows.
    pub orientation: Orientation,
    /// Per-section margin override.  `None` keeps the current document margins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margins: Option<SectionMargins>,
}

impl SectionBreak {
    pub fn portrait() -> Self {
        Self {
            orientation: Orientation::Portrait,
            margins: None,
        }
    }

    pub fn landscape() -> Self {
        Self {
            orientation: Orientation::Landscape,
            margins: None,
        }
    }

    pub fn with_margins(mut self, margins: SectionMargins) -> Self {
        self.margins = Some(margins);
        self
    }
}

impl Element for SectionBreak {
    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        // Signal the document loop to start a new page.
        ctx.force_page_break = true;
        Ok(RenderResult::done())
    }
}

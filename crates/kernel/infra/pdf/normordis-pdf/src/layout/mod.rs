pub mod engine;
pub mod fixed_box;
pub mod knuth_plass;
pub mod line;
pub mod page_flow;

pub use engine::{LayoutResult, TabStop, TabStopAlign, TextLayoutEngine};
pub use knuth_plass::{KnuthPlassOptimizer, WordBox};
pub use fixed_box::{BorderStyle, BoxBorder, FixedBox, OverflowPolicy};
pub use line::{LineBox, LineSegment};
pub use page_flow::PageFlow;

// Convenience re-exports so callers can import text-run types from `layout::`.
pub use crate::richtext::marks::{
    AppliedStyle, DecorationLine, GlyphUsageTracker, HighlightColor,
    LineBreakingMode, OpenTypeFeatures, TextDecoration, TextRun,
};

use serde::{Deserialize, Serialize};

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Justify,
    Center,
    /// Right-aligned. For dates, numeric columns, and letter headings.
    Right,
}

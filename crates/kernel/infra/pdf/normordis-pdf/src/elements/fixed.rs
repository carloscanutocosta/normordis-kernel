//! Unified re-exports for all fixed-position element types.
//!
//! Fixed elements are placed at absolute coordinates and do not affect
//! the `PageFlow` cursor. Import from here or from the individual sub-modules.

pub use super::fixed_image::{FixedImageBox, ImageFit};
pub use super::fixed_line::FixedLineElement;
pub use super::fixed_text::{FixedTextBox, VerticalAlign};

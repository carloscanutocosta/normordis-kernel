pub mod converter;
pub mod marks;
pub mod model;

pub use model::NcrtfDocument;

/// NCRTF format version supported by this release.
pub const NCRTF_VERSION: &str = "1.3.0";

use crate::{elements::Element, styles::DocumentStyle, NormaxisPdfError, Result};

/// Parse a JSON string as an NCRTF v1.0 document.
///
/// # Errors
///
/// Returns [`NormaxisPdfError::ParseError`] if the JSON is invalid or does not
/// conform to the NCRTF v1.0 schema.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::parse_ncrtf;
///
/// let json = r#"{"ncrtf":"1.0","blocks":[]}"#;
/// let doc = parse_ncrtf(json).unwrap();
/// assert_eq!(doc.ncrtf, "1.0");
/// ```
pub fn parse_ncrtf(json: &str) -> Result<NcrtfDocument> {
    serde_json::from_str(json).map_err(|e| NormaxisPdfError::ParseError(e.to_string()))
}

/// Convert a parsed `NcrtfDocument` into renderable `normordis-pdf` elements.
pub fn ncrtf_to_elements(doc: &NcrtfDocument, style: &DocumentStyle) -> Vec<Box<dyn Element>> {
    converter::ncrtf_to_elements(doc, style)
}

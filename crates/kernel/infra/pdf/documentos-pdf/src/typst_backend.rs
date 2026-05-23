pub use render_typst::pdf::WarmFonts;

use crate::PdfError;

pub fn render_typst_document(
    source: &str,
    extra_files: &[(String, Vec<u8>)],
    warm: &WarmFonts,
) -> Result<Vec<u8>, PdfError> {
    render_typst::pdf::compile_with_warm_fonts_and_files(source, warm, extra_files)
        .map(|r| r.pdf_bytes)
        .map_err(|e| PdfError(e.to_string()))
}

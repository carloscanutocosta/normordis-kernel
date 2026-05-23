pub use normordis_pdf::template::NdtData;
pub use normordis_pdf::{CompileOptions, NdfDocument, PreparedPdf, SignatureOptions};

use crate::PdfError;

fn map_err(e: impl std::fmt::Display) -> PdfError {
    PdfError(e.to_string())
}

/// Compila NDT template + dados → NdfDocument (pipeline auditável).
pub fn compile_ndt(
    ndt: &str,
    data: &NdtData,
    options: CompileOptions,
) -> Result<NdfDocument, PdfError> {
    normordis_pdf::compile_ndt(ndt, data, options).map_err(map_err)
}

/// Renderiza NDF JSON → PDF bytes.
pub fn render_ndf(ndf_json: &str) -> Result<Vec<u8>, PdfError> {
    normordis_pdf::render_ndf(ndf_json).map_err(map_err)
}

/// NDT + dados → PDF em passo único (sem acesso ao NDF intermédio).
pub fn render_ndt(ndt: &str, data: &NdtData, options: CompileOptions) -> Result<Vec<u8>, PdfError> {
    let ndf = compile_ndt(ndt, data, options)?;
    let ndf_json = ndf.to_canonical_json().map_err(map_err)?;
    render_ndf(&ndf_json)
}

/// Prepara PDF para assinatura digital (dois passos: prepare → embed_signature).
pub fn render_ndf_for_signing(
    ndf_json: &str,
    options: SignatureOptions,
) -> Result<PreparedPdf, PdfError> {
    normordis_pdf::render_ndf_prepared_for_signing(ndf_json, options).map_err(map_err)
}

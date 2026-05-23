#[cfg(feature = "typst")]
pub mod richtext;

#[cfg(feature = "typst")]
mod typst_backend;
#[cfg(feature = "typst")]
pub use typst_backend::{render_typst_document, WarmFonts};

#[cfg(feature = "normordis")]
mod normordis_backend;
#[cfg(feature = "normordis")]
pub use normordis_backend::{compile_ndt, render_ndf, render_ndf_for_signing, render_ndt};

pub use support_pdf::PdfError;

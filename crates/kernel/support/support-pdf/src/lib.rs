use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct PdfError(pub String);

pub trait PdfRenderer: Send + Sync {
    fn render(&self, source: &str) -> Result<Vec<u8>, PdfError>;
}

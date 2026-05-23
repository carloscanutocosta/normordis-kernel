use thiserror::Error;

/// All errors that can occur during PDF generation.
#[derive(Debug, Error)]
pub enum NormaxisPdfError {
    #[error("font load error: {0}")]
    FontLoadError(String),

    #[error("image load error: {0}")]
    ImageLoadError(String),

    #[error("render error: {0}")]
    RenderError(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("template error: {0}")]
    Template(String),

    #[error("cycle detected in style inheritance chain: '{0}'")]
    StyleCycleError(String),

    #[error("unknown style name: '{0}'")]
    UnknownStyle(String),

    #[error("NDF integrity error: {0}")]
    NdfIntegrityError(String),

    #[error("NDF audit chain error: {0}")]
    NdfAuditError(String),

    #[error("NDF revision error: {0}")]
    NdfRevisionError(String),

    #[error("NDF compile error: {0}")]
    NdfCompileError(String),

    #[error("PDF/UA-2 accessibility error: {0}")]
    AccessibilityError(String),

    #[error("serialisation error: {0}")]
    SerdeError(String),
}

pub type Result<T> = std::result::Result<T, NormaxisPdfError>;

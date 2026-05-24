pub mod error;
pub mod service;

pub use error::ScanIngestError;
pub use service::{
    ingest_scanned_document, scanned_document_ingest_config, AlwaysCleanScanner, ScanIngestRequest,
    SCANNED_DOCUMENT_KIND,
};

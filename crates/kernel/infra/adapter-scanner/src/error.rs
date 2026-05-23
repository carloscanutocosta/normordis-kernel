use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const SCANNER_COMPONENT: &str = "adapter-scanner";
pub const DEVICE_NOT_FOUND: &str = "MINI.SCAN.DEVICE_NOT_FOUND";
pub const DEVICE_BUSY: &str = "MINI.SCAN.DEVICE_BUSY";
pub const FORMAT_NOT_SUPPORTED: &str = "MINI.SCAN.FORMAT_NOT_SUPPORTED";
pub const SOURCE_NOT_SUPPORTED: &str = "MINI.SCAN.SOURCE_NOT_SUPPORTED";
pub const HTTP_ERROR: &str = "MINI.SCAN.HTTP_ERROR";
pub const XML_PARSE_ERROR: &str = "MINI.SCAN.XML_PARSE_ERROR";
pub const JOB_FAILED: &str = "MINI.SCAN.JOB_FAILED";
pub const TIMEOUT: &str = "MINI.SCAN.TIMEOUT";
pub const NETWORK_ERROR: &str = "MINI.SCAN.NETWORK_ERROR";
pub const INVALID_CONFIG: &str = "MINI.SCAN.INVALID_CONFIG";

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("device not found at {host}:{port}")]
    DeviceNotFound { host: String, port: u16 },

    #[error("device busy or in error state: {state}")]
    DeviceBusy { state: String },

    #[error("format not supported by device: {format}")]
    FormatNotSupported { format: String },

    #[error("scan source not supported: {kind}")]
    SourceNotSupported { kind: String },

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("error parsing XML response: {0}")]
    XmlParseError(String),

    #[error("scan job failed: {reason}")]
    JobFailed { reason: String },

    #[error("timeout: scan took longer than {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Internal only — used in the polling loop, never exposed to callers.
    #[doc(hidden)]
    #[error("document not yet available")]
    NotReady,
}

impl ScannerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DeviceNotFound { .. } => DEVICE_NOT_FOUND,
            Self::DeviceBusy { .. } => DEVICE_BUSY,
            Self::FormatNotSupported { .. } => FORMAT_NOT_SUPPORTED,
            Self::SourceNotSupported { .. } => SOURCE_NOT_SUPPORTED,
            Self::HttpError { .. } => HTTP_ERROR,
            Self::XmlParseError(_) => XML_PARSE_ERROR,
            Self::JobFailed { .. } => JOB_FAILED,
            Self::Timeout { .. } => TIMEOUT,
            Self::NetworkError(_) => NETWORK_ERROR,
            Self::InvalidConfig(_) => INVALID_CONFIG,
            Self::NotReady => TIMEOUT,
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("adapter-scanner error codes are valid"),
            Component::new(SCANNER_COMPONENT).expect("adapter-scanner component is valid"),
            self.to_string(),
        )
    }
}

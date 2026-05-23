pub mod client;
pub mod error;
pub mod types;
pub mod xml;

#[cfg(feature = "discovery")]
pub mod discovery;

pub use client::ScannerClient;
pub use error::{
    ScannerError, DEVICE_BUSY, DEVICE_NOT_FOUND, FORMAT_NOT_SUPPORTED, HTTP_ERROR, INVALID_CONFIG,
    JOB_FAILED, NETWORK_ERROR, SCANNER_COMPONENT, SOURCE_NOT_SUPPORTED, TIMEOUT, XML_PARSE_ERROR,
};
pub use types::{
    ColorMode, InputCapabilities, ScanCapabilities, ScanFormat, ScanIntent, ScanRegion, ScanSettings,
    ScanSource, ScannerClientConfig, ScannerDevice, ScannerState, ScannedDocument,
};
pub use xml::{build_scan_settings_xml, parse_capabilities, parse_scanner_state};

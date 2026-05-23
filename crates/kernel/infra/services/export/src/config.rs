use serde::{Deserialize, Serialize};

use crate::{ExportAdapterError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Csv,
    Xml,
    Sqlite,
    Xlsx,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Csv => "csv",
            Provider::Xml => "xml",
            Provider::Sqlite => "sqlite",
            Provider::Xlsx => "xlsx",
        }
    }
}

impl From<core_exports::ExportFormat> for Provider {
    fn from(value: core_exports::ExportFormat) -> Self {
        match value {
            core_exports::ExportFormat::Csv => Self::Csv,
            core_exports::ExportFormat::Xml => Self::Xml,
            core_exports::ExportFormat::Sqlite => Self::Sqlite,
            core_exports::ExportFormat::Xlsx => Self::Xlsx,
        }
    }
}

impl From<Provider> for core_exports::ExportFormat {
    fn from(value: Provider) -> Self {
        match value {
            Provider::Csv => Self::Csv,
            Provider::Xml => Self::Xml,
            Provider::Sqlite => Self::Sqlite,
            Provider::Xlsx => Self::Xlsx,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub format: Provider,
    pub hash_algorithm: String,
}

impl Config {
    pub fn new(format: Provider) -> Self {
        Self {
            format,
            hash_algorithm: "sha256".into(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if !self.hash_algorithm.eq_ignore_ascii_case("sha256") {
            return Err(ExportAdapterError::InvalidHashAlgorithm(
                self.hash_algorithm.clone(),
            ));
        }
        Ok(())
    }
}

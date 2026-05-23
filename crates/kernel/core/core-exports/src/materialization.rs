use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ExportError, ExportSnapshot};

pub type TabularRow = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Xml,
    Sqlite,
    Xlsx,
}

impl ExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xml => "xml",
            Self::Sqlite => "sqlite",
            Self::Xlsx => "xlsx",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteroperabilityProfile {
    Audit,
    Exchange,
    Reporting,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularDataset {
    pub columns: Vec<String>,
    pub rows: Vec<TabularRow>,
}

impl TabularDataset {
    pub fn validate(&self) -> Result<(), ExportError> {
        if self.columns.is_empty() {
            return Err(ExportError::MissingField {
                field: "columns".into(),
            });
        }
        if self.rows.is_empty() {
            return Err(ExportError::MissingField {
                field: "rows".into(),
            });
        }
        for column in &self.columns {
            if column.trim().is_empty() {
                return Err(ExportError::MissingField {
                    field: "columns[]".into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportMaterializationRequest {
    pub snapshot_id: String,
    pub format: ExportFormat,
    pub profile: InteroperabilityProfile,
    pub dataset: TabularDataset,
    pub snapshot: Option<ExportSnapshot>,
    pub output_ref: String,
    pub root_name: Option<String>,
    pub sheet_name: Option<String>,
    pub table_name: Option<String>,
}

impl ExportMaterializationRequest {
    pub fn validate(&self) -> Result<(), ExportError> {
        if self.snapshot_id.trim().is_empty() {
            return Err(ExportError::MissingField {
                field: "snapshot_id".into(),
            });
        }
        if self.output_ref.trim().is_empty() {
            return Err(ExportError::MissingField {
                field: "output_ref".into(),
            });
        }
        self.dataset.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtefact {
    pub kind: String,
    pub output_ref: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportMaterializationResult {
    pub format: ExportFormat,
    pub artefacts: Vec<ExportArtefact>,
}

pub trait ExportMaterializerPort {
    fn materialize(
        &self,
        request: &ExportMaterializationRequest,
    ) -> Result<ExportMaterializationResult, ExportError>;
}

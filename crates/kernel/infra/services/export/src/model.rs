use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_exports::ExportSnapshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{ExportAdapterError, Provider, Result};

pub type ExportRow = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRequest {
    pub snapshot_id: String,
    pub snapshot: Option<ExportSnapshot>,
    pub rows: Vec<ExportRow>,
    pub columns: Vec<String>,
    pub root_name: Option<String>,
    pub sheet_name: Option<String>,
    pub table_name: Option<String>,
    pub output_path: PathBuf,
}

impl ExportRequest {
    pub fn validate(&self) -> Result<()> {
        if self.snapshot_id.trim().is_empty() {
            return Err(ExportAdapterError::EmptyField("snapshot_id"));
        }
        if self.snapshot.is_none() && self.rows.is_empty() {
            return Err(ExportAdapterError::InvalidRequest(
                "snapshot ou rows obrigatorios".into(),
            ));
        }
        if self.output_path.as_os_str().is_empty() {
            return Err(ExportAdapterError::EmptyField("output_path"));
        }
        Ok(())
    }

    pub fn effective_columns(&self) -> Vec<String> {
        if !self.columns.is_empty() {
            return self.columns.clone();
        }
        let mut columns = BTreeMap::new();
        for row in self.effective_rows() {
            for key in row.keys() {
                columns.insert(key.clone(), ());
            }
        }
        columns.into_keys().collect()
    }

    pub fn effective_rows(&self) -> Vec<ExportRow> {
        if !self.rows.is_empty() {
            return self.rows.clone();
        }
        self.snapshot
            .as_ref()
            .map(snapshot_to_row)
            .into_iter()
            .collect()
    }

    pub fn root_name(&self) -> &str {
        self.root_name.as_deref().unwrap_or("export")
    }

    pub fn sheet_name(&self) -> &str {
        self.sheet_name.as_deref().unwrap_or("Export")
    }

    pub fn table_name(&self) -> &str {
        self.table_name.as_deref().unwrap_or("export_rows")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artefact {
    pub kind: String,
    pub path: PathBuf,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub provider: Provider,
    pub output_path: PathBuf,
    pub artefacts: Vec<Artefact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportResult {
    pub provider: Provider,
    pub artefacts: Vec<Artefact>,
}

pub fn build_single_artefact_plan(
    provider: Provider,
    output_path: impl AsRef<Path>,
    payload: &[u8],
) -> Plan {
    Plan {
        provider,
        output_path: output_path.as_ref().to_path_buf(),
        artefacts: vec![Artefact {
            kind: format!("snapshot_{}", provider.as_str()),
            path: output_path.as_ref().to_path_buf(),
            hash: sha256_hex(payload),
        }],
    }
}

pub fn payload_bytes(req: &ExportRequest) -> Result<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!({
        "snapshot_id": req.snapshot_id,
        "snapshot": req.snapshot,
        "rows": req.rows,
        "columns": req.columns,
        "root_name": req.root_name,
        "sheet_name": req.sheet_name,
        "table_name": req.table_name,
    }))
    .map_err(Into::into)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn snapshot_to_row(snapshot: &ExportSnapshot) -> ExportRow {
    let mut row = ExportRow::new();
    row.insert(
        "snapshot_id".into(),
        Value::String(snapshot.snapshot_id.clone()),
    );
    row.insert(
        "exported_at".into(),
        Value::String(snapshot.exported_at.to_rfc3339()),
    );
    row.insert(
        "source_kind".into(),
        Value::String(snapshot.source.kind.clone()),
    );
    row.insert(
        "source_subject_id".into(),
        Value::String(snapshot.source.subject_id.clone()),
    );
    row.insert(
        "source_version".into(),
        Value::String(snapshot.source.version.clone()),
    );
    row.insert(
        "manifest_algorithm".into(),
        Value::String(snapshot.manifest.algorithm.clone()),
    );
    row.insert(
        "manifest_hash".into(),
        Value::String(snapshot.manifest.hash.clone()),
    );
    row.insert(
        "manifest_item_count".into(),
        Value::Number(snapshot.manifest.item_count.into()),
    );
    row.insert(
        "document_id".into(),
        Value::String(snapshot.document_package.document_id.clone()),
    );
    row
}

use std::collections::BTreeMap;

use core_exports::{
    ExportFormat, ExportMaterializationRequest, ExportSnapshot, InteroperabilityProfile,
    TabularDataset, TabularRow,
};
use serde_json::Value;

use crate::{InteroperabilityError, Result};

#[derive(Debug, Clone)]
pub struct ExportRequestBuilder {
    snapshot_id: String,
    format: ExportFormat,
    profile: InteroperabilityProfile,
    rows: Vec<TabularRow>,
    columns: Vec<String>,
    snapshot: Option<ExportSnapshot>,
    output_ref: String,
    root_name: Option<String>,
    sheet_name: Option<String>,
    table_name: Option<String>,
}

impl ExportRequestBuilder {
    pub fn new(
        snapshot_id: impl Into<String>,
        format: ExportFormat,
        output_ref: impl Into<String>,
    ) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            format,
            profile: InteroperabilityProfile::Exchange,
            rows: Vec::new(),
            columns: Vec::new(),
            snapshot: None,
            output_ref: output_ref.into(),
            root_name: None,
            sheet_name: None,
            table_name: None,
        }
    }

    pub fn profile(mut self, profile: InteroperabilityProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn columns(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    pub fn row(mut self, row: TabularRow) -> Self {
        self.rows.push(row);
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = TabularRow>) -> Self {
        self.rows.extend(rows);
        self
    }

    pub fn snapshot(mut self, snapshot: ExportSnapshot) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    pub fn root_name(mut self, root_name: impl Into<String>) -> Self {
        self.root_name = Some(root_name.into());
        self
    }

    pub fn sheet_name(mut self, sheet_name: impl Into<String>) -> Self {
        self.sheet_name = Some(sheet_name.into());
        self
    }

    pub fn table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }

    pub fn build(self) -> Result<ExportMaterializationRequest> {
        let rows = if self.rows.is_empty() {
            self.snapshot
                .as_ref()
                .map(snapshot_to_row)
                .into_iter()
                .collect()
        } else {
            self.rows
        };
        let columns = if self.columns.is_empty() {
            infer_columns(&rows)
        } else {
            self.columns
        };
        if columns.is_empty() {
            return Err(InteroperabilityError::EmptyField("columns"));
        }
        if rows.is_empty() {
            return Err(InteroperabilityError::EmptyField("rows"));
        }

        let request = ExportMaterializationRequest {
            snapshot_id: self.snapshot_id,
            format: self.format,
            profile: self.profile,
            dataset: TabularDataset { columns, rows },
            snapshot: self.snapshot,
            output_ref: self.output_ref,
            root_name: self.root_name,
            sheet_name: self.sheet_name,
            table_name: self.table_name,
        };
        request.validate()?;
        Ok(request)
    }
}

fn infer_columns(rows: &[TabularRow]) -> Vec<String> {
    let mut columns = BTreeMap::new();
    for row in rows {
        for key in row.keys() {
            columns.insert(key.clone(), ());
        }
    }
    columns.into_keys().collect()
}

fn snapshot_to_row(snapshot: &ExportSnapshot) -> TabularRow {
    let mut row = TabularRow::new();
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

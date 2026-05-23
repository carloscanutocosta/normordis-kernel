use std::fs;

use serde_json::Value;

use crate::{ExportRequest, Result};

pub struct CsvExportAdapter;

impl CsvExportAdapter {
    pub fn render(req: &ExportRequest) -> String {
        let columns = req.effective_columns();
        let mut lines = Vec::with_capacity(req.rows.len() + 1);
        lines.push(
            columns
                .iter()
                .map(|column| csv_field(column))
                .collect::<Vec<_>>()
                .join(","),
        );

        for row in req.effective_rows() {
            lines.push(
                columns
                    .iter()
                    .map(|column| csv_field(&value_to_cell(row.get(column))))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        lines.join("\r\n")
    }

    pub fn export(req: &ExportRequest) -> Result<()> {
        if let Some(parent) = req.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&req.output_path, Self::render(req))?;
        Ok(())
    }
}

pub(crate) fn value_to_cell(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(v)) => v.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(v @ Value::Array(_)) | Some(v @ Value::Object(_)) => v.to_string(),
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

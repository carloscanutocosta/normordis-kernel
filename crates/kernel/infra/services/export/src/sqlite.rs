use std::fs;

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use rusqlite::{params_from_iter, types::Value as SqlValue};

use crate::csv::value_to_cell;
use crate::{ExportAdapterError, ExportRequest, Result};

pub struct SqliteExportAdapter;

impl SqliteExportAdapter {
    pub fn export(req: &ExportRequest) -> Result<()> {
        if let Some(parent) = req.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let config = SqliteRelationalConfig::read_write_create(&req.output_path);
        let conn = open_relational_connection(&config)
            .map_err(|e| ExportAdapterError::Infra(format!("abrir base SQLite de export: {e}")))?;
        run_relational_migrations(&conn, &[EXPORT_METADATA_MIGRATION]).map_err(|e| {
            ExportAdapterError::Infra(format!("migrar metadata SQLite de export: {e}"))
        })?;

        let table = sql_ident(req.table_name());
        let columns = req.effective_columns();
        let column_defs = columns
            .iter()
            .map(|column| format!("{} TEXT", sql_ident(column)))
            .collect::<Vec<_>>()
            .join(", ");
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {table} ({column_defs});"
        ))?;

        conn.execute(
            "INSERT OR REPLACE INTO export_metadata
                (snapshot_id, provider, table_name, column_json)
             VALUES (?1, 'sqlite', ?2, ?3)",
            (
                &req.snapshot_id,
                req.table_name(),
                serde_json::to_string(&columns)?,
            ),
        )?;

        let placeholders = (0..columns.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let insert_sql = format!(
            "INSERT INTO {table} ({}) VALUES ({placeholders})",
            columns
                .iter()
                .map(|column| sql_ident(column))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for row in req.effective_rows() {
            let values = columns
                .iter()
                .map(|column| SqlValue::Text(value_to_cell(row.get(column))))
                .collect::<Vec<_>>();
            conn.execute(&insert_sql, params_from_iter(values))?;
        }

        Ok(())
    }
}

const EXPORT_METADATA_MIGRATION: &str = r#"
    CREATE TABLE IF NOT EXISTS export_metadata (
        snapshot_id TEXT PRIMARY KEY,
        provider TEXT NOT NULL,
        table_name TEXT NOT NULL,
        column_json TEXT NOT NULL,
        exported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
"#;

fn sql_ident(raw: &str) -> String {
    let mut ident = String::new();
    for (idx, ch) in raw.trim().chars().enumerate() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_';
        if valid && !(idx == 0 && ch.is_ascii_digit()) {
            ident.push(ch);
        } else if valid {
            ident.push('_');
            ident.push(ch);
        } else {
            ident.push('_');
        }
    }
    if ident.is_empty() {
        ident.push_str("export_rows");
    }
    format!("\"{}\"", ident.replace('"', "\"\""))
}

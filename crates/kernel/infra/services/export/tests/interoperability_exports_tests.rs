use std::fs;

use core_exports::{ExportFormat, TabularRow};
use infra_export::RuntimeExporter;
use interoperability::{
    AllowAllExportAuthorization, ExportAuthorizationContext, ExportRequestBuilder,
    InteroperabilityExportService,
};
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

fn sample_row() -> TabularRow {
    let mut row = TabularRow::new();
    row.insert("id".into(), json!("PT-001"));
    row.insert("nome".into(), json!("Interoperabilidade"));
    row.insert("valor".into(), json!(42));
    row
}

fn auth_context() -> ExportAuthorizationContext {
    ExportAuthorizationContext {
        actor: "user:interop-test".into(),
        purpose: "interoperability-export-test".into(),
        correlation_id: "corr-interop-001".into(),
    }
}

#[test]
fn support_interoperability_exporta_para_todos_os_formatos_suportados() {
    let dir = tempdir().unwrap();

    for (format, filename) in [
        (ExportFormat::Csv, "dataset.csv"),
        (ExportFormat::Xml, "dataset.xml"),
        (ExportFormat::Sqlite, "dataset.sqlite"),
        (ExportFormat::Xlsx, "dataset.xlsx"),
    ] {
        let output = dir.path().join(filename);
        let request =
            ExportRequestBuilder::new("exp:interop:dataset:v1", format, output.to_string_lossy())
                .columns(["id", "nome", "valor"])
                .row(sample_row())
                .root_name("interoperability_export")
                .sheet_name("Dados")
                .table_name("interop_rows")
                .build()
                .unwrap();

        let service = InteroperabilityExportService::new(
            RuntimeExporter::new(format.into()),
            AllowAllExportAuthorization,
        );
        let result = service.export(&auth_context(), &request).unwrap();

        assert_eq!(result.format, format);
        assert_eq!(result.artefacts.len(), 1);
        assert!(result.artefacts[0].hash.starts_with("sha256:"));
        assert!(output.exists(), "output em falta para {format:?}");

        match format {
            ExportFormat::Csv => {
                let csv = fs::read_to_string(output).unwrap();
                assert!(csv.contains("id,nome,valor"));
                assert!(csv.contains("PT-001"));
            }
            ExportFormat::Xml => {
                let xml = fs::read_to_string(output).unwrap();
                assert!(xml.contains("<interoperability_export"));
                assert!(xml.contains("<nome>Interoperabilidade</nome>"));
            }
            ExportFormat::Sqlite => {
                let conn = Connection::open(output).unwrap();
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM interop_rows", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(count, 1);
            }
            ExportFormat::Xlsx => {
                let bytes = fs::read(output).unwrap();
                assert_eq!(&bytes[0..2], b"PK");
            }
        }
    }
}

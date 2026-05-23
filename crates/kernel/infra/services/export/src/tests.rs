use std::fs;

use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

use crate::{
    Config, CsvExportAdapter, ExportRequest, ExportRow, Exporter, Provider, RuntimeBinding,
    RuntimeExporter, XmlExportAdapter,
};

fn sample_request(path: impl Into<std::path::PathBuf>) -> ExportRequest {
    let mut row = ExportRow::new();
    row.insert("id".into(), json!("A-1"));
    row.insert("name".into(), json!("Ana, Silva"));
    row.insert("total".into(), json!(42));

    ExportRequest {
        snapshot_id: "exp:demo:1".into(),
        snapshot: None,
        rows: vec![row],
        columns: vec!["id".into(), "name".into(), "total".into()],
        root_name: Some("export".into()),
        sheet_name: Some("Dados".into()),
        table_name: Some("export_rows".into()),
        output_path: path.into(),
    }
}

#[test]
fn csv_render_cita_campos_com_virgula() {
    let req = sample_request("out.csv");
    let csv = CsvExportAdapter::render(&req);
    assert!(csv.starts_with("id,name,total"));
    assert!(csv.contains("\"Ana, Silva\""));
}

#[test]
fn xml_render_escapa_texto_e_inclui_snapshot_id() {
    let mut req = sample_request("out.xml");
    req.rows[0].insert("name".into(), json!("A&B <C>"));
    let xml = XmlExportAdapter::render(&req);
    assert!(xml.contains("snapshot_id=\"exp:demo:1\""));
    assert!(xml.contains("A&amp;B &lt;C&gt;"));
}

#[test]
fn runtime_exporta_csv_xml_sqlite_e_xlsx() {
    let dir = tempdir().unwrap();

    for (provider, file) in [
        (Provider::Csv, "out.csv"),
        (Provider::Xml, "out.xml"),
        (Provider::Sqlite, "out.sqlite"),
        (Provider::Xlsx, "out.xlsx"),
    ] {
        let req = sample_request(dir.path().join(file));
        let result = RuntimeExporter::new(provider).export(&req).unwrap();
        assert_eq!(result.provider, provider);
        assert!(req.output_path.exists());
        assert_eq!(result.artefacts.len(), 1);
        assert!(result.artefacts[0].hash.starts_with("sha256:"));
    }

    let sqlite_path = dir.path().join("out.sqlite");
    let conn = Connection::open(sqlite_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM export_rows", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let xlsx = fs::read(dir.path().join("out.xlsx")).unwrap();
    assert_eq!(&xlsx[0..2], b"PK");
}

#[test]
fn binding_valida_config_do_provider_selecionado() {
    let binding = RuntimeBinding {
        default_provider: Provider::Csv,
        csv: Some(Config::new(Provider::Csv)),
        xml: None,
        sqlite: None,
        xlsx: None,
    };
    assert!(binding.open_exporter().is_ok());
}

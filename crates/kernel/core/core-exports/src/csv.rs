use crate::snapshot::ExportSnapshot;

/// Gera CSV RFC 4180 para uma lista de snapshots.
///
/// Cada linha representa um snapshot. Campos com vírgulas ou aspas são citados.
/// Adequado para reporting e auditorias simples de volume reduzido.
pub fn snapshots_to_csv(snapshots: &[ExportSnapshot]) -> String {
    let header = "snapshot_id,exported_at,source_kind,source_subject_id,source_version,\
                  manifest_algorithm,manifest_hash,manifest_item_count,document_id";
    let mut lines = Vec::with_capacity(snapshots.len() + 1);
    lines.push(header.to_string());

    for s in snapshots {
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{}",
            csv_field(&s.snapshot_id),
            csv_field(&s.exported_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            csv_field(&s.source.kind),
            csv_field(&s.source.subject_id),
            csv_field(&s.source.version),
            csv_field(&s.manifest.algorithm),
            csv_field(&s.manifest.hash),
            s.manifest.item_count,
            csv_field(&s.document_package.document_id),
        ));
    }
    lines.join("\r\n")
}

/// Cita um campo CSV se contiver vírgula, aspas ou newline (RFC 4180).
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use core_documental::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};
    use serde_json::json;

    fn sample_snapshot() -> ExportSnapshot {
        let ts = chrono::Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap();
        ExportSnapshot {
            snapshot_id: "exp:config_profile:dev:1.0.0:abc123def456".into(),
            exported_at: ts,
            source: crate::snapshot::SourceRef {
                kind: "config_profile".into(),
                subject_id: "dev".into(),
                version: "1.0.0".into(),
            },
            document_package: DocumentPackage {
                document_id: "doc:config_profile:dev:1.0.0".into(),
                created_at: ts,
                template: TemplateRef {
                    template_id: "t".into(),
                    template_version: "v1".into(),
                    valid_at: None,
                },
                engine: EngineRef {
                    engine_id: "e".into(),
                    engine_version: "v1".into(),
                },
                artefacts: vec![Artefact {
                    kind: "json".into(),
                    artefact_ref: "ref".into(),
                    hash_result: HashResult {
                        algorithm: "SHA-256".into(),
                        hash: "sha256:abc".into(),
                        timestamp: ts,
                        input_kind: None,
                        input_ref: None,
                        meta: None,
                    },
                    mime: None,
                    size_bytes: None,
                }],
                subject: None,
                meta: Some(json!({})),
            },
            manifest: crate::snapshot::Manifest {
                algorithm: "SHA-256".into(),
                hash: "sha256:deadbeef".into(),
                item_count: 1,
            },
            meta: None,
        }
    }

    #[test]
    fn csv_tem_header_e_uma_linha_de_dados() {
        let csv = snapshots_to_csv(&[sample_snapshot()]);
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("snapshot_id,"));
        assert!(lines[1].contains("config_profile"));
    }

    #[test]
    fn csv_vazio_tem_apenas_header() {
        let csv = snapshots_to_csv(&[]);
        assert!(!csv.contains("\r\n"));
        assert!(csv.starts_with("snapshot_id,"));
    }

    #[test]
    fn csv_cita_campos_com_virgula() {
        let mut s = sample_snapshot();
        s.source.kind = "tipo,com,virgulas".into();
        let csv = snapshots_to_csv(&[s]);
        assert!(csv.contains("\"tipo,com,virgulas\""));
    }
}

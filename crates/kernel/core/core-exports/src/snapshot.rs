use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use core_audit::{AuditActor, AuditEvent, AuditOutcome, AuditTarget};
use core_documental::{validate_document_package, DocumentPackage};
use core_validation::sha256_bytes;

use crate::error::ExportError;

const HASH_PREFIX: &str = "sha256:";
const HASH_PREFIX_LEN: usize = HASH_PREFIX.len();
// 16 hex chars = 64 bits. Dentro de cada namespace (kind+subject_id+version) o
// espaço de colisão é sha256[HASH_PREFIX_LEN..HASH_PREFIX_LEN+16]; a probabilidade
// de colisão acidental entre conteúdos distintos no mesmo namespace é negligenciável
// para os volumes expectáveis em mini-apps AP. O hash completo fica em manifest.hash.
const SNAPSHOT_ID_HASH_LEN: usize = 16;

// ── Tipos públicos ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: String,
    pub subject_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub algorithm: String,
    pub hash: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub snapshot_id: String,
    pub exported_at: DateTime<Utc>,
    pub source: SourceRef,
    pub document_package: DocumentPackage,
    pub manifest: Manifest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

/// Recibo atómico de exportação: snapshot + evento de audit produzidos em conjunto.
///
/// O par é indivisível por design — qualquer export produz obrigatoriamente
/// evidência de audit com `actor` e `correlation_id` identificados.
/// Zero-trust: não existe snapshot sem rasto de quem o originou.
#[derive(Debug, Clone)]
pub struct ExportReceipt {
    pub snapshot: ExportSnapshot,
    pub audit_event: AuditEvent,
}

pub struct BuildSnapshotConfig {
    /// Quando `None`, usa `document_package.created_at`.
    pub exported_at: Option<DateTime<Utc>>,
    /// Identificador do principal que autoriza o export (obrigatório).
    pub actor: String,
    /// Identificador de correlação da operação (obrigatório).
    pub correlation_id: String,
    /// Meio de transporte do export. Usa `"inline"` quando `None` ou vazio.
    pub transport: Option<String>,
}

// ── Tipo interno ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ManifestPayload<'a> {
    source: &'a SourceRef,
    document_package: &'a DocumentPackage,
}

// ── API pública ────────────────────────────────────────────────────────────────

/// Constrói um `ExportReceipt` (snapshot + audit event) a partir de um `DocumentPackage`.
///
/// `actor` e `correlation_id` são obrigatórios — qualquer export de documentos
/// institucionais exige identificação do principal e rastreabilidade da operação
/// (princípio zero-trust para AP).
pub fn build_export_receipt(
    pkg: DocumentPackage,
    source: SourceRef,
    cfg: BuildSnapshotConfig,
) -> Result<ExportReceipt, ExportError> {
    validate_document_package(&pkg).map_err(|e| ExportError::InvalidPackage(e.to_string()))?;

    if source.kind.trim().is_empty()
        || source.subject_id.trim().is_empty()
        || source.version.trim().is_empty()
    {
        return Err(ExportError::MissingField {
            field: "source".into(),
        });
    }
    if cfg.actor.trim().is_empty() {
        return Err(ExportError::MissingField {
            field: "actor".into(),
        });
    }
    if cfg.correlation_id.trim().is_empty() {
        return Err(ExportError::MissingField {
            field: "correlation_id".into(),
        });
    }

    let payload = canonical_manifest_payload(&source, &pkg)?;
    let hash = format!("{}{}", HASH_PREFIX, sha256_bytes(&payload));
    let item_count = pkg.artefacts.len();
    let exported_at = cfg.exported_at.unwrap_or(pkg.created_at);

    let hash_prefix = &hash[HASH_PREFIX_LEN..HASH_PREFIX_LEN + SNAPSHOT_ID_HASH_LEN];
    let snapshot_id = format!(
        "exp:{}:{}:{}:{}",
        source.kind, source.subject_id, source.version, hash_prefix
    );
    let transport = cfg
        .transport
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("inline");

    let snapshot = ExportSnapshot {
        snapshot_id,
        exported_at,
        source,
        document_package: pkg,
        manifest: Manifest {
            algorithm: "SHA-256".into(),
            hash,
            item_count,
        },
        meta: Some(json!({ "transport": transport })),
    };

    validate_export_snapshot(&snapshot)?;
    let audit_event = build_audit_event(&snapshot, &cfg.actor, &cfg.correlation_id)?;
    Ok(ExportReceipt {
        snapshot,
        audit_event,
    })
}

pub fn validate_export_snapshot(snapshot: &ExportSnapshot) -> Result<(), ExportError> {
    validate_document_package(&snapshot.document_package)
        .map_err(|e| ExportError::InvalidPackage(e.to_string()))?;

    if snapshot.snapshot_id.trim().is_empty() {
        return Err(ExportError::MissingField {
            field: "snapshot_id".into(),
        });
    }
    if snapshot.source.kind.trim().is_empty()
        || snapshot.source.subject_id.trim().is_empty()
        || snapshot.source.version.trim().is_empty()
    {
        return Err(ExportError::InvalidSnapshot {
            message: "source incompleto".into(),
        });
    }
    if snapshot.manifest.algorithm != "SHA-256" {
        return Err(ExportError::InvalidSnapshot {
            message: "manifest.algorithm deve ser SHA-256".into(),
        });
    }
    if snapshot.manifest.hash.is_empty() {
        return Err(ExportError::InvalidSnapshot {
            message: "manifest.hash vazio".into(),
        });
    }
    let expected_count = snapshot.document_package.artefacts.len();
    if snapshot.manifest.item_count != expected_count {
        return Err(ExportError::InvalidSnapshot {
            message: format!(
                "manifest.item_count ({}) não coincide com artefacts.len() ({expected_count})",
                snapshot.manifest.item_count
            ),
        });
    }

    let recomputed = recompute_manifest_hash(snapshot)?;
    if snapshot.manifest.hash != recomputed {
        return Err(ExportError::InvalidSnapshot {
            message: format!(
                "manifest hash inválido: esperado {recomputed}, obteve {}",
                snapshot.manifest.hash
            ),
        });
    }
    Ok(())
}

/// Serializa o snapshot em bytes canónicos (chaves ordenadas recursivamente).
///
/// O resultado é determinístico independentemente da ordem de inserção de campos
/// `Value`. Adequado para assinar ou transmitir o snapshot com garantia de
/// reprodutibilidade. Nota: a função hash do manifesto cobre apenas
/// `(source, document_package)`, não o snapshot completo.
pub fn canonical_bytes(snapshot: &ExportSnapshot) -> Result<Vec<u8>, ExportError> {
    let v =
        serde_json::to_value(snapshot).map_err(|e| ExportError::MarshalFailed(e.to_string()))?;
    serde_json::to_vec(&sort_value_keys(v)).map_err(|e| ExportError::MarshalFailed(e.to_string()))
}

// ── Internos ───────────────────────────────────────────────────────────────────

fn build_audit_event(
    snapshot: &ExportSnapshot,
    actor: &str,
    correlation_id: &str,
) -> Result<AuditEvent, ExportError> {
    let audit_actor = AuditActor::new(actor).map_err(|e| ExportError::AuditError(e.to_string()))?;
    let audit_target = AuditTarget::new("export", &snapshot.source.subject_id)
        .map_err(|e| ExportError::AuditError(e.to_string()))?;

    let details = json!({
        "correlation_id": correlation_id,
        "snapshot_id": snapshot.snapshot_id,
        "source_kind": snapshot.source.kind,
        "subject_id": snapshot.source.subject_id,
        "version": snapshot.source.version,
        "manifest_hash": snapshot.manifest.hash,
    });

    AuditEvent::new(
        "export.generated",
        audit_actor,
        audit_target,
        AuditOutcome::Success,
        None,
        Some(details),
    )
    .map_err(|e| ExportError::AuditError(e.to_string()))
}

fn recompute_manifest_hash(snapshot: &ExportSnapshot) -> Result<String, ExportError> {
    let payload = canonical_manifest_payload(&snapshot.source, &snapshot.document_package)?;
    Ok(format!("{}{}", HASH_PREFIX, sha256_bytes(&payload)))
}

fn canonical_manifest_payload(
    source: &SourceRef,
    pkg: &DocumentPackage,
) -> Result<Vec<u8>, ExportError> {
    // Serializa para Value, normaliza chaves (sort), depois para bytes.
    // Garante determinismo independentemente da ordem de inserção de campos Value.
    let raw = serde_json::to_value(ManifestPayload {
        source,
        document_package: pkg,
    })
    .map_err(|e| ExportError::MarshalFailed(e.to_string()))?;
    serde_json::to_vec(&sort_value_keys(raw)).map_err(|e| ExportError::MarshalFailed(e.to_string()))
}

fn sort_value_keys(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let sorted: std::collections::BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_value_keys(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_value_keys).collect()),
        other => other,
    }
}

// ── Testes ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::ExportError;
    use core_documental::{Artefact, EngineRef, HashResult, TemplateRef};

    fn sample_package() -> DocumentPackage {
        let ts = Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap();
        DocumentPackage {
            document_id: "doc:config_profile:dev:1.0.0".into(),
            created_at: ts,
            template: TemplateRef {
                template_id: "config-profile-export".into(),
                template_version: "v1".into(),
                valid_at: None,
            },
            engine: EngineRef {
                engine_id: "config-json".into(),
                engine_version: "v1".into(),
            },
            artefacts: vec![Artefact {
                kind: "config_profile_json".into(),
                artefact_ref: "config-profile:dev:1.0.0".into(),
                hash_result: HashResult {
                    algorithm: "SHA-256".into(),
                    hash: "sha256:abc123".into(),
                    timestamp: ts,
                    input_kind: Some("config_profile".into()),
                    input_ref: Some("dev@1.0.0".into()),
                    meta: None,
                },
                mime: Some("application/json".into()),
                size_bytes: Some(64),
            }],
            subject: Some(json!({ "kind": "config_profile", "profile_id": "dev" })),
            meta: Some(json!({ "source": "unit-test", "version": "1.0.0" })),
        }
    }

    fn sample_source() -> SourceRef {
        SourceRef {
            kind: "config_profile".into(),
            subject_id: "dev".into(),
            version: "1.0.0".into(),
        }
    }

    fn sample_cfg() -> BuildSnapshotConfig {
        BuildSnapshotConfig {
            exported_at: Some(Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap()),
            actor: "daemon:apid".into(),
            correlation_id: "corr-gate-f-export".into(),
            transport: None,
        }
    }

    #[test]
    fn receipt_imutavel_para_mesmo_input() {
        let a = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        let b = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        assert_eq!(a.snapshot.manifest.hash, b.snapshot.manifest.hash);
        assert_eq!(a.snapshot.snapshot_id, b.snapshot.snapshot_id);
    }

    #[test]
    fn item_count_reflecte_artefacts_reais() {
        let r = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        assert_eq!(r.snapshot.manifest.item_count, 1);
    }

    #[test]
    fn manifest_muda_quando_document_muda() {
        let mut pkg = sample_package();
        let a = build_export_receipt(pkg.clone(), sample_source(), sample_cfg()).unwrap();
        pkg.meta = Some(json!({ "source": "changed" }));
        let b = build_export_receipt(pkg, sample_source(), sample_cfg()).unwrap();
        assert_ne!(a.snapshot.manifest.hash, b.snapshot.manifest.hash);
    }

    #[test]
    fn manifest_muda_quando_source_muda() {
        let pkg = sample_package();
        let a = build_export_receipt(pkg.clone(), sample_source(), sample_cfg()).unwrap();
        let b = build_export_receipt(
            pkg,
            SourceRef {
                kind: "config_profile".into(),
                subject_id: "prod".into(),
                version: "1.0.0".into(),
            },
            sample_cfg(),
        )
        .unwrap();
        assert_ne!(a.snapshot.manifest.hash, b.snapshot.manifest.hash);
    }

    #[test]
    fn rejeita_actor_vazio() {
        let mut cfg = sample_cfg();
        cfg.actor = String::new();
        let err = build_export_receipt(sample_package(), sample_source(), cfg).unwrap_err();
        assert!(matches!(err, ExportError::MissingField { field } if field == "actor"));
    }

    #[test]
    fn rejeita_correlation_id_vazio() {
        let mut cfg = sample_cfg();
        cfg.correlation_id = String::new();
        let err = build_export_receipt(sample_package(), sample_source(), cfg).unwrap_err();
        assert!(matches!(err, ExportError::MissingField { field } if field == "correlation_id"));
    }

    #[test]
    fn rejeita_item_count_errado() {
        let receipt =
            build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        let mut snapshot = receipt.snapshot;
        snapshot.manifest.item_count = 99;
        let err = validate_export_snapshot(&snapshot).unwrap_err();
        assert!(matches!(err, ExportError::InvalidSnapshot { .. }));
    }

    #[test]
    fn rejeita_manifest_hash_adulterado() {
        let receipt =
            build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        let mut snapshot = receipt.snapshot;
        snapshot.manifest.hash =
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        let err = validate_export_snapshot(&snapshot).unwrap_err();
        assert!(matches!(err, ExportError::InvalidSnapshot { .. }));
    }

    #[test]
    fn audit_event_tem_action_e_details_correctos() {
        let r = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        assert_eq!(r.audit_event.event_type, "export.generated");
        let details = r.audit_event.details_json.unwrap();
        assert_eq!(details["correlation_id"], "corr-gate-f-export");
        assert_eq!(details["subject_id"], "dev");
    }

    #[test]
    fn hash_determinista_para_maps_com_ordem_diferente() {
        let ts = Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap();
        let base = sample_package();
        let mut pkg_a = base.clone();
        pkg_a.meta = Some(json!({ "source": "unit-test", "version": "1.0.0" }));
        let mut pkg_b = base;
        pkg_b.meta = Some(json!({ "version": "1.0.0", "source": "unit-test" }));
        let cfg = || BuildSnapshotConfig {
            exported_at: Some(ts),
            actor: "daemon:apid".into(),
            correlation_id: "corr-x".into(),
            transport: None,
        };
        let a = build_export_receipt(pkg_a, sample_source(), cfg()).unwrap();
        let b = build_export_receipt(pkg_b, sample_source(), cfg()).unwrap();
        assert_eq!(a.snapshot.manifest.hash, b.snapshot.manifest.hash);
    }

    #[test]
    fn canonical_bytes_determinista_com_fields_value_em_ordens_diferentes() {
        let mut pkg_a = sample_package();
        pkg_a.meta = Some(json!({ "a": 1, "b": 2 }));
        let mut pkg_b = sample_package();
        pkg_b.meta = Some(json!({ "b": 2, "a": 1 }));
        let cfg = || sample_cfg();
        let r_a = build_export_receipt(pkg_a, sample_source(), cfg()).unwrap();
        let r_b = build_export_receipt(pkg_b, sample_source(), cfg()).unwrap();
        let bytes_a = canonical_bytes(&r_a.snapshot).unwrap();
        let bytes_b = canonical_bytes(&r_b.snapshot).unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "canonical_bytes deve ser igual para snapshots equivalentes"
        );
    }

    #[test]
    fn transport_configavel_no_snapshot_meta() {
        let mut cfg = sample_cfg();
        cfg.transport = Some("file".into());
        let r = build_export_receipt(sample_package(), sample_source(), cfg).unwrap();
        let meta = r.snapshot.meta.unwrap();
        assert_eq!(meta["transport"], "file");
    }

    #[test]
    fn transport_default_e_inline() {
        let r = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        let meta = r.snapshot.meta.unwrap();
        assert_eq!(meta["transport"], "inline");
    }

    #[test]
    fn snapshot_id_tem_16_chars_de_hash() {
        let r = build_export_receipt(sample_package(), sample_source(), sample_cfg()).unwrap();
        let parts: Vec<&str> = r.snapshot.snapshot_id.split(':').collect();
        // "exp:kind:subject:version:HASH16"
        let hash_part = parts.last().unwrap();
        assert_eq!(hash_part.len(), 16);
    }

    #[test]
    fn core_exports_nao_depende_de_sqlite() {
        let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(!m.contains("rusqlite") && !m.contains("adapter-sqlite"));
    }

    #[test]
    fn core_exports_nao_depende_de_tauri() {
        assert!(
            !include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).contains("tauri")
        );
    }
}

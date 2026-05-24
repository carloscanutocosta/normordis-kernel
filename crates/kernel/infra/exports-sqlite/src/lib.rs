#![allow(clippy::too_many_arguments)]

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::{DateTime, Utc};
use core_exports::{
    validate_export_snapshot, ExportError, ExportReceipt, ExportSnapshot, ExportSnapshotPort,
    Manifest, SourceRef,
};
use rusqlite::{params, Connection, OptionalExtension};
use support_errors::MiniError;
use thiserror::Error;

// ── Migrations ────────────────────────────────────────────────────────────────

pub const EXPORTS_SQLITE_MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE IF NOT EXISTS export_snapshots (
        snapshot_id          TEXT PRIMARY KEY,
        exported_at          TEXT NOT NULL,
        source_kind          TEXT NOT NULL,
        source_subject_id    TEXT NOT NULL,
        source_version       TEXT NOT NULL,
        manifest_algorithm   TEXT NOT NULL,
        manifest_hash        TEXT NOT NULL,
        manifest_item_count  INTEGER NOT NULL,
        document_package_json TEXT NOT NULL,
        meta_json            TEXT,
        saved_at             TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS export_audit_events (
        event_id     TEXT PRIMARY KEY,
        snapshot_id  TEXT NOT NULL REFERENCES export_snapshots(snapshot_id),
        event_type   TEXT NOT NULL,
        event_json   TEXT NOT NULL,
        saved_at     TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_exports_subject
        ON export_snapshots (source_subject_id, exported_at);
    CREATE INDEX IF NOT EXISTS idx_export_audit_snapshot
        ON export_audit_events (snapshot_id);
    "#];

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ExportsSqliteError {
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro de infra: {0}")]
    Infra(String),
    #[error("erro de serialização: {0}")]
    Json(String),
    #[error("data/hora inválida: {0}")]
    InvalidDateTime(String),
    #[error("snapshot não encontrado: {0}")]
    SnapshotNotFound(String),
}

impl From<MiniError> for ExportsSqliteError {
    fn from(e: MiniError) -> Self {
        ExportsSqliteError::Infra(e.to_string())
    }
}

impl From<ExportsSqliteError> for ExportError {
    fn from(e: ExportsSqliteError) -> Self {
        ExportError::InvalidPackage(e.to_string())
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Adapter SQLite para `ExportSnapshotPort`.
///
/// `open_write_create` — cria ou abre em leitura-escrita para guardar recibos.
/// `open_readonly`     — abre em modo read-only (`PRAGMA query_only = ON`),
///                       adequado para consumidores do export (zero risco de escrita acidental).
pub struct ExportsSqliteStore {
    conn: Connection,
}

impl ExportsSqliteStore {
    pub fn open_write_create(config: &SqliteRelationalConfig) -> Result<Self, ExportsSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_readonly(path: impl Into<std::path::PathBuf>) -> Result<Self, ExportsSqliteError> {
        let config = SqliteRelationalConfig::read_only(path);
        let conn = open_relational_connection(&config)?;
        Ok(Self { conn })
    }

    fn migrate(&self) -> Result<(), ExportsSqliteError> {
        run_relational_migrations(&self.conn, EXPORTS_SQLITE_MIGRATIONS)?;
        Ok(())
    }
}

// ── Helpers de serialização ───────────────────────────────────────────────────

fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn str_to_dt(s: &str) -> Result<DateTime<Utc>, ExportsSqliteError> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3fZ"))
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ExportsSqliteError::InvalidDateTime(s.to_string()))
}

fn row_to_snapshot(
    snapshot_id: String,
    exported_at_s: String,
    source_kind: String,
    source_subject_id: String,
    source_version: String,
    manifest_algorithm: String,
    manifest_hash: String,
    manifest_item_count: i64,
    document_package_json: String,
    meta_json: Option<String>,
) -> Result<ExportSnapshot, ExportsSqliteError> {
    let document_package = serde_json::from_str(&document_package_json)
        .map_err(|e| ExportsSqliteError::Json(e.to_string()))?;
    let meta = meta_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e: serde_json::Error| ExportsSqliteError::Json(e.to_string()))?;

    Ok(ExportSnapshot {
        snapshot_id,
        exported_at: str_to_dt(&exported_at_s)?,
        source: SourceRef {
            kind: source_kind,
            subject_id: source_subject_id,
            version: source_version,
        },
        document_package,
        manifest: Manifest {
            algorithm: manifest_algorithm,
            hash: manifest_hash,
            item_count: manifest_item_count as usize,
        },
        meta,
    })
}

// ── ExportSnapshotPort ────────────────────────────────────────────────────────

impl ExportSnapshotPort for ExportsSqliteStore {
    /// Persiste snapshot + audit event de forma atómica numa transação.
    fn save_receipt(&self, receipt: &ExportReceipt) -> Result<(), ExportError> {
        save_receipt_impl(&self.conn, receipt)
            .map_err(|e| ExportError::InvalidPackage(e.to_string()))
    }

    fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<ExportSnapshot>, ExportError> {
        load_snapshot_impl(&self.conn, snapshot_id)
            .map_err(|e| ExportError::InvalidPackage(e.to_string()))
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ExportSnapshot>, ExportError> {
        list_for_subject_impl(&self.conn, subject_id, limit, offset)
            .map_err(|e| ExportError::InvalidPackage(e.to_string()))
    }
}

fn save_receipt_impl(conn: &Connection, receipt: &ExportReceipt) -> Result<(), ExportsSqliteError> {
    let s = &receipt.snapshot;
    let pkg_json = serde_json::to_string(&s.document_package)
        .map_err(|e| ExportsSqliteError::Json(e.to_string()))?;
    let meta_json = s
        .meta
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| ExportsSqliteError::Json(e.to_string()))?;
    let event_json = serde_json::to_string(&receipt.audit_event)
        .map_err(|e| ExportsSqliteError::Json(e.to_string()))?;
    let saved_at = dt_to_str(Utc::now());

    // Transação atómica: snapshot + audit event juntos ou nenhum.
    conn.execute_batch("BEGIN;")?;
    let result = (|| -> Result<(), ExportsSqliteError> {
        conn.execute(
            "INSERT OR IGNORE INTO export_snapshots
                 (snapshot_id, exported_at, source_kind, source_subject_id, source_version,
                  manifest_algorithm, manifest_hash, manifest_item_count,
                  document_package_json, meta_json, saved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                s.snapshot_id,
                dt_to_str(s.exported_at),
                s.source.kind,
                s.source.subject_id,
                s.source.version,
                s.manifest.algorithm,
                s.manifest.hash,
                s.manifest.item_count as i64,
                pkg_json,
                meta_json,
                saved_at,
            ],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO export_audit_events
                 (event_id, snapshot_id, event_type, event_json, saved_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                receipt.audit_event.event_id,
                s.snapshot_id,
                receipt.audit_event.event_type,
                event_json,
                saved_at,
            ],
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

fn load_snapshot_impl(
    conn: &Connection,
    snapshot_id: &str,
) -> Result<Option<ExportSnapshot>, ExportsSqliteError> {
    let row = conn
        .query_row(
            "SELECT snapshot_id, exported_at, source_kind, source_subject_id, source_version,
                manifest_algorithm, manifest_hash, manifest_item_count,
                document_package_json, meta_json
         FROM export_snapshots WHERE snapshot_id = ?1",
            params![snapshot_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()?;

    let Some((sid, eat, sk, ssid, sv, ma, mh, mic, dpj, mj)) = row else {
        return Ok(None);
    };
    let snapshot = row_to_snapshot(sid, eat, sk, ssid, sv, ma, mh, mic, dpj, mj)?;
    // Zero-trust: re-valida o snapshot lido da BD, incluindo re-computação do hash
    // do manifesto, para detetar adulteração silenciosa ao nível do ficheiro SQLite.
    validate_export_snapshot(&snapshot)
        .map_err(|e| ExportsSqliteError::Infra(format!("snapshot corrompido na BD: {e}")))?;
    Ok(Some(snapshot))
}

fn list_for_subject_impl(
    conn: &Connection,
    subject_id: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<ExportSnapshot>, ExportsSqliteError> {
    let mut stmt = conn.prepare(
        "SELECT snapshot_id, exported_at, source_kind, source_subject_id, source_version,
                manifest_algorithm, manifest_hash, manifest_item_count,
                document_package_json, meta_json
         FROM export_snapshots
         WHERE source_subject_id = ?1
         ORDER BY exported_at DESC
         LIMIT ?2 OFFSET ?3",
    )?;

    let rows = stmt.query_map(params![subject_id, limit as i64, offset as i64], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, i64>(7)?,
            r.get::<_, String>(8)?,
            r.get::<_, Option<String>>(9)?,
        ))
    })?;

    let mut result = Vec::new();
    for row in rows {
        let (sid, eat, sk, ssid, sv, ma, mh, mic, dpj, mj) = row?;
        let snapshot = row_to_snapshot(sid, eat, sk, ssid, sv, ma, mh, mic, dpj, mj)?;
        validate_export_snapshot(&snapshot)
            .map_err(|e| ExportsSqliteError::Infra(format!("snapshot corrompido na BD: {e}")))?;
        result.push(snapshot);
    }
    Ok(result)
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::TimeZone;
    use core_documental::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};
    use core_exports::{build_export_receipt, BuildSnapshotConfig, SourceRef};
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn test_store() -> (ExportsSqliteStore, NamedTempFile) {
        let tmp = NamedTempFile::new().unwrap();
        let store = ExportsSqliteStore::open_write_create(
            &SqliteRelationalConfig::read_write_create(tmp.path()),
        )
        .unwrap();
        (store, tmp)
    }

    fn sample_package() -> DocumentPackage {
        let ts = Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap();
        DocumentPackage {
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
                artefact_ref: "ref:dev:1.0.0".into(),
                hash_result: HashResult {
                    algorithm: "SHA-256".into(),
                    hash: "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                        .into(),
                    timestamp: ts,
                    input_kind: None,
                    input_ref: None,
                    meta: None,
                },
                mime: Some("application/json".into()),
                size_bytes: Some(64),
            }],
            subject: Some(json!({ "kind": "config_profile" })),
            meta: Some(json!({ "source": "test" })),
        }
    }

    fn sample_receipt() -> ExportReceipt {
        let ts = Utc.with_ymd_and_hms(2026, 3, 8, 12, 0, 0).unwrap();
        build_export_receipt(
            sample_package(),
            SourceRef {
                kind: "config_profile".into(),
                subject_id: "dev".into(),
                version: "1.0.0".into(),
            },
            BuildSnapshotConfig {
                exported_at: Some(ts),
                actor: "daemon:apid".into(),
                correlation_id: "corr-test-001".into(),
                transport: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn save_e_load_round_trip() {
        let (store, _tmp) = test_store();
        let receipt = sample_receipt();
        let id = receipt.snapshot.snapshot_id.clone();

        store.save_receipt(&receipt).unwrap();

        let loaded = store.load_snapshot(&id).unwrap().unwrap();
        assert_eq!(loaded.snapshot_id, id);
        assert_eq!(loaded.source.subject_id, "dev");
        assert_eq!(loaded.manifest.item_count, 1);
    }

    #[test]
    fn load_inexistente_devolve_none() {
        let (store, _tmp) = test_store();
        let result = store
            .load_snapshot("exp:nao:existe:0.0.0:000000000000")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_for_subject_filtra_correctamente() {
        let (store, _tmp) = test_store();
        let receipt = sample_receipt();
        store.save_receipt(&receipt).unwrap();

        let lista = store.list_for_subject("dev", 100, 0).unwrap();
        assert_eq!(lista.len(), 1);

        let vazia = store.list_for_subject("prod", 100, 0).unwrap();
        assert!(vazia.is_empty());
    }

    #[test]
    fn save_idempotente_com_or_ignore() {
        let (store, _tmp) = test_store();
        let receipt = sample_receipt();
        store.save_receipt(&receipt).unwrap();
        store.save_receipt(&receipt).unwrap(); // segunda vez: OR IGNORE, sem erro
        let lista = store.list_for_subject("dev", 100, 0).unwrap();
        assert_eq!(lista.len(), 1);
    }

    #[test]
    fn open_readonly_rejeita_escritas() {
        let tmp = NamedTempFile::new().unwrap();
        // Primeiro cria e migra
        ExportsSqliteStore::open_write_create(&SqliteRelationalConfig::read_write_create(
            tmp.path(),
        ))
        .unwrap();

        // Abre read-only e tenta escrever — deve falhar
        let ro = ExportsSqliteStore::open_readonly(tmp.path()).unwrap();
        let receipt = sample_receipt();
        let err = ro.save_receipt(&receipt);
        assert!(err.is_err(), "read-only deve rejeitar escritas");
    }

    #[test]
    fn exports_sqlite_nao_depende_de_core_config() {
        let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(!m.contains("core-config"));
    }
}

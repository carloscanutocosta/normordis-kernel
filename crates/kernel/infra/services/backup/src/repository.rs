use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::params;
use uuid::Uuid;

use crate::error::BackupServiceError;
use crate::types::{DbMaintenanceDetail, HealthStatus, MaintenanceRun, MaintenanceStatus};

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS maintenance_log (
        id TEXT PRIMARY KEY,
        run_date TEXT NOT NULL UNIQUE,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        triggered_by TEXT NOT NULL DEFAULT 'scheduler',
        status TEXT NOT NULL,
        backup_path TEXT,
        backup_size INTEGER,
        checksum TEXT,
        purged_at TEXT
    );

    CREATE TABLE IF NOT EXISTS maintenance_db_log (
        id TEXT PRIMARY KEY,
        maintenance_id TEXT NOT NULL REFERENCES maintenance_log(id),
        db_name TEXT NOT NULL,
        health_status TEXT NOT NULL,
        health_detail TEXT,
        file_size_bytes INTEGER,
        wal_size_bytes INTEGER,
        last_modified_at TEXT,
        backup_included INTEGER NOT NULL DEFAULT 0,
        backup_status TEXT,
        error_message TEXT
    );
";

#[derive(Clone)]
pub struct MaintenanceRepository {
    db_path: String,
}

impl MaintenanceRepository {
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    fn open(&self) -> Result<rusqlite::Connection, BackupServiceError> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
        Ok(conn)
    }

    /// Tenta adquirir o lock diário via INSERT OR IGNORE na run_date de hoje.
    /// Retorna `Some(run)` se o lock foi adquirido, `None` se já existia uma entrada hoje.
    pub async fn try_acquire_lock(
        &self,
        triggered_by: &str,
    ) -> Result<Option<MaintenanceRun>, BackupServiceError> {
        let repo = self.clone();
        let triggered_by = triggered_by.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            let id = Uuid::new_v4();
            let run_date = Utc::now().date_naive();
            let started_at = Utc::now();

            conn.execute(
                "INSERT OR IGNORE INTO maintenance_log \
                 (id, run_date, started_at, triggered_by, status) \
                 VALUES (?1, ?2, ?3, ?4, 'running')",
                params![
                    id.to_string(),
                    run_date.to_string(),
                    started_at.to_rfc3339(),
                    triggered_by,
                ],
            )
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;

            if conn.changes() == 0 {
                return Ok(None);
            }

            Ok(Some(MaintenanceRun {
                id,
                run_date,
                started_at,
                finished_at: None,
                triggered_by: triggered_by.clone(),
                status: MaintenanceStatus::Running,
                backup_path: None,
                backup_size: None,
                checksum: None,
                purged_at: None,
            }))
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    pub async fn save_db_detail(
        &self,
        maintenance_id: Uuid,
        db_name: &str,
        health: &HealthStatus,
        file_size_bytes: Option<i64>,
        wal_size_bytes: Option<i64>,
        last_modified_at: Option<DateTime<Utc>>,
        backup_included: bool,
        backup_status: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), BackupServiceError> {
        let repo = self.clone();
        let db_name = db_name.to_string();
        let health_status = health.as_str().to_string();
        let health_detail = health.detail().map(str::to_string);
        let backup_status = backup_status.map(str::to_string);
        let error_message = error_message.map(str::to_string);

        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            let id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO maintenance_db_log \
                 (id, maintenance_id, db_name, health_status, health_detail, \
                  file_size_bytes, wal_size_bytes, last_modified_at, \
                  backup_included, backup_status, error_message) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    id.to_string(),
                    maintenance_id.to_string(),
                    db_name,
                    health_status,
                    health_detail,
                    file_size_bytes,
                    wal_size_bytes,
                    last_modified_at.map(|d| d.to_rfc3339()),
                    backup_included as i64,
                    backup_status,
                    error_message,
                ],
            )
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    pub async fn finalize_run(
        &self,
        id: Uuid,
        status: MaintenanceStatus,
        backup_path: Option<&str>,
        backup_size: Option<i64>,
        checksum: Option<&str>,
    ) -> Result<(), BackupServiceError> {
        let repo = self.clone();
        let status_str = status.as_str().to_string();
        let backup_path = backup_path.map(str::to_string);
        let checksum = checksum.map(str::to_string);

        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            conn.execute(
                "UPDATE maintenance_log \
                 SET finished_at=?2, status=?3, backup_path=?4, backup_size=?5, checksum=?6 \
                 WHERE id=?1",
                params![
                    id.to_string(),
                    Utc::now().to_rfc3339(),
                    status_str,
                    backup_path,
                    backup_size,
                    checksum,
                ],
            )
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    pub async fn mark_purged(&self, id: Uuid) -> Result<(), BackupServiceError> {
        let repo = self.clone();
        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            conn.execute(
                "UPDATE maintenance_log SET purged_at=?2 WHERE id=?1",
                params![id.to_string(), Utc::now().to_rfc3339()],
            )
            .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    /// Lista backups bem-sucedidos não purgados, do mais recente para o mais antigo.
    pub async fn list_successful_backups(&self) -> Result<Vec<MaintenanceRun>, BackupServiceError> {
        let repo = self.clone();
        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, run_date, started_at, finished_at, triggered_by, \
                             status, backup_path, backup_size, checksum, purged_at \
                     FROM maintenance_log \
                     WHERE status='success' AND purged_at IS NULL \
                     ORDER BY run_date DESC",
                )
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            let rows = stmt
                .query_map([], row_to_run)
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    pub async fn list_all_runs(&self, limit: usize) -> Result<Vec<MaintenanceRun>, BackupServiceError> {
        let repo = self.clone();
        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, run_date, started_at, finished_at, triggered_by, \
                             status, backup_path, backup_size, checksum, purged_at \
                     FROM maintenance_log \
                     ORDER BY run_date DESC \
                     LIMIT ?1",
                )
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            let rows = stmt
                .query_map(params![limit as i64], row_to_run)
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }

    pub async fn get_run_details(
        &self,
        maintenance_id: Uuid,
    ) -> Result<Vec<DbMaintenanceDetail>, BackupServiceError> {
        let repo = self.clone();
        tokio::task::spawn_blocking(move || {
            let conn = repo.open()?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, maintenance_id, db_name, health_status, health_detail, \
                             file_size_bytes, wal_size_bytes, last_modified_at, \
                             backup_included, backup_status, error_message \
                     FROM maintenance_db_log \
                     WHERE maintenance_id=?1",
                )
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            let rows = stmt
                .query_map(params![maintenance_id.to_string()], row_to_detail)
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))
        })
        .await
        .map_err(|e| BackupServiceError::ControlDbFailed(e.to_string()))?
    }
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<MaintenanceRun> {
    let id: String = row.get(0)?;
    let run_date: String = row.get(1)?;
    let started_at: String = row.get(2)?;
    let finished_at: Option<String> = row.get(3)?;
    let status: String = row.get(5)?;
    let purged_at: Option<String> = row.get(9)?;

    Ok(MaintenanceRun {
        id: Uuid::parse_str(&id).unwrap_or_default(),
        run_date: run_date.parse::<NaiveDate>().unwrap_or_default(),
        started_at: parse_dt(&started_at),
        finished_at: finished_at.as_deref().map(parse_dt),
        triggered_by: row.get(4)?,
        status: parse_status(&status),
        backup_path: row.get(6)?,
        backup_size: row.get(7)?,
        checksum: row.get(8)?,
        purged_at: purged_at.as_deref().map(parse_dt),
    })
}

fn row_to_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbMaintenanceDetail> {
    let id: String = row.get(0)?;
    let maintenance_id: String = row.get(1)?;
    let health_status_str: String = row.get(3)?;
    let health_detail: Option<String> = row.get(4)?;
    let backup_included: i64 = row.get(8)?;
    let last_modified_at: Option<String> = row.get(7)?;

    let health_status = match health_status_str.as_str() {
        "warning" => HealthStatus::Warning(health_detail.unwrap_or_default()),
        "failed" => HealthStatus::Failed(health_detail.unwrap_or_default()),
        _ => HealthStatus::Ok,
    };

    Ok(DbMaintenanceDetail {
        id: Uuid::parse_str(&id).unwrap_or_default(),
        maintenance_id: Uuid::parse_str(&maintenance_id).unwrap_or_default(),
        db_name: row.get(2)?,
        health_status,
        file_size_bytes: row.get(5)?,
        wal_size_bytes: row.get(6)?,
        last_modified_at: last_modified_at.as_deref().map(parse_dt),
        backup_included: backup_included != 0,
        backup_status: row.get(9)?,
        error_message: row.get(10)?,
    })
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_default()
}

fn parse_status(s: &str) -> MaintenanceStatus {
    match s {
        "success" => MaintenanceStatus::Success,
        "partial" => MaintenanceStatus::Partial,
        "failed" => MaintenanceStatus::Failed,
        "skipped" => MaintenanceStatus::Skipped,
        _ => MaintenanceStatus::Running,
    }
}

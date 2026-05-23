use std::path::Path;

use crate::error::BackupServiceError;
use crate::types::HealthStatus;

const WAL_WARN_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

pub async fn check_database(db_path: &str) -> Result<HealthStatus, BackupServiceError> {
    let db_path = db_path.to_string();
    tokio::task::spawn_blocking(move || check_sync(&db_path))
        .await
        .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?
}

fn check_sync(db_path: &str) -> Result<HealthStatus, BackupServiceError> {
    let path = Path::new(db_path);

    if !path.exists() {
        return Err(BackupServiceError::HealthFailed {
            path: db_path.to_string(),
            reason: "file does not exist".into(),
        });
    }

    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| BackupServiceError::HealthFailed {
        path: db_path.to_string(),
        reason: e.to_string(),
    })?;

    // PRAGMA integrity_check — deve retornar "ok"
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| BackupServiceError::HealthFailed {
            path: db_path.to_string(),
            reason: format!("integrity_check failed: {e}"),
        })?;

    if integrity != "ok" {
        return Ok(HealthStatus::Failed(format!(
            "integrity_check: {integrity}"
        )));
    }

    // Verificar chaves estrangeiras
    let fk_errors: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check()",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if fk_errors > 0 {
        return Ok(HealthStatus::Warning(format!(
            "{fk_errors} foreign key violation(s)"
        )));
    }

    // Verificar tamanho do WAL
    let wal_path = format!("{db_path}-wal");
    let wal_size = std::fs::metadata(&wal_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if wal_size > WAL_WARN_BYTES {
        return Ok(HealthStatus::Warning(format!(
            "WAL size is {} MB (> 100 MB)",
            wal_size / (1024 * 1024)
        )));
    }

    Ok(HealthStatus::Ok)
}

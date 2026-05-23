use std::path::Path;

use chrono::Utc;

use sha2::{Digest, Sha256};

use crate::backup::{create_archive, extract_archive, vacuum_into};
use crate::config::MaintenanceConfig;
use crate::error::BackupServiceError;
use crate::health_check::check_database;
use crate::repository::MaintenanceRepository;
use crate::rotation::rotate_backups;
use crate::types::{HealthStatus, MaintenanceStatus};

pub struct MaintenanceService {
    config: MaintenanceConfig,
    repo: MaintenanceRepository,
}

impl MaintenanceService {
    pub fn new(config: MaintenanceConfig) -> Result<Self, BackupServiceError> {
        config.validate()?;
        let repo = MaintenanceRepository::new(&config.control_db_path);
        Ok(Self { config, repo })
    }

    /// Executa o ciclo completo de manutenção: lock → health → backup → rotação → finalização.
    /// Retorna `Ok(())` silenciosamente se o lock já estiver adquirido (já correu hoje).
    pub async fn run(&self, triggered_by: &str) -> Result<(), BackupServiceError> {
        let Some(run) = self.repo.try_acquire_lock(triggered_by).await? else {
            return Ok(()); // já correu hoje
        };

        let staging = tempfile::tempdir()
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

        let mut any_backup = false;
        let mut any_failure = false;

        for db_path in &self.config.db_paths {
            // Ignorar o control.db
            if db_path == &self.config.control_db_path {
                continue;
            }

            let db_name = Path::new(db_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(db_path);

            let health = check_database(db_path).await.unwrap_or_else(|e| {
                HealthStatus::Failed(e.to_string())
            });

            let file_meta = std::fs::metadata(db_path).ok();
            let file_size = file_meta.as_ref().map(|m| m.len() as i64);
            let last_modified = file_meta
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::<Utc>::from);

            let wal_size = std::fs::metadata(format!("{db_path}-wal"))
                .map(|m| m.len() as i64)
                .ok();

            let (backup_included, backup_status, error_msg) = if health.is_failed() {
                any_failure = true;
                (false, Some("skipped".to_string()), None)
            } else {
                match vacuum_into(db_path, staging.path()).await {
                    Ok(_) => {
                        any_backup = true;
                        (true, Some("ok".to_string()), None)
                    }
                    Err(e) => {
                        any_failure = true;
                        (false, Some("failed".to_string()), Some(e.to_string()))
                    }
                }
            };

            self.repo
                .save_db_detail(
                    run.id,
                    db_name,
                    &health,
                    file_size,
                    wal_size,
                    last_modified,
                    backup_included,
                    backup_status.as_deref(),
                    error_msg.as_deref(),
                )
                .await?;
        }

        if !any_backup {
            self.repo
                .finalize_run(run.id, MaintenanceStatus::Failed, None, None, None)
                .await?;
            return Err(BackupServiceError::ArchiveFailed(
                "no databases were successfully backed up".into(),
            ));
        }

        let date_str = run.run_date.format("%Y-%m-%d").to_string();
        let dest_dir = Path::new(&self.config.destination_path);

        let (archive_path, checksum, size) =
            match create_archive(staging.path(), dest_dir, &date_str, &self.config.backup_passphrase).await {
                Ok(result) => result,
                Err(e) => {
                    self.repo
                        .finalize_run(run.id, MaintenanceStatus::Failed, None, None, None)
                        .await
                        .ok();
                    return Err(e);
                }
            };

        rotate_backups(&self.repo, self.config.keep_last).await?;

        let final_status = if any_failure {
            MaintenanceStatus::Partial
        } else {
            MaintenanceStatus::Success
        };

        self.repo
            .finalize_run(
                run.id,
                final_status,
                Some(&archive_path.to_string_lossy()),
                Some(size),
                Some(&checksum),
            )
            .await?;

        Ok(())
    }

    pub fn repository(&self) -> &MaintenanceRepository {
        &self.repo
    }

    /// Restaura um backup para `dest_dir`, verificando o checksum antes de extrair.
    /// Retorna os caminhos dos ficheiros SQLite restaurados.
    /// O chamador é responsável por substituir as bases de dados ativas.
    pub async fn restore(
        &self,
        run: &crate::types::MaintenanceRun,
        dest_dir: &Path,
    ) -> Result<Vec<std::path::PathBuf>, BackupServiceError> {
        let backup_path = run.backup_path.as_deref().ok_or_else(|| {
            BackupServiceError::IoFailed("run has no backup_path".into())
        })?;

        let backup_path_owned = backup_path.to_string();
        let data = tokio::task::spawn_blocking(move || std::fs::read(&backup_path_owned))
            .await
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

        if let Some(expected) = &run.checksum {
            let actual = hex::encode(Sha256::digest(&data));
            if actual != *expected {
                return Err(BackupServiceError::ArchiveFailed(
                    "checksum mismatch — archive may be corrupted or tampered".into(),
                ));
            }
        }

        let passphrase = self.config.backup_passphrase.clone();
        let dest_dir = dest_dir.to_path_buf();

        tokio::task::spawn_blocking(move || extract_archive(&data, &passphrase, &dest_dir))
            .await
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?
    }

    pub fn config(&self) -> &MaintenanceConfig {
        &self.config
    }
}

use crate::error::BackupServiceError;
use crate::repository::MaintenanceRepository;

/// Purga backups antigos mantendo os `keep_last` mais recentes.
/// Backups purgados ficam marcados no control.db (trilha de auditoria).
/// Retorna o número de entradas purgadas.
pub async fn rotate_backups(
    repo: &MaintenanceRepository,
    keep_last: usize,
) -> Result<usize, BackupServiceError> {
    let backups = repo.list_successful_backups().await?;

    let to_purge = backups.into_iter().skip(keep_last).collect::<Vec<_>>();
    let count = to_purge.len();

    for run in to_purge {
        if let Some(path) = &run.backup_path {
            if let Err(e) = std::fs::remove_file(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(BackupServiceError::RetentionFailed(e.to_string()));
                }
            }
        }
        repo.mark_purged(run.id).await?;
    }

    Ok(count)
}

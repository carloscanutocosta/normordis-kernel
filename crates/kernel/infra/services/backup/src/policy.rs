use std::path::PathBuf;

use crate::error::BackupServiceError;

#[derive(Debug, Clone)]
pub struct BackupPolicy {
    /// Diretório de destino dos backups (pode ser partilha de rede mapeada).
    pub backup_dir: PathBuf,
    /// Passphrase de cifragem dos arquivos `.mbak`.
    pub passphrase: String,
    /// Número máximo de backups a reter por base de dados.
    pub max_backups: usize,
    /// Intervalo entre manutenções (em horas). Usado pelo agendador externo.
    pub interval_hours: u32,
}

impl BackupPolicy {
    pub fn new(
        backup_dir: impl Into<PathBuf>,
        passphrase: impl Into<String>,
        max_backups: usize,
        interval_hours: u32,
    ) -> Result<Self, BackupServiceError> {
        let passphrase = passphrase.into();
        let backup_dir = backup_dir.into();

        if passphrase.trim().is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "passphrase cannot be empty".into(),
            ));
        }
        if max_backups == 0 {
            return Err(BackupServiceError::PolicyInvalid(
                "max_backups must be at least 1".into(),
            ));
        }
        if interval_hours == 0 {
            return Err(BackupServiceError::PolicyInvalid(
                "interval_hours must be at least 1".into(),
            ));
        }
        if backup_dir.as_os_str().is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "backup_dir cannot be empty".into(),
            ));
        }

        Ok(Self {
            backup_dir,
            passphrase,
            max_backups,
            interval_hours,
        })
    }
}

use serde::{Deserialize, Serialize};

use crate::error::BackupServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    /// Hora do agendamento diário no formato "HH:MM" (ex: "16:00").
    pub schedule_time: String,
    /// Caminho de destino dos backups (pode ser partilha de rede mapeada).
    pub destination_path: String,
    /// Número de backups a reter (rotação). Padrão: 7.
    pub keep_last: usize,
    /// Caminhos dos ficheiros SQLite a incluir no backup.
    pub db_paths: Vec<String>,
    /// Caminho do control.db (nunca incluído no backup).
    pub control_db_path: String,
    /// Passphrase de cifragem do arquivo final.
    pub backup_passphrase: String,
}

impl MaintenanceConfig {
    pub fn validate(&self) -> Result<(), BackupServiceError> {
        if self.destination_path.trim().is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "destination_path cannot be empty".into(),
            ));
        }
        if self.control_db_path.trim().is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "control_db_path cannot be empty".into(),
            ));
        }
        if self.db_paths.is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "db_paths must have at least one entry".into(),
            ));
        }
        if self.keep_last == 0 {
            return Err(BackupServiceError::PolicyInvalid(
                "keep_last must be at least 1".into(),
            ));
        }
        if self.backup_passphrase.trim().is_empty() {
            return Err(BackupServiceError::PolicyInvalid(
                "backup_passphrase cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn schedule_hour_minute(&self) -> (u32, u32) {
        let mut parts = self.schedule_time.splitn(2, ':');
        let hour = parts.next().and_then(|h| h.parse().ok()).unwrap_or(16);
        let minute = parts.next().and_then(|m| m.parse().ok()).unwrap_or(0);
        (hour, minute)
    }
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            schedule_time: "16:00".into(),
            destination_path: String::new(),
            keep_last: 7,
            db_paths: Vec::new(),
            control_db_path: String::new(),
            backup_passphrase: String::new(),
        }
    }
}

use std::fs;
use std::time::{Duration, SystemTime};

use crate::config::LoggingConfig;
use crate::error::LogError;
use crate::rotate::is_managed_log_file;

pub fn apply_retention(config: &LoggingConfig) -> Result<(), LogError> {
    if !config.log_dir.exists() {
        return Ok(());
    }

    let cutoff = if config.retention_days == 0 {
        SystemTime::now()
    } else {
        SystemTime::now()
            .checked_sub(Duration::from_secs(
                config.retention_days.saturating_mul(24 * 60 * 60),
            ))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    };

    let entries = fs::read_dir(&config.log_dir).map_err(|_| LogError::RetentionFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| LogError::RetentionFailed)?;
        let path = entry.path();
        if !path.is_file() || !is_managed_log_file(config, &path) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|_| LogError::RetentionFailed)?;
        if modified < cutoff {
            fs::remove_file(path).map_err(|_| LogError::RetentionFailed)?;
        }
    }

    Ok(())
}

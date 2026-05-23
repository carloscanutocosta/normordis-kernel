use std::fs;
use std::path::PathBuf;

use crate::config::LoggingConfig;
use crate::error::LogError;

pub fn rotate_if_needed(config: &LoggingConfig) -> Result<(), LogError> {
    let active = config.active_path();
    if !active.exists() {
        return Ok(());
    }

    let len = fs::metadata(&active)
        .map_err(|_| LogError::RotationFailed)?
        .len();
    if len <= config.max_file_size_bytes() {
        return Ok(());
    }

    rotate(config)
}

fn rotate(config: &LoggingConfig) -> Result<(), LogError> {
    if config.max_files <= 1 {
        let active = config.active_path();
        if active.exists() {
            fs::remove_file(active).map_err(|_| LogError::RotationFailed)?;
        }
        return Ok(());
    }

    let max_rotated = config.max_files.saturating_sub(1);
    let last = rotated_path(config, max_rotated);
    if last.exists() {
        fs::remove_file(&last).map_err(|_| LogError::RotationFailed)?;
    }

    for index in (1..max_rotated).rev() {
        let from = rotated_path(config, index);
        let to = rotated_path(config, index + 1);
        if from.exists() {
            fs::rename(from, to).map_err(|_| LogError::RotationFailed)?;
        }
    }

    let active = config.active_path();
    if active.exists() {
        fs::rename(active, rotated_path(config, 1)).map_err(|_| LogError::RotationFailed)?;
    }

    Ok(())
}

pub fn rotated_path(config: &LoggingConfig, index: usize) -> PathBuf {
    let file_name = &config.file_name;
    if let Some(base) = file_name.strip_suffix(".log") {
        config.log_dir.join(format!("{base}.{index}.log"))
    } else {
        config.log_dir.join(format!("{file_name}.{index}"))
    }
}

pub fn is_managed_log_file(config: &LoggingConfig, path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if name == config.file_name {
        return true;
    }
    if let Some(base) = config.file_name.strip_suffix(".log") {
        return name.starts_with(&format!("{base}.")) && name.ends_with(".log");
    }
    name.starts_with(&format!("{}.", config.file_name))
}

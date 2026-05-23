use core_config::{resolve_paths, PathsConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLayout {
    pub database_dir: PathBuf,
    pub data_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub logs_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum FilesError {
    #[error("erro de IO: {0}")]
    Io(#[from] std::io::Error),
}

pub fn resolve_layout(base_dir: impl AsRef<Path>, paths: &PathsConfig) -> FileLayout {
    let resolved = resolve_paths(base_dir, paths);
    FileLayout {
        database_dir: resolved.database_dir,
        data_dir: resolved.data_dir,
        artifacts_dir: resolved.artifacts_dir,
        temp_dir: resolved.temp_dir,
        logs_dir: resolved.logs_dir,
    }
}

pub fn ensure_directories(layout: &FileLayout) -> Result<(), FilesError> {
    fs::create_dir_all(&layout.database_dir)?;
    fs::create_dir_all(&layout.data_dir)?;
    fs::create_dir_all(&layout.artifacts_dir)?;
    fs::create_dir_all(&layout.temp_dir)?;
    fs::create_dir_all(&layout.logs_dir)?;
    Ok(())
}

pub fn prune_stale_temp_files(
    temp_dir: impl AsRef<Path>,
    max_age: Duration,
) -> Result<(), FilesError> {
    let temp_dir = temp_dir.as_ref();
    if !temp_dir.exists() {
        return Ok(());
    }

    let cutoff = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    prune_temp_dir_contents(temp_dir, cutoff)?;
    Ok(())
}

fn prune_temp_dir_contents(dir: &Path, cutoff: SystemTime) -> Result<bool, FilesError> {
    let mut has_kept_entries = false;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let child_has_kept_entries = prune_temp_dir_contents(&path, cutoff)?;
            if child_has_kept_entries {
                has_kept_entries = true;
            } else {
                fs::remove_dir(&path)?;
            }
            continue;
        }

        if file_type.is_file() {
            let should_keep = match entry.metadata().and_then(|metadata| metadata.modified()) {
                Ok(modified) => modified >= cutoff,
                Err(_) => true,
            };
            if should_keep {
                has_kept_entries = true;
            } else {
                fs::remove_file(&path)?;
            }
            continue;
        }

        has_kept_entries = true;
    }

    Ok(has_kept_entries)
}

pub fn generate_technical_filename(prefix: &str, id: &str, extension: &str) -> String {
    let clean_prefix = sanitize_segment(prefix);
    let clean_id = sanitize_segment(id);
    let clean_ext = extension.trim().trim_start_matches('.');
    format!("{clean_prefix}_{clean_id}.{clean_ext}")
}

fn sanitize_segment(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

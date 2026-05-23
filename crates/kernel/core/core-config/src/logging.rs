use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_LOG_FILE_NAME: &str = "normordis-miniapps.jsonl";
pub const DEFAULT_MAX_FILE_SIZE_MB: u64 = 10;
pub const DEFAULT_MAX_FILES: usize = 10;
pub const DEFAULT_RETENTION_DAYS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingProfile {
    pub enabled: bool,
    pub log_dir: Option<PathBuf>,
    pub file_name: String,
    pub max_file_size_mb: u64,
    pub max_files: usize,
    pub retention_days: u64,
}

impl Default for LoggingProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            log_dir: None,
            file_name: DEFAULT_LOG_FILE_NAME.to_owned(),
            max_file_size_mb: DEFAULT_MAX_FILE_SIZE_MB,
            max_files: DEFAULT_MAX_FILES,
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

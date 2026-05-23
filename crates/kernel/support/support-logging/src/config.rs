use std::path::PathBuf;

use crate::error::LogError;
use crate::level::LogLevel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfig {
    pub log_dir: PathBuf,
    pub file_name: String,
    pub max_file_size_mb: u64,
    pub max_files: usize,
    pub retention_days: u64,
    pub min_level: LogLevel,
    pub max_message_chars: usize,
    pub max_details_bytes: usize,
    pub flush_each_event: bool,
}

impl LoggingConfig {
    pub fn new(log_dir: impl Into<PathBuf>, file_name: impl Into<String>) -> Self {
        Self {
            log_dir: log_dir.into(),
            file_name: file_name.into(),
            max_file_size_mb: 10,
            max_files: 10,
            retention_days: 30,
            min_level: LogLevel::Info,
            max_message_chars: 4_096,
            max_details_bytes: 16 * 1024,
            flush_each_event: true,
        }
    }

    pub fn validate(&self) -> Result<(), LogError> {
        if self.log_dir.as_os_str().is_empty()
            || self.file_name.trim().is_empty()
            || self.file_name.contains('/')
            || self.file_name.contains('\\')
            || self.max_file_size_mb == 0
            || self.max_files == 0
            || self.max_message_chars == 0
            || self.max_details_bytes == 0
        {
            return Err(LogError::ConfigInvalid);
        }

        Ok(())
    }

    pub fn active_path(&self) -> PathBuf {
        self.log_dir.join(&self.file_name)
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb.saturating_mul(1024 * 1024)
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self::new("logs", "app.log")
    }
}

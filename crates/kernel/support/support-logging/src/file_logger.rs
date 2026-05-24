use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use support_errors::MiniError;

use crate::config::LoggingConfig;
use crate::error::LogError;
use crate::event::LogEvent;
use crate::logger::{LogResult, TechnicalLogger};
use crate::retention::apply_retention;
use crate::rotate::rotate_if_needed;
use crate::sanitize::sanitize_event;

#[derive(Debug)]
pub struct FileLogger {
    config: LoggingConfig,
    lock: Mutex<()>,
}

impl FileLogger {
    pub fn new(config: LoggingConfig) -> Result<Self, Box<MiniError>> {
        config
            .validate()
            .map_err(MiniError::from)
            .map_err(Box::new)?;
        fs::create_dir_all(&config.log_dir)
            .map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        apply_retention(&config)
            .map_err(MiniError::from)
            .map_err(Box::new)?;

        Ok(Self {
            config,
            lock: Mutex::new(()),
        })
    }

    pub fn config(&self) -> &LoggingConfig {
        &self.config
    }
}

impl TechnicalLogger for FileLogger {
    fn log(&self, event: LogEvent) -> LogResult {
        if event.level.severity() < self.config.min_level.severity() {
            return Ok(());
        }

        let _guard = self
            .lock
            .lock()
            .map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        apply_retention(&self.config)
            .map_err(MiniError::from)
            .map_err(Box::new)?;
        rotate_if_needed(&self.config)
            .map_err(MiniError::from)
            .map_err(Box::new)?;

        let event = sanitize_event(event, &self.config);
        let line = serde_json::to_string(&event)
            .map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.config.active_path())
            .map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        writeln!(file, "{line}").map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        if self.config.flush_each_event {
            file.flush()
                .map_err(|_| Box::new(LogError::WriteFailed.to_mini_error()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;

    use serde_json::Value;
    use support_errors::{Component, ErrorCode, MiniError};
    use tempfile::tempdir;

    use super::*;
    use crate::{log_mini_error, LogLevel};

    #[test]
    fn writes_valid_json_line() {
        let dir = tempdir().unwrap();
        let logger = FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap();

        logger
            .log(LogEvent::new(
                LogLevel::Error,
                "runtime-bootstrap",
                "failed",
            ))
            .unwrap();

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["level"], "ERROR");
        assert_eq!(value["component"], "runtime-bootstrap");
    }

    #[test]
    fn creates_directory_automatically() {
        let dir = tempdir().unwrap();
        let log_dir = dir.path().join("nested").join("logs");

        FileLogger::new(LoggingConfig::new(&log_dir, "app.log")).unwrap();

        assert!(log_dir.exists());
    }

    #[test]
    fn appends_lines() {
        let dir = tempdir().unwrap();
        let logger = FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap();

        logger.debug("runtime", "ignored by default min_level");
        logger.info("runtime", "one");
        logger.warn("runtime", "two");

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        assert_eq!(content.lines().count(), 2);
    }

    #[test]
    fn filters_below_min_level() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.min_level = LogLevel::Warn;
        let logger = FileLogger::new(config).unwrap();

        logger.info("runtime", "ignored");
        logger.warn("runtime", "written");

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        assert_eq!(content.lines().count(), 1);
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["level"], "WARN");
    }

    #[test]
    fn sanitizes_message_newlines_and_truncates() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.max_message_chars = 5;
        let logger = FileLogger::new(config).unwrap();

        logger.info("runtime", "abc\ndefghij");

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["message"], "abc d[TRUNCATED]");
    }

    #[test]
    fn redacts_sensitive_details_before_writing() {
        let dir = tempdir().unwrap();
        let logger = FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap();

        logger
            .log(
                LogEvent::new(LogLevel::Info, "runtime", "details").with_details(
                    serde_json::json!({
                        "password": "super-secret",
                        "safe": "visible",
                        "nested": {"recovery_passphrase": "never"}
                    }),
                ),
            )
            .unwrap();

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["details"]["password"], "[REDACTED]");
        assert_eq!(value["details"]["safe"], "visible");
        assert_eq!(
            value["details"]["nested"]["recovery_passphrase"],
            "[REDACTED]"
        );
        assert!(!content.contains("super-secret"));
        assert!(!content.contains("never"));
    }

    #[test]
    fn limits_oversized_details() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.max_details_bytes = 32;
        let logger = FileLogger::new(config).unwrap();

        logger
            .log(
                LogEvent::new(LogLevel::Info, "runtime", "details")
                    .with_details(serde_json::json!({"safe": "x".repeat(256)})),
            )
            .unwrap();

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["details"], "[TRUNCATED]");
    }

    #[test]
    fn rotates_when_file_exceeds_limit() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.max_file_size_mb = 1;
        let active = config.active_path();
        fs::write(&active, vec![b'x'; 1024 * 1024 + 1]).unwrap();
        let logger = FileLogger::new(config).unwrap();

        logger.info("runtime", "after rotation");

        assert!(dir.path().join("app.1.log").exists());
        assert!(active.exists());
    }

    #[test]
    fn max_files_is_respected() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.max_file_size_mb = 1;
        config.max_files = 2;
        let logger = FileLogger::new(config.clone()).unwrap();

        for _ in 0..3 {
            fs::write(config.active_path(), vec![b'x'; 1024 * 1024 + 1]).unwrap();
            logger.info("runtime", "rotate");
        }

        assert!(dir.path().join("app.1.log").exists());
        assert!(!dir.path().join("app.2.log").exists());
    }

    #[test]
    fn retention_deletes_old_logs() {
        let dir = tempdir().unwrap();
        let mut config = LoggingConfig::new(dir.path(), "app.log");
        config.retention_days = 0;
        fs::write(dir.path().join("app.1.log"), "old\n").unwrap();
        fs::write(dir.path().join("other.log"), "keep\n").unwrap();
        let logger = FileLogger::new(config).unwrap();
        logger.info("runtime", "new");

        assert!(!dir.path().join("app.1.log").exists());
        assert!(dir.path().join("other.log").exists());
        assert!(dir.path().join("app.log").exists());
    }

    #[test]
    fn json_is_valid_line_by_line() {
        let dir = tempdir().unwrap();
        let logger = FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap();

        logger.debug("runtime", "one");
        logger.error("runtime", "two");

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        for line in content.lines() {
            serde_json::from_str::<Value>(line).unwrap();
        }
    }

    #[test]
    fn multiple_writes_do_not_panic() {
        let dir = tempdir().unwrap();
        let logger = std::sync::Arc::new(
            FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap(),
        );
        let mut handles = Vec::new();

        for index in 0..16 {
            let logger = std::sync::Arc::clone(&logger);
            handles.push(thread::spawn(move || {
                logger.info("runtime", format!("message-{index}"));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        assert_eq!(content.lines().count(), 16);
    }

    #[test]
    fn log_mini_error_writes_public_error() {
        let dir = tempdir().unwrap();
        let logger = FileLogger::new(LoggingConfig::new(dir.path(), "app.log")).unwrap();
        let err = MiniError::new(
            ErrorCode::new("MINI.SQLITE.BUSY_TIMEOUT").unwrap(),
            Component::new("runtime-bootstrap").unwrap(),
            "runtime technical failure",
        );

        log_mini_error(&logger, &err);

        let content = fs::read_to_string(dir.path().join("app.log")).unwrap();
        let value: Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["component"], "runtime-bootstrap");
        assert_eq!(value["code"], "MINI.SQLITE.BUSY_TIMEOUT");
    }

    #[test]
    fn config_rejects_path_like_file_name() {
        let dir = tempdir().unwrap();
        let config = LoggingConfig::new(dir.path(), "nested/app.log");

        assert_eq!(config.validate().unwrap_err(), LogError::ConfigInvalid);
    }

    #[test]
    fn manifest_does_not_depend_on_database_backend() {
        let manifest =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();

        assert!(!manifest.contains(&format!("rusq{}", "lite")));
        assert!(!manifest.contains(&format!("sq{}", "lite")));
    }

    #[test]
    fn manifest_does_not_depend_on_ui_runtime() {
        let manifest =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();

        assert!(!manifest.contains(&format!("tau{}", "ri")));
    }
}

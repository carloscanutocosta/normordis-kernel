use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SqliteConfig {
    pub database_path: PathBuf,
    pub create_parent_dir: bool,
    pub busy_timeout_ms: u64,
    pub enable_foreign_keys: bool,
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronous,
    pub wal_autocheckpoint_pages: u32,
    pub write_queue_capacity: usize,
    pub write_batch_max_commands: usize,
    pub write_batch_max_delay_ms: u64,
    pub write_retry_max_attempts: u32,
    pub write_retry_base_delay_ms: u64,
    pub write_retry_max_delay_ms: u64,
    pub write_retry_jitter_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteJournalMode {
    Wal,
    Delete,
}

impl SqliteJournalMode {
    pub(crate) fn as_pragma_value(self) -> &'static str {
        match self {
            Self::Wal => "WAL",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteSynchronous {
    Normal,
    Full,
}

impl SqliteSynchronous {
    pub(crate) fn as_pragma_value(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
        }
    }
}

impl SqliteConfig {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            create_parent_dir: true,
            busy_timeout_ms: 5_000,
            enable_foreign_keys: true,
            journal_mode: SqliteJournalMode::Wal,
            synchronous: SqliteSynchronous::Normal,
            wal_autocheckpoint_pages: 1_000,
            write_queue_capacity: 1_024,
            write_batch_max_commands: 64,
            write_batch_max_delay_ms: 5,
            write_retry_max_attempts: 5,
            write_retry_base_delay_ms: 5,
            write_retry_max_delay_ms: 250,
            write_retry_jitter_ms: 5,
        }
    }
}

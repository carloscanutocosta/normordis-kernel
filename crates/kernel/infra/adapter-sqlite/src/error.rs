use support_errors::{Component, ErrorCode, MiniError};

pub const COMPONENT: &str = "adapter-sqlite";

pub const INVALID_PATH: &str = "MINI.SQLITE.INVALID_PATH";
pub const OPEN_FAILED: &str = "MINI.SQLITE.OPEN_FAILED";
pub const CREATE_PARENT_DIR_FAILED: &str = "MINI.SQLITE.CREATE_PARENT_DIR_FAILED";
pub const CONFIGURE_FAILED: &str = "MINI.SQLITE.CONFIGURE_FAILED";
pub const INIT_FAILED: &str = "MINI.SQLITE.INIT_FAILED";
pub const EXECUTE_FAILED: &str = "MINI.SQLITE.EXECUTE_FAILED";
pub const QUERY_FAILED: &str = "MINI.SQLITE.QUERY_FAILED";
pub const TRANSACTION_FAILED: &str = "MINI.SQLITE.TRANSACTION_FAILED";
pub const METADATA_READ_FAILED: &str = "MINI.SQLITE.METADATA_READ_FAILED";
pub const METADATA_WRITE_FAILED: &str = "MINI.SQLITE.METADATA_WRITE_FAILED";
pub const INVALID_METADATA_KEY: &str = "MINI.SQLITE.INVALID_METADATA_KEY";
pub const OPTIMIZE_FAILED: &str = "MINI.SQLITE.OPTIMIZE_FAILED";
pub const CHECKPOINT_FAILED: &str = "MINI.SQLITE.CHECKPOINT_FAILED";
pub const BUSY_TIMEOUT: &str = "MINI.SQLITE.BUSY_TIMEOUT";
pub const WRITE_QUEUE_CLOSED: &str = "MINI.SQLITE.WRITE_QUEUE_CLOSED";
pub const WRITE_QUEUE_FULL: &str = "MINI.SQLITE.WRITE_QUEUE_FULL";
pub const WRITE_QUEUE_FAILED: &str = "MINI.SQLITE.WRITE_QUEUE_FAILED";
pub const WRITE_QUEUE_SHUTDOWN_FAILED: &str = "MINI.SQLITE.WRITE_QUEUE_SHUTDOWN_FAILED";
pub const LOCK_FAILED: &str = "MINI.SQLITE.LOCK_FAILED";

pub fn sqlite_error(code: &str, message: &str) -> MiniError {
    MiniError::new(
        ErrorCode::new(code).expect("adapter-sqlite error codes must be valid"),
        Component::new(COMPONENT).expect("adapter-sqlite component must be valid"),
        message,
    )
}

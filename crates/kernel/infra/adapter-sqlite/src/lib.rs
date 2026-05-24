#![allow(clippy::result_large_err)]

mod compat;
mod config;
mod connection;
mod error;
mod reader;
mod relational;
mod storage_raw;
mod writer;

#[cfg(feature = "encrypted")]
mod key;
#[cfg(feature = "encrypted")]
mod manager;
#[cfg(feature = "encrypted")]
mod storage_mode;
#[cfg(feature = "encrypted")]
mod traits;

pub use compat::{
    apply_pragmas, open_connection, run_migrations, sqlite_uri, with_transaction, RetryPolicy,
    SqliteOpenMode, SqliteOptions, WriteBatch, WriteBatchError,
};
pub use config::{SqliteConfig, SqliteJournalMode, SqliteSynchronous};
pub use connection::SqliteAdapter;
pub use reader::SqliteReader;
pub use relational::{
    apply_relational_pragmas, open_relational_connection, relational_sqlite_uri,
    run_relational_migrations, with_relational_transaction, AttachedRelationalDatabase,
    MultiDbRelationalWriteBatch, RelationalRetryPolicy, RelationalWriteBatch,
    SqliteRelationalConfig, SqliteRelationalOpenMode,
};
pub use storage_raw::SqliteRawStorage;
pub use writer::{SqliteWriteQueue, SqliteWriteQueueMetrics};

#[cfg(feature = "encrypted")]
pub use key::db_key_from_env;
#[cfg(feature = "encrypted")]
pub use manager::DbManager;
#[cfg(feature = "encrypted")]
pub use storage_mode::{DbError, StorageMode};
#[cfg(feature = "encrypted")]
pub use traits::{UsesPlainDb, UsesSecureDb};

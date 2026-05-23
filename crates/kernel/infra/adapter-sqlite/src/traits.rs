use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::storage_mode::DbError;

pub trait UsesPlainDb {
    fn db(&self) -> &Pool<SqliteConnectionManager>;
}

pub trait UsesSecureDb {
    fn secure_db(&self) -> Result<&Pool<SqliteConnectionManager>, DbError>;
}

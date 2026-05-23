use crate::storage_mode::DbError;

pub fn db_key_from_env() -> Result<String, DbError> {
    std::env::var("NORMAXIS_DB_KEY").map_err(|_| DbError::MissingEncryptionKey)
}

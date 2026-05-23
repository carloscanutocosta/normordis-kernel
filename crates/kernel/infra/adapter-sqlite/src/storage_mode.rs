#[derive(Debug, Clone)]
pub enum StorageMode {
    Plain,
    Encrypted { key: String },
    Mixed { secure_key: String },
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Erro de pool: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("Base de dados segura não disponível neste modo de armazenamento")]
    SecureDbNotAvailable,

    #[error("Base de dados plain não disponível neste modo de armazenamento")]
    PlainDbNotAvailable,

    #[error("Chave de encriptação em falta. Define a variável de ambiente NORMAXIS_DB_KEY")]
    MissingEncryptionKey,

    #[error("Chave de encriptação inválida ou base de dados corrompida")]
    InvalidEncryptionKey,

    #[error("Falha na migração: {0}")]
    MigrationError(String),

    #[error("Operação falhada após {0} tentativas (último erro: {1})")]
    Exhausted(u32, String),
}

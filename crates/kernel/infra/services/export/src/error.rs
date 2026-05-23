use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportAdapterError {
    #[error("campo obrigatorio vazio: {0}")]
    EmptyField(&'static str),
    #[error("provider de export nao suportado: {0}")]
    UnsupportedProvider(String),
    #[error("hash_algorithm invalido: {0}")]
    InvalidHashAlgorithm(String),
    #[error("pedido de export invalido: {0}")]
    InvalidRequest(String),
    #[error("path de output invalido: {0}")]
    InvalidOutputPath(PathBuf),
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("erro ZIP/XLSX: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("erro de infra: {0}")]
    Infra(String),
}

pub type Result<T> = std::result::Result<T, ExportAdapterError>;

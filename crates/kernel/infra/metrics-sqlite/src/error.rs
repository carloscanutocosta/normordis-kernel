use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetricsSqliteError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro de serialização: {0}")]
    Json(String),
    #[error("data/hora inválida: {0}")]
    InvalidDateTime(String),
    #[error("valor desconhecido: {0}")]
    UnknownValue(String),
}

impl From<MetricsSqliteError> for core_metrics::MetricError {
    fn from(e: MetricsSqliteError) -> Self {
        match e {
            MetricsSqliteError::Sqlite(ref re) if is_unique_violation(re) => {
                core_metrics::MetricError::Conflict
            }
            _ => core_metrics::MetricError::RepoUnavailable,
        }
    }
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ConstraintViolation,
                ..
            },
            _
        )
    )
}

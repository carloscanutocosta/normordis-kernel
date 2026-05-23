use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanIngestError {
    #[error("failed to build export snapshot: {0}")]
    SnapshotBuild(String),

    #[error("failed to serialize snapshot: {0}")]
    Serialization(String),
}

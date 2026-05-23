use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const METRICS_COMPONENT: &str = "core-metrics";
pub const MISSING_FIELD: &str = "MINI.METRICS.MISSING_FIELD";
pub const INVALID_NAME: &str = "MINI.METRICS.INVALID_NAME";
pub const INVALID_VALUE: &str = "MINI.METRICS.INVALID_VALUE";
pub const MARSHAL_FAILED: &str = "MINI.METRICS.MARSHAL_FAILED";
pub const NOT_FOUND: &str = "MINI.METRICS.NOT_FOUND";
pub const CONFLICT: &str = "MINI.METRICS.CONFLICT";
pub const REPO_UNAVAILABLE: &str = "MINI.METRICS.REPO_UNAVAILABLE";
pub const DATA_CORRUPTION: &str = "MINI.METRICS.DATA_CORRUPTION";
pub const INVALID_CRITERIA: &str = "MINI.METRICS.INVALID_CRITERIA";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetricError {
    #[error("metric field is missing")]
    MissingField,
    #[error("metric name is invalid")]
    InvalidName,
    #[error("metric value is not finite")]
    InvalidValue,
    #[error("metric payload could not be serialized")]
    MarshalFailed,
    #[error("metric event not found")]
    NotFound,
    #[error("metric event already exists")]
    Conflict,
    #[error("metric repository unavailable")]
    RepoUnavailable,
    #[error("metric data corrupted")]
    DataCorruption,
    #[error("metric list criteria invalid")]
    InvalidCriteria,
}

impl MetricError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingField => MISSING_FIELD,
            Self::InvalidName => INVALID_NAME,
            Self::InvalidValue => INVALID_VALUE,
            Self::MarshalFailed => MARSHAL_FAILED,
            Self::NotFound => NOT_FOUND,
            Self::Conflict => CONFLICT,
            Self::RepoUnavailable => REPO_UNAVAILABLE,
            Self::DataCorruption => DATA_CORRUPTION,
            Self::InvalidCriteria => INVALID_CRITERIA,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::MissingField => "required metric field is missing",
            Self::InvalidName => "metric name does not match required pattern [a-z][a-z0-9_.-]*",
            Self::InvalidValue => "metric value is not finite",
            Self::MarshalFailed => "metric payload could not be serialized",
            Self::NotFound => "metric event not found",
            Self::Conflict => "metric event already exists",
            Self::RepoUnavailable => "metric repository is unavailable",
            Self::DataCorruption => "metric data is corrupted",
            Self::InvalidCriteria => "metric list criteria is invalid",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("core-metrics error codes must be valid"),
            Component::new(METRICS_COMPONENT).expect("core-metrics component must be valid"),
            self.public_message(),
        )
    }
}

impl From<MetricError> for MiniError {
    fn from(value: MetricError) -> Self {
        value.to_mini_error()
    }
}

impl From<support_storage::StorageError> for MetricError {
    fn from(_: support_storage::StorageError) -> Self {
        Self::RepoUnavailable
    }
}

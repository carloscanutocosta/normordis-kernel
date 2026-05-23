use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceStatus {
    Running,
    Success,
    Partial,
    Failed,
    Skipped,
}

impl MaintenanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Ok,
    Warning(String),
    Failed(String),
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning(_) => "warning",
            Self::Failed(_) => "failed",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Ok => None,
            Self::Warning(msg) | Self::Failed(msg) => Some(msg.as_str()),
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

#[derive(Debug, Clone)]
pub struct MaintenanceRun {
    pub id: Uuid,
    pub run_date: NaiveDate,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub triggered_by: String,
    pub status: MaintenanceStatus,
    pub backup_path: Option<String>,
    pub backup_size: Option<i64>,
    pub checksum: Option<String>,
    pub purged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DbMaintenanceDetail {
    pub id: Uuid,
    pub maintenance_id: Uuid,
    pub db_name: String,
    pub health_status: HealthStatus,
    pub file_size_bytes: Option<i64>,
    pub wal_size_bytes: Option<i64>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub backup_included: bool,
    pub backup_status: Option<String>,
    pub error_message: Option<String>,
}

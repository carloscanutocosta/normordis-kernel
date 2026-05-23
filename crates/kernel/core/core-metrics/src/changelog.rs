use chrono::{DateTime, Utc};

use crate::error::MetricError;
use crate::pagination::ListOptions;

// ── GovernanceLogEntry ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GovernanceLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub action: String,
    pub from_value: Option<String>,
    pub to_value: String,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
}

// ── GovernanceChangeLog ───────────────────────────────────────────────────────

pub trait GovernanceChangeLog: Send + Sync {
    fn log(&self, entry: &GovernanceLogEntry) -> Result<(), MetricError>;
    fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<GovernanceLogEntry>, MetricError>;
}

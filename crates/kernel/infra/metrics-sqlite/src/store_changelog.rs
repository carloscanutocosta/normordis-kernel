use core_metrics::{GovernanceChangeLog, GovernanceLogEntry, ListOptions, MetricError};
use rusqlite::params;

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset, str_to_dt};
use crate::MetricsSqliteStore;

impl GovernanceChangeLog for MetricsSqliteStore {
    fn log(&self, entry: &GovernanceLogEntry) -> Result<(), MetricError> {
        if entry.id.trim().is_empty()
            || entry.entity_type.trim().is_empty()
            || entry.entity_id.trim().is_empty()
            || entry.action.trim().is_empty()
            || entry.to_value.trim().is_empty()
            || entry.changed_by.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        self.db()
            .execute(
                "INSERT INTO metric_governance_log
                     (id, entity_type, entity_id, action, from_value, to_value, changed_by, changed_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    entry.id,
                    entry.entity_type,
                    entry.entity_id,
                    entry.action,
                    entry.from_value,
                    entry.to_value,
                    entry.changed_by,
                    dt_to_str(entry.changed_at),
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<GovernanceLogEntry>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, entity_type, entity_id, action, from_value, to_value, changed_by, changed_at
                 FROM metric_governance_log
                 WHERE entity_type = ?1 AND entity_id = ?2
                 ORDER BY changed_at DESC{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![entity_type, entity_id], row_to_entry)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }
}

fn row_to_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<GovernanceLogEntry> {
    let changed_s: String = r.get(7)?;
    let changed_at = str_to_dt(&changed_s).unwrap_or_else(|_| chrono::Utc::now());
    Ok(GovernanceLogEntry {
        id: r.get(0)?,
        entity_type: r.get(1)?,
        entity_id: r.get(2)?,
        action: r.get(3)?,
        from_value: r.get(4)?,
        to_value: r.get(5)?,
        changed_by: r.get(6)?,
        changed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::Utc;
    use tempfile::NamedTempFile;

    fn store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn entry(id: &str, entity_id: &str) -> GovernanceLogEntry {
        GovernanceLogEntry {
            id: id.to_string(),
            entity_type: "metric_definition".to_string(),
            entity_id: entity_id.to_string(),
            action: "status_changed".to_string(),
            from_value: Some("draft".to_string()),
            to_value: "active".to_string(),
            changed_by: "admin".to_string(),
            changed_at: Utc::now(),
        }
    }

    #[test]
    fn log_and_list() {
        let s = store();
        s.log(&entry("log-001", "d-001")).unwrap();
        s.log(&entry("log-002", "d-001")).unwrap();
        s.log(&entry("log-003", "d-002")).unwrap();

        let list = s
            .list_for_entity("metric_definition", "d-001", ListOptions::default())
            .unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|e| e.entity_id == "d-001"));
    }

    #[test]
    fn log_preserves_from_value() {
        let s = store();
        let mut e = entry("log-001", "d-001");
        e.from_value = None;
        s.log(&e).unwrap();
        let list = s
            .list_for_entity("metric_definition", "d-001", ListOptions::default())
            .unwrap();
        assert_eq!(list[0].from_value, None);
    }

    #[test]
    fn missing_required_field_returns_error() {
        let s = store();
        let mut e = entry("log-001", "d-001");
        e.changed_by = String::new();
        assert!(matches!(s.log(&e), Err(MetricError::MissingField)));
    }

    #[test]
    fn different_entity_types_are_isolated() {
        let s = store();
        let mut e1 = entry("log-001", "d-001");
        e1.entity_type = "metric_definition".to_string();
        let mut e2 = entry("log-002", "d-001");
        e2.entity_type = "evaluation_cycle".to_string();
        s.log(&e1).unwrap();
        s.log(&e2).unwrap();

        let defs = s
            .list_for_entity("metric_definition", "d-001", ListOptions::default())
            .unwrap();
        let cycles = s
            .list_for_entity("evaluation_cycle", "d-001", ListOptions::default())
            .unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(cycles.len(), 1);
    }
}

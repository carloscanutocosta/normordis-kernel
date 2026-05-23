use core_metrics::{MetricError, MetricEvent, MetricListCriteria, MetricStore};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, str_to_dt};
use crate::MetricsSqliteStore;

impl MetricStore for MetricsSqliteStore {
    fn save(&self, event: MetricEvent) -> Result<(), MetricError> {
        event.validate()?;
        let labels = event
            .labels
            .as_ref()
            .map(|l| serde_json::to_string(l))
            .transpose()
            .map_err(|_| MetricError::MarshalFailed)?;
        let payload = event
            .payload
            .as_ref()
            .map(|p| serde_json::to_string(p))
            .transpose()
            .map_err(|_| MetricError::MarshalFailed)?;
        self.db()
            .execute(
                "INSERT INTO metric_events
                     (id, metric_code, metric_version_id, evaluation_cycle_id,
                      value, unit, correlation_id, entity_type, entity_id,
                      state, org_unit_id, source_app, version,
                      valid_at, labels_json, payload_json, timestamp)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![
                    event.id,
                    event.metric_code,
                    event.metric_version_id,
                    event.evaluation_cycle_id,
                    event.value,
                    event.unit,
                    event.correlation_id,
                    event.entity_type,
                    event.entity_id,
                    event.state,
                    event.org_unit_id,
                    event.source_app,
                    event.version,
                    event.valid_at.map(dt_to_str),
                    labels,
                    payload,
                    dt_to_str(event.timestamp),
                ],
            )
            .map_err(|e| MetricsSqliteError::from(e))?;
        Ok(())
    }

    fn get_by_id(&self, id: &str) -> Result<MetricEvent, MetricError> {
        let row = self.db()
            .query_row(
                "SELECT id, metric_code, metric_version_id, evaluation_cycle_id,
                        value, unit, correlation_id, entity_type, entity_id,
                        state, org_unit_id, source_app, version,
                        valid_at, labels_json, payload_json, timestamp
                 FROM metric_events WHERE id = ?1",
                params![id],
                row_to_event,
            )
            .optional()
            .map_err(|e| MetricsSqliteError::from(e))?;
        row.ok_or(MetricError::NotFound)
    }

    fn list(&self, criteria: &MetricListCriteria) -> Result<Vec<MetricEvent>, MetricError> {
        criteria.validate()?;
        let limit = if criteria.limit == 0 {
            core_metrics::DEFAULT_LIST_LIMIT
        } else {
            criteria.limit
        };
        let mut sql = String::from(
            "SELECT id, metric_code, metric_version_id, evaluation_cycle_id,
                    value, unit, correlation_id, entity_type, entity_id,
                    state, org_unit_id, source_app, version,
                    valid_at, labels_json, payload_json, timestamp
             FROM metric_events WHERE 1=1",
        );
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(code) = &criteria.metric_code {
            sql.push_str(" AND metric_code = ?");
            binds.push(Box::new(code.clone()));
        }
        if let Some(vid) = &criteria.metric_version_id {
            sql.push_str(" AND metric_version_id = ?");
            binds.push(Box::new(vid.clone()));
        }
        if let Some(cid) = &criteria.evaluation_cycle_id {
            sql.push_str(" AND evaluation_cycle_id = ?");
            binds.push(Box::new(cid.clone()));
        }
        if let Some(cid) = &criteria.correlation_id {
            sql.push_str(" AND correlation_id = ?");
            binds.push(Box::new(cid.clone()));
        }
        if let Some(et) = &criteria.entity_type {
            sql.push_str(" AND entity_type = ?");
            binds.push(Box::new(et.clone()));
        }
        if let Some(eid) = &criteria.entity_id {
            sql.push_str(" AND entity_id = ?");
            binds.push(Box::new(eid.clone()));
        }
        if let Some(st) = &criteria.state {
            sql.push_str(" AND state = ?");
            binds.push(Box::new(st.clone()));
        }
        if let Some(ou) = &criteria.org_unit_id {
            sql.push_str(" AND org_unit_id = ?");
            binds.push(Box::new(ou.clone()));
        }
        if let Some(sa) = &criteria.source_app {
            sql.push_str(" AND source_app = ?");
            binds.push(Box::new(sa.clone()));
        }
        if let Some(from) = criteria.time_from {
            sql.push_str(" AND timestamp >= ?");
            binds.push(Box::new(dt_to_str(from)));
        }
        if let Some(to) = criteria.time_to {
            sql.push_str(" AND timestamp <= ?");
            binds.push(Box::new(dt_to_str(to)));
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
        binds.push(Box::new(limit as i64));
        binds.push(Box::new(criteria.offset as i64));

        let refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let conn = self.db();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MetricsSqliteError::from(e))?;
        let rows = stmt
            .query_map(refs.as_slice(), row_to_event)
            .map_err(|e| MetricsSqliteError::from(e))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MetricsSqliteError::from(e))?);
        }
        Ok(result)
    }
}

fn row_to_event(r: &rusqlite::Row<'_>) -> rusqlite::Result<MetricEvent> {
    use std::collections::HashMap;

    let labels_s: Option<String> = r.get(14)?;
    let payload_s: Option<String> = r.get(15)?;
    let valid_at_s: Option<String> = r.get(13)?;
    let timestamp_s: String = r.get(16)?;

    let labels: Option<HashMap<String, String>> = labels_s
        .as_deref()
        .map(|s| serde_json::from_str(s).unwrap_or_default());
    let payload: Option<Value> = payload_s.as_deref().map(|s| {
        serde_json::from_str(s).unwrap_or(Value::Null)
    });
    let valid_at = valid_at_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let timestamp = str_to_dt(&timestamp_s).unwrap_or_else(|_| chrono::Utc::now());

    Ok(MetricEvent {
        id: r.get(0)?,
        metric_code: r.get(1)?,
        metric_version_id: r.get(2)?,
        evaluation_cycle_id: r.get(3)?,
        value: r.get(4)?,
        unit: r.get(5)?,
        correlation_id: r.get(6)?,
        entity_type: r.get(7)?,
        entity_id: r.get(8)?,
        state: r.get(9)?,
        org_unit_id: r.get(10)?,
        source_app: r.get(11)?,
        version: r.get(12)?,
        valid_at,
        labels: labels.filter(|m| !m.is_empty()),
        payload: payload.filter(|v| !v.is_null()),
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use core_metrics::{new_event, MetricListCriteria};
    use tempfile::NamedTempFile;

    fn store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn ev(id: &str, code: &str) -> MetricEvent {
        new_event(id, code, 1.0, Some("count"), None)
    }

    #[test]
    fn save_and_get_by_id() {
        let s = store();
        s.save(ev("e-001", "process.duration")).unwrap();
        let got = s.get_by_id("e-001").unwrap();
        assert_eq!(got.metric_code, "process.duration");
    }

    #[test]
    fn duplicate_returns_conflict() {
        let s = store();
        s.save(ev("e-001", "process.duration")).unwrap();
        assert_eq!(
            s.save(ev("e-001", "process.duration")),
            Err(MetricError::Conflict)
        );
    }

    #[test]
    fn get_missing_returns_not_found() {
        let s = store();
        assert_eq!(s.get_by_id("nope"), Err(MetricError::NotFound));
    }

    #[test]
    fn list_with_metric_code_filter() {
        let s = store();
        s.save(ev("e-001", "process.duration")).unwrap();
        s.save(ev("e-002", "document.count")).unwrap();

        let r = s
            .list(&MetricListCriteria {
                metric_code: Some("document.count".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "e-002");
    }

    #[test]
    fn list_with_evaluation_cycle_filter() {
        let s = store();
        let mut e1 = ev("e-001", "process.duration");
        e1.evaluation_cycle_id = Some("cycle-siadap-2026".to_string());
        let e2 = ev("e-002", "process.duration");
        s.save(e1).unwrap();
        s.save(e2).unwrap();

        let r = s
            .list(&MetricListCriteria {
                evaluation_cycle_id: Some("cycle-siadap-2026".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "e-001");
    }

    #[test]
    fn list_preserves_labels_and_payload() {
        let s = store();
        let mut e = ev("e-001", "process.duration");
        e.labels = Some([("env".to_string(), "prod".to_string())].into());
        e.payload = Some(serde_json::json!({"cycle": "2026"}));
        s.save(e).unwrap();

        let got = s.get_by_id("e-001").unwrap();
        assert_eq!(got.labels.unwrap()["env"], "prod");
        assert_eq!(got.payload.unwrap()["cycle"], "2026");
    }
}

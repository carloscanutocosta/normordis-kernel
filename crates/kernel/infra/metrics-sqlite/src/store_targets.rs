use core_metrics::{ListOptions, MetricError, TargetDefinition, TargetDefinitionStore};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl TargetDefinitionStore for MetricsSqliteStore {
    fn save_target(&self, t: &TargetDefinition) -> Result<(), MetricError> {
        t.validate()?;
        let thresholds_json =
            serde_json::to_string(&t.thresholds).map_err(|_| MetricError::MarshalFailed)?;
        self.db()
            .execute(
                "INSERT INTO target_definitions
                     (id, metric_version_id, scope_type, scope_id,
                      target_value, unit, thresholds_json,
                      valid_from, valid_to, created_at, created_by)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    t.id,
                    t.metric_version_id,
                    t.scope_type.as_str(),
                    t.scope_id,
                    t.target_value,
                    t.unit,
                    thresholds_json,
                    dt_to_str(t.valid_from),
                    t.valid_to.map(dt_to_str),
                    dt_to_str(t.created_at),
                    t.created_by,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_target(&self, id: &str) -> Result<TargetDefinition, MetricError> {
        let row = self
            .db()
            .query_row(
                "SELECT id, metric_version_id, scope_type, scope_id,
                        target_value, unit, thresholds_json,
                        valid_from, valid_to, created_at, created_by
                 FROM target_definitions WHERE id = ?1",
                params![id],
                row_to_target,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        row.ok_or(MetricError::NotFound)
    }

    fn list_targets_for_version(
        &self,
        metric_version_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<TargetDefinition>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, metric_version_id, scope_type, scope_id,
                        target_value, unit, thresholds_json,
                        valid_from, valid_to, created_at, created_by
                 FROM target_definitions
                 WHERE metric_version_id = ?1
                 ORDER BY scope_type, scope_id{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![metric_version_id], row_to_target)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn list_targets_for_version_and_scope(
        &self,
        metric_version_id: &str,
        scope_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<TargetDefinition>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, metric_version_id, scope_type, scope_id,
                        target_value, unit, thresholds_json,
                        valid_from, valid_to, created_at, created_by
                 FROM target_definitions
                 WHERE metric_version_id = ?1 AND scope_id = ?2
                 ORDER BY valid_from{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![metric_version_id, scope_id], row_to_target)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }
}

fn row_to_target(r: &rusqlite::Row<'_>) -> rusqlite::Result<TargetDefinition> {
    use core_metrics::ScopeType;

    let scope_s: String = r.get(2)?;
    let thresholds_s: String = r.get(6)?;
    let valid_from_s: String = r.get(7)?;
    let valid_to_s: Option<String> = r.get(8)?;
    let created_s: String = r.get(9)?;

    let scope_type = ScopeType::from_str(&scope_s).unwrap_or(ScopeType::Global);
    let thresholds = serde_json::from_str(&thresholds_s).unwrap_or_default();
    let valid_from = crate::util::str_to_dt(&valid_from_s).unwrap_or_else(|_| chrono::Utc::now());
    let valid_to = valid_to_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let created_at = crate::util::str_to_dt(&created_s).unwrap_or_else(|_| chrono::Utc::now());

    Ok(TargetDefinition {
        id: r.get(0)?,
        metric_version_id: r.get(1)?,
        scope_type,
        scope_id: r.get(3)?,
        target_value: r.get(4)?,
        unit: r.get(5)?,
        thresholds,
        valid_from,
        valid_to,
        created_at,
        created_by: r.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::Utc;
    use core_metrics::{ScopeType, Threshold};
    use tempfile::NamedTempFile;

    fn store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn target(id: &str, version_id: &str, scope_id: &str) -> TargetDefinition {
        TargetDefinition {
            id: id.to_string(),
            metric_version_id: version_id.to_string(),
            scope_type: ScopeType::OrgUnit,
            scope_id: scope_id.to_string(),
            target_value: 90.0,
            unit: "percent".to_string(),
            thresholds: vec![],
            valid_from: Utc::now(),
            valid_to: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
        }
    }

    fn seed_version(s: &MetricsSqliteStore) {
        use core_metrics::{MetricDefinition, MetricDefinitionStatus, MetricDefinitionStore};
        use core_metrics::{MetricVersion, MetricVersionStatus, MetricVersionStore};
        let def = MetricDefinition {
            id: "d-001".to_string(),
            code: "proc.duration".to_string(),
            name: "Duração".to_string(),
            description: "Desc".to_string(),
            purpose: "Prop".to_string(),
            owner_org_unit_id: "uo:porto".to_string(),
            governance_owner: "director".to_string(),
            status: MetricDefinitionStatus::Active,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: None,
            updated_by: None,
        };
        s.save_definition(&def).unwrap();
        let ver = MetricVersion {
            id: "v-001".to_string(),
            metric_definition_id: "d-001".to_string(),
            version: "1.0".to_string(),
            status: MetricVersionStatus::Published,
            valid_from: Utc::now(),
            valid_to: None,
            formula_ref: "formula:proc.duration:v1".to_string(),
            calculation_binding: None,
            evidence_requirements: vec![],
            approval_ref: None,
            published_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
        };
        s.save_version(&ver).unwrap();
    }

    #[test]
    fn save_and_get() {
        let s = store();
        seed_version(&s);
        s.save_target(&target("t-001", "v-001", "uo:porto"))
            .unwrap();
        let got = s.get_target("t-001").unwrap();
        assert_eq!(got.target_value, 90.0);
        assert_eq!(got.scope_id, "uo:porto");
    }

    #[test]
    fn list_for_version() {
        let s = store();
        seed_version(&s);
        s.save_target(&target("t-001", "v-001", "uo:porto"))
            .unwrap();
        s.save_target(&target("t-002", "v-001", "uo:lisboa"))
            .unwrap();
        let list = s
            .list_targets_for_version("v-001", ListOptions::default())
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_for_version_and_scope() {
        let s = store();
        seed_version(&s);
        s.save_target(&target("t-001", "v-001", "uo:porto"))
            .unwrap();
        s.save_target(&target("t-002", "v-001", "uo:lisboa"))
            .unwrap();
        let list = s
            .list_targets_for_version_and_scope("v-001", "uo:porto", ListOptions::default())
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "t-001");
    }

    #[test]
    fn thresholds_roundtrip() {
        let s = store();
        seed_version(&s);
        let mut t = target("t-001", "v-001", "uo:porto");
        t.thresholds = vec![Threshold {
            label: "verde".to_string(),
            min_value: Some(80.0),
            max_value: None,
            color: "green".to_string(),
        }];
        s.save_target(&t).unwrap();
        let got = s.get_target("t-001").unwrap();
        assert_eq!(got.thresholds.len(), 1);
        assert_eq!(got.thresholds[0].color, "green");
    }
}

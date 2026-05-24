use core_metrics::{
    IndicatorInstance, IndicatorInstanceStore, InstanceStatus, ListOptions, MetricError,
};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl IndicatorInstanceStore for MetricsSqliteStore {
    fn save_instance(&self, inst: &IndicatorInstance) -> Result<(), MetricError> {
        inst.validate()?;
        self.db()
            .execute(
                "INSERT INTO indicator_instances
                     (id, metric_version_id, evaluation_cycle_id, org_unit_id,
                      responsible_actor_id, scope, status, created_at, created_by, closed_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    inst.id,
                    inst.metric_version_id,
                    inst.evaluation_cycle_id,
                    inst.org_unit_id,
                    inst.responsible_actor_id,
                    inst.scope,
                    inst.status.as_str(),
                    dt_to_str(inst.created_at),
                    inst.created_by,
                    inst.closed_at.map(dt_to_str),
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_instance(&self, id: &str) -> Result<IndicatorInstance, MetricError> {
        let row = self
            .db()
            .query_row(
                "SELECT id, metric_version_id, evaluation_cycle_id, org_unit_id,
                        responsible_actor_id, scope, status, created_at, created_by, closed_at
                 FROM indicator_instances WHERE id = ?1",
                params![id],
                row_to_instance,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        row.ok_or(MetricError::NotFound)
    }

    fn list_instances_for_cycle(
        &self,
        evaluation_cycle_id: &str,
        opts: ListOptions,
        status: Option<&InstanceStatus>,
    ) -> Result<Vec<IndicatorInstance>, MetricError> {
        let lo = limit_offset(&opts);
        let extra = status.map_or(String::new(), |s| format!(" AND status = '{}'", s.as_str()));
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, metric_version_id, evaluation_cycle_id, org_unit_id,
                        responsible_actor_id, scope, status, created_at, created_by, closed_at
                 FROM indicator_instances
                 WHERE evaluation_cycle_id = ?1{extra}
                 ORDER BY org_unit_id{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![evaluation_cycle_id], row_to_instance)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn save_instances_batch(&self, instances: &[IndicatorInstance]) -> Result<(), MetricError> {
        let mut conn = self.db();
        let tx = conn.transaction().map_err(MetricsSqliteError::Sqlite)?;
        for inst in instances {
            inst.validate()?;
            tx.execute(
                "INSERT INTO indicator_instances
                     (id, metric_version_id, evaluation_cycle_id, org_unit_id,
                      responsible_actor_id, scope, status, created_at, created_by, closed_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    inst.id,
                    inst.metric_version_id,
                    inst.evaluation_cycle_id,
                    inst.org_unit_id,
                    inst.responsible_actor_id,
                    inst.scope,
                    inst.status.as_str(),
                    dt_to_str(inst.created_at),
                    inst.created_by,
                    inst.closed_at.map(dt_to_str),
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        }
        tx.commit().map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn list_instances_for_cycle_and_org_unit(
        &self,
        evaluation_cycle_id: &str,
        org_unit_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<IndicatorInstance>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, metric_version_id, evaluation_cycle_id, org_unit_id,
                        responsible_actor_id, scope, status, created_at, created_by, closed_at
                 FROM indicator_instances
                 WHERE evaluation_cycle_id = ?1 AND org_unit_id = ?2
                 ORDER BY metric_version_id{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![evaluation_cycle_id, org_unit_id], row_to_instance)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn update_instance_status(&self, id: &str, status: &InstanceStatus) -> Result<(), MetricError> {
        let mut extra = String::new();
        if matches!(status, InstanceStatus::Closed) {
            extra = format!(", closed_at = '{}'", dt_to_str(chrono::Utc::now()));
        }
        let sql = format!("UPDATE indicator_instances SET status = ?1{extra} WHERE id = ?2");
        let n = self
            .db()
            .execute(&sql, params![status.as_str(), id])
            .map_err(MetricsSqliteError::Sqlite)?;
        if n == 0 {
            return Err(MetricError::NotFound);
        }
        Ok(())
    }
}

fn row_to_instance(r: &rusqlite::Row<'_>) -> rusqlite::Result<IndicatorInstance> {
    let status_s: String = r.get(6)?;
    let created_s: String = r.get(7)?;
    let closed_s: Option<String> = r.get(9)?;

    let status = InstanceStatus::from_str(&status_s).unwrap_or(InstanceStatus::Pending);
    let created_at = crate::util::str_to_dt(&created_s).unwrap_or_else(|_| chrono::Utc::now());
    let closed_at = closed_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());

    Ok(IndicatorInstance {
        id: r.get(0)?,
        metric_version_id: r.get(1)?,
        evaluation_cycle_id: r.get(2)?,
        org_unit_id: r.get(3)?,
        responsible_actor_id: r.get(4)?,
        scope: r.get(5)?,
        status,
        created_at,
        created_by: r.get(8)?,
        closed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::{Duration, Utc};
    use core_metrics::{
        CycleStatus, CycleType, EvaluationCycle, EvaluationCycleStore, InstanceStatus,
        MetricDefinition, MetricDefinitionStatus, MetricDefinitionStore, MetricVersion,
        MetricVersionStatus, MetricVersionStore,
    };
    use tempfile::NamedTempFile;

    fn store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn seed(s: &MetricsSqliteStore) {
        let now = Utc::now();
        let def = MetricDefinition {
            id: "d-001".to_string(),
            code: "proc.duration".to_string(),
            name: "Duração".to_string(),
            description: "Desc".to_string(),
            purpose: "Prop".to_string(),
            owner_org_unit_id: "uo:porto".to_string(),
            governance_owner: "director".to_string(),
            status: MetricDefinitionStatus::Active,
            created_at: now,
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
            valid_from: now,
            valid_to: None,
            formula_ref: "formula:proc.duration:v1".to_string(),
            calculation_binding: None,
            evidence_requirements: vec![],
            approval_ref: None,
            published_at: None,
            created_at: now,
            created_by: "admin".to_string(),
        };
        s.save_version(&ver).unwrap();
        let cycle = EvaluationCycle {
            id: "c-001".to_string(),
            code: "siadap-2026".to_string(),
            name: "SIADAP 2026".to_string(),
            cycle_type: CycleType::SiadapAnnual,
            period_start: now,
            period_end: now + Duration::days(365),
            governance_context: None,
            status: CycleStatus::Open,
            created_at: now,
            created_by: "admin".to_string(),
        };
        s.save_cycle(&cycle).unwrap();
    }

    fn instance(id: &str, org_unit_id: &str) -> IndicatorInstance {
        IndicatorInstance {
            id: id.to_string(),
            metric_version_id: "v-001".to_string(),
            evaluation_cycle_id: "c-001".to_string(),
            org_unit_id: org_unit_id.to_string(),
            responsible_actor_id: "actor-001".to_string(),
            scope: None,
            status: InstanceStatus::Pending,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            closed_at: None,
        }
    }

    #[test]
    fn save_and_get() {
        let s = store();
        seed(&s);
        s.save_instance(&instance("i-001", "uo:porto")).unwrap();
        let got = s.get_instance("i-001").unwrap();
        assert_eq!(got.org_unit_id, "uo:porto");
        assert_eq!(got.status, InstanceStatus::Pending);
    }

    #[test]
    fn list_for_cycle() {
        let s = store();
        seed(&s);
        s.save_instance(&instance("i-001", "uo:porto")).unwrap();
        s.save_instance(&instance("i-002", "uo:lisboa")).unwrap();
        let list = s
            .list_instances_for_cycle("c-001", ListOptions::default(), None)
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_for_cycle_and_org_unit() {
        let s = store();
        seed(&s);
        s.save_instance(&instance("i-001", "uo:porto")).unwrap();
        s.save_instance(&instance("i-002", "uo:lisboa")).unwrap();
        let list = s
            .list_instances_for_cycle_and_org_unit("c-001", "uo:porto", ListOptions::default())
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "i-001");
    }

    #[test]
    fn update_status_sets_closed_at() {
        let s = store();
        seed(&s);
        s.save_instance(&instance("i-001", "uo:porto")).unwrap();
        s.update_instance_status("i-001", &InstanceStatus::Closed)
            .unwrap();
        let got = s.get_instance("i-001").unwrap();
        assert_eq!(got.status, InstanceStatus::Closed);
        assert!(got.closed_at.is_some());
    }

    #[test]
    fn save_instances_batch_is_atomic() {
        let s = store();
        seed(&s);
        let batch = vec![
            instance("i-001", "uo:porto"),
            instance("i-002", "uo:lisboa"),
            instance("i-003", "uo:coimbra"),
        ];
        s.save_instances_batch(&batch).unwrap();
        let list = s
            .list_instances_for_cycle("c-001", ListOptions::default(), None)
            .unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn list_instances_with_status_filter() {
        let s = store();
        seed(&s);
        let i1 = instance("i-001", "uo:porto");
        let mut i2 = instance("i-002", "uo:lisboa");
        i2.status = InstanceStatus::InProgress;
        s.save_instance(&i1).unwrap();
        s.save_instance(&i2).unwrap();
        let pending = s
            .list_instances_for_cycle(
                "c-001",
                ListOptions::default(),
                Some(&InstanceStatus::Pending),
            )
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "i-001");
    }

    #[test]
    fn get_missing_returns_not_found() {
        let s = store();
        assert!(matches!(s.get_instance("nope"), Err(MetricError::NotFound)));
    }
}

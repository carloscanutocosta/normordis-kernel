use core_metrics::{CycleStatus, EvaluationCycle, EvaluationCycleStore, ListOptions, MetricError};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl EvaluationCycleStore for MetricsSqliteStore {
    fn save_cycle(&self, c: &EvaluationCycle) -> Result<(), MetricError> {
        c.validate()?;
        self.db()
            .execute(
                "INSERT INTO evaluation_cycles
                     (id, code, name, cycle_type, period_start, period_end,
                      governance_context, status, created_at, created_by)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    c.id,
                    c.code,
                    c.name,
                    c.cycle_type.as_str(),
                    dt_to_str(c.period_start),
                    dt_to_str(c.period_end),
                    c.governance_context,
                    c.status.as_str(),
                    dt_to_str(c.created_at),
                    c.created_by,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_cycle(&self, id: &str) -> Result<EvaluationCycle, MetricError> {
        load_cycle_by_col(&self.db(), "id", id)
    }

    fn get_cycle_by_code(&self, code: &str) -> Result<EvaluationCycle, MetricError> {
        load_cycle_by_col(&self.db(), "code", code)
    }

    fn list_cycles(
        &self,
        opts: ListOptions,
        status: Option<&CycleStatus>,
    ) -> Result<Vec<EvaluationCycle>, MetricError> {
        let lo = limit_offset(&opts);
        let filter = status
            .map_or(String::new(), |s| format!(" WHERE status = '{}'", s.as_str()));
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, code, name, cycle_type, period_start, period_end,
                        governance_context, status, created_at, created_by
                 FROM evaluation_cycles{filter} ORDER BY period_start DESC{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map([], row_to_cycle)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn update_cycle_status(&self, id: &str, status: &CycleStatus) -> Result<(), MetricError> {
        let n = self.db()
            .execute(
                "UPDATE evaluation_cycles SET status = ?1 WHERE id = ?2",
                params![status.as_str(), id],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        if n == 0 {
            return Err(MetricError::NotFound);
        }
        Ok(())
    }
}

fn load_cycle_by_col(
    conn: &rusqlite::Connection,
    col: &str,
    val: &str,
) -> Result<EvaluationCycle, MetricError> {
    let sql = format!(
        "SELECT id, code, name, cycle_type, period_start, period_end,
                governance_context, status, created_at, created_by
         FROM evaluation_cycles WHERE {col} = ?1"
    );
    let row = conn
        .query_row(&sql, params![val], row_to_cycle)
        .optional()
        .map_err(MetricsSqliteError::Sqlite)?;
    row.ok_or(MetricError::NotFound)
}

fn row_to_cycle(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvaluationCycle> {
    use core_metrics::{CycleStatus, CycleType};

    let cycle_type_s: String = r.get(3)?;
    let period_start_s: String = r.get(4)?;
    let period_end_s: String = r.get(5)?;
    let status_s: String = r.get(7)?;
    let created_s: String = r.get(8)?;

    let cycle_type = CycleType::from_str(&cycle_type_s).unwrap_or(CycleType::Custom);
    let period_start =
        crate::util::str_to_dt(&period_start_s).unwrap_or_else(|_| chrono::Utc::now());
    let period_end =
        crate::util::str_to_dt(&period_end_s).unwrap_or_else(|_| chrono::Utc::now());
    let status = CycleStatus::from_str(&status_s).unwrap_or(CycleStatus::Planned);
    let created_at = crate::util::str_to_dt(&created_s).unwrap_or_else(|_| chrono::Utc::now());

    Ok(EvaluationCycle {
        id: r.get(0)?,
        code: r.get(1)?,
        name: r.get(2)?,
        cycle_type,
        period_start,
        period_end,
        governance_context: r.get(6)?,
        status,
        created_at,
        created_by: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::{Duration, Utc};
    use core_metrics::{CycleStatus, CycleType};
    use tempfile::NamedTempFile;

    fn store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn cycle(id: &str, code: &str) -> EvaluationCycle {
        let now = Utc::now();
        EvaluationCycle {
            id: id.to_string(),
            code: code.to_string(),
            name: "SIADAP 2026".to_string(),
            cycle_type: CycleType::SiadapAnnual,
            period_start: now,
            period_end: now + Duration::days(365),
            governance_context: None,
            status: CycleStatus::Planned,
            created_at: now,
            created_by: "admin".to_string(),
        }
    }

    #[test]
    fn save_and_get_by_code() {
        let s = store();
        s.save_cycle(&cycle("c-001", "siadap-2026")).unwrap();
        let got = s.get_cycle_by_code("siadap-2026").unwrap();
        assert_eq!(got.id, "c-001");
        assert_eq!(got.cycle_type, CycleType::SiadapAnnual);
    }

    #[test]
    fn duplicate_code_returns_conflict() {
        let s = store();
        s.save_cycle(&cycle("c-001", "siadap-2026")).unwrap();
        let err = s.save_cycle(&cycle("c-002", "siadap-2026"));
        assert!(matches!(err, Err(MetricError::Conflict)));
    }

    #[test]
    fn update_status() {
        let s = store();
        s.save_cycle(&cycle("c-001", "siadap-2026")).unwrap();
        s.update_cycle_status("c-001", &CycleStatus::Open).unwrap();
        let got = s.get_cycle("c-001").unwrap();
        assert_eq!(got.status, CycleStatus::Open);
    }

    #[test]
    fn list_cycles() {
        let s = store();
        s.save_cycle(&cycle("c-001", "siadap-2026")).unwrap();
        s.save_cycle(&cycle("c-002", "bsc-q1-2026")).unwrap();
        let list = s.list_cycles(ListOptions::default(), None).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn get_missing_returns_not_found() {
        let s = store();
        assert!(matches!(s.get_cycle("nope"), Err(MetricError::NotFound)));
    }
}

use core_metrics::{
    ListOptions, MetricDefinition, MetricDefinitionStatus, MetricDefinitionStore, MetricError,
};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl MetricDefinitionStore for MetricsSqliteStore {
    fn save_definition(&self, def: &MetricDefinition) -> Result<(), MetricError> {
        def.validate()?;
        self.db()
            .execute(
                "INSERT INTO metric_definitions
                     (id, code, name, description, purpose,
                      owner_org_unit_id, governance_owner, status,
                      created_at, created_by)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    def.id,
                    def.code,
                    def.name,
                    def.description,
                    def.purpose,
                    def.owner_org_unit_id,
                    def.governance_owner,
                    def.status.as_str(),
                    dt_to_str(def.created_at),
                    def.created_by,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_definition(&self, id: &str) -> Result<MetricDefinition, MetricError> {
        load_definition_by_col(&self.db(), "id", id)
    }

    fn get_definition_by_code(&self, code: &str) -> Result<MetricDefinition, MetricError> {
        load_definition_by_col(&self.db(), "code", code)
    }

    fn list_definitions(
        &self,
        opts: ListOptions,
        status: Option<&MetricDefinitionStatus>,
    ) -> Result<Vec<MetricDefinition>, MetricError> {
        let lo = limit_offset(&opts);
        let filter = status.map_or(String::new(), |s| {
            format!(" WHERE status = '{}'", s.as_str())
        });
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, code, name, description, purpose,
                        owner_org_unit_id, governance_owner, status,
                        created_at, created_by, updated_at, updated_by
                 FROM metric_definitions{filter} ORDER BY code{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map([], row_to_definition)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn update_definition_status(
        &self,
        id: &str,
        status: &MetricDefinitionStatus,
        updated_by: &str,
    ) -> Result<(), MetricError> {
        let n = self
            .db()
            .execute(
                "UPDATE metric_definitions SET status = ?1, updated_at = ?2, updated_by = ?3
                 WHERE id = ?4",
                params![
                    status.as_str(),
                    dt_to_str(chrono::Utc::now()),
                    updated_by,
                    id
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        if n == 0 {
            return Err(MetricError::NotFound);
        }
        Ok(())
    }
}

fn load_definition_by_col(
    conn: &rusqlite::Connection,
    col: &str,
    val: &str,
) -> Result<MetricDefinition, MetricError> {
    let sql = format!(
        "SELECT id, code, name, description, purpose,
                owner_org_unit_id, governance_owner, status,
                created_at, created_by, updated_at, updated_by
         FROM metric_definitions WHERE {col} = ?1"
    );
    let row = conn
        .query_row(&sql, params![val], row_to_definition)
        .optional()
        .map_err(MetricsSqliteError::Sqlite)?;
    row.ok_or(MetricError::NotFound)
}

fn row_to_definition(r: &rusqlite::Row<'_>) -> rusqlite::Result<MetricDefinition> {
    let status_s: String = r.get(7)?;
    let created_s: String = r.get(8)?;
    let updated_s: Option<String> = r.get(10)?;

    let status =
        MetricDefinitionStatus::from_str(&status_s).unwrap_or(MetricDefinitionStatus::Draft);
    let created_at = crate::util::str_to_dt(&created_s).unwrap_or_else(|_| chrono::Utc::now());
    let updated_at = updated_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());

    Ok(MetricDefinition {
        id: r.get(0)?,
        code: r.get(1)?,
        name: r.get(2)?,
        description: r.get(3)?,
        purpose: r.get(4)?,
        owner_org_unit_id: r.get(5)?,
        governance_owner: r.get(6)?,
        status,
        created_at,
        created_by: r.get(9)?,
        updated_at,
        updated_by: r.get(11)?,
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

    fn def(id: &str, code: &str) -> MetricDefinition {
        MetricDefinition {
            id: id.to_string(),
            code: code.to_string(),
            name: "Duração de processo".to_string(),
            description: "Tempo médio de duração".to_string(),
            purpose: "Medir eficiência operacional".to_string(),
            owner_org_unit_id: "uo:porto".to_string(),
            governance_owner: "director-servicos".to_string(),
            status: MetricDefinitionStatus::Draft,
            created_at: Utc::now(),
            created_by: "user-001".to_string(),
            updated_at: None,
            updated_by: None,
        }
    }

    #[test]
    fn save_and_get_by_code() {
        let s = store();
        s.save_definition(&def("d-001", "process.duration"))
            .unwrap();
        let got = s.get_definition_by_code("process.duration").unwrap();
        assert_eq!(got.id, "d-001");
        assert_eq!(got.name, "Duração de processo");
    }

    #[test]
    fn duplicate_code_returns_conflict() {
        let s = store();
        s.save_definition(&def("d-001", "process.duration"))
            .unwrap();
        let err = s.save_definition(&def("d-002", "process.duration"));
        assert!(matches!(err, Err(MetricError::Conflict)));
    }

    #[test]
    fn update_status() {
        let s = store();
        s.save_definition(&def("d-001", "process.duration"))
            .unwrap();
        s.update_definition_status("d-001", &MetricDefinitionStatus::Active, "admin")
            .unwrap();
        let got = s.get_definition("d-001").unwrap();
        assert_eq!(got.status, MetricDefinitionStatus::Active);
        assert!(got.updated_by.as_deref() == Some("admin"));
    }

    #[test]
    fn list_definitions() {
        let s = store();
        s.save_definition(&def("d-001", "process.duration"))
            .unwrap();
        s.save_definition(&def("d-002", "document.count")).unwrap();
        let list = s.list_definitions(ListOptions::default(), None).unwrap();
        assert_eq!(list.len(), 2);
    }
}

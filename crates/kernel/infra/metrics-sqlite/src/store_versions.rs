use chrono::{DateTime, Utc};
use core_metrics::{
    CalculationBinding, EvidenceRequirement, ListOptions, MetricError, MetricVersion,
    MetricVersionStatus, MetricVersionStore,
};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl MetricVersionStore for MetricsSqliteStore {
    fn save_version(&self, v: &MetricVersion) -> Result<(), MetricError> {
        v.validate()?;
        let binding_json = v
            .calculation_binding
            .as_ref()
            .map(|b| serde_json::to_string(b))
            .transpose()
            .map_err(|_| MetricError::MarshalFailed)?;
        let evidence_json =
            serde_json::to_string(&v.evidence_requirements).map_err(|_| MetricError::MarshalFailed)?;
        self.db()
            .execute(
                "INSERT INTO metric_versions
                     (id, metric_definition_id, version, status,
                      valid_from, valid_to, formula_ref,
                      calculation_binding_json, evidence_requirements_json,
                      approval_ref, published_at, created_at, created_by)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    v.id,
                    v.metric_definition_id,
                    v.version,
                    v.status.as_str(),
                    dt_to_str(v.valid_from),
                    v.valid_to.map(dt_to_str),
                    v.formula_ref,
                    binding_json,
                    evidence_json,
                    v.approval_ref,
                    v.published_at.map(dt_to_str),
                    dt_to_str(v.created_at),
                    v.created_by,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_version(&self, id: &str) -> Result<MetricVersion, MetricError> {
        let row = self.db()
            .query_row(
                "SELECT id, metric_definition_id, version, status,
                        valid_from, valid_to, formula_ref,
                        calculation_binding_json, evidence_requirements_json,
                        approval_ref, published_at, created_at, created_by
                 FROM metric_versions WHERE id = ?1",
                params![id],
                row_to_version,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        row.ok_or(MetricError::NotFound)
    }

    fn list_versions_for_definition(
        &self,
        metric_definition_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<MetricVersion>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, metric_definition_id, version, status,
                        valid_from, valid_to, formula_ref,
                        calculation_binding_json, evidence_requirements_json,
                        approval_ref, published_at, created_at, created_by
                 FROM metric_versions
                 WHERE metric_definition_id = ?1
                 ORDER BY valid_from{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![metric_definition_id], row_to_version)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn get_active_version_for_code(
        &self,
        metric_code: &str,
        at: DateTime<Utc>,
    ) -> Result<Option<MetricVersion>, MetricError> {
        let at_str = dt_to_str(at);
        let row = self.db()
            .query_row(
                "SELECT mv.id, mv.metric_definition_id, mv.version, mv.status,
                        mv.valid_from, mv.valid_to, mv.formula_ref,
                        mv.calculation_binding_json, mv.evidence_requirements_json,
                        mv.approval_ref, mv.published_at, mv.created_at, mv.created_by
                 FROM metric_versions mv
                 JOIN metric_definitions md ON md.id = mv.metric_definition_id
                 WHERE md.code = ?1
                   AND mv.status = 'published'
                   AND mv.valid_from <= ?2
                   AND (mv.valid_to IS NULL OR mv.valid_to > ?2)
                 ORDER BY mv.valid_from DESC
                 LIMIT 1",
                params![metric_code, at_str],
                row_to_version,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(row)
    }

    fn update_version_status(
        &self,
        id: &str,
        status: &MetricVersionStatus,
        _updated_by: &str,
    ) -> Result<(), MetricError> {
        let mut extra = String::new();
        if matches!(status, MetricVersionStatus::Published) {
            extra = format!(", published_at = '{}'", dt_to_str(Utc::now()));
        }
        let sql = format!(
            "UPDATE metric_versions SET status = ?1{extra} WHERE id = ?2"
        );
        let n = self.db()
            .execute(&sql, params![status.as_str(), id])
            .map_err(MetricsSqliteError::Sqlite)?;
        if n == 0 {
            return Err(MetricError::NotFound);
        }
        Ok(())
    }
}

fn row_to_version(r: &rusqlite::Row<'_>) -> rusqlite::Result<MetricVersion> {
    let status_s: String = r.get(3)?;
    let valid_from_s: String = r.get(4)?;
    let valid_to_s: Option<String> = r.get(5)?;
    let binding_s: Option<String> = r.get(7)?;
    let evidence_s: String = r.get(8)?;
    let published_s: Option<String> = r.get(10)?;
    let created_s: String = r.get(11)?;

    let status =
        MetricVersionStatus::from_str(&status_s).unwrap_or(MetricVersionStatus::Draft);
    let valid_from = crate::util::str_to_dt(&valid_from_s).unwrap_or_else(|_| Utc::now());
    let valid_to = valid_to_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let calculation_binding: Option<CalculationBinding> =
        binding_s.as_deref().and_then(|s| serde_json::from_str(s).ok());
    let evidence_requirements: Vec<EvidenceRequirement> =
        serde_json::from_str(&evidence_s).unwrap_or_default();
    let published_at = published_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let created_at = crate::util::str_to_dt(&created_s).unwrap_or_else(|_| Utc::now());

    Ok(MetricVersion {
        id: r.get(0)?,
        metric_definition_id: r.get(1)?,
        version: r.get(2)?,
        status,
        valid_from,
        valid_to,
        formula_ref: r.get(6)?,
        calculation_binding,
        evidence_requirements,
        approval_ref: r.get(9)?,
        published_at,
        created_at,
        created_by: r.get(12)?,
    })
}

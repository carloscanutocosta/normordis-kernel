use core_metrics::{
    EvidenceLink, ListOptions, MeasurementResult, MeasurementResultStore, MeasurementStatus,
    MetricError,
};
use rusqlite::{params, OptionalExtension};

use crate::error::MetricsSqliteError;
use crate::util::{dt_to_str, limit_offset};
use crate::MetricsSqliteStore;

impl MeasurementResultStore for MetricsSqliteStore {
    fn save_result(&self, res: &MeasurementResult) -> Result<(), MetricError> {
        res.validate()?;
        let quality_json =
            serde_json::to_string(&res.quality_flags).map_err(|_| MetricError::MarshalFailed)?;
        let payload_json = res
            .payload
            .as_ref()
            .map(|p| serde_json::to_string(p).map_err(|_| MetricError::MarshalFailed))
            .transpose()?;
        self.db()
            .execute(
                "INSERT INTO measurement_results
                     (id, indicator_instance_id, metric_version_id,
                      value, unit, status, calculated_at, calculated_by,
                      calculation_snapshot_hash, quality_flags_json,
                      valid_at, rectifies_result_id, payload_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    res.id,
                    res.indicator_instance_id,
                    res.metric_version_id,
                    res.value,
                    res.unit,
                    res.status.as_str(),
                    dt_to_str(res.calculated_at),
                    res.calculated_by,
                    res.calculation_snapshot_hash,
                    quality_json,
                    res.valid_at.map(dt_to_str),
                    res.rectifies_result_id,
                    payload_json,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn save_results_batch(&self, results: &[MeasurementResult]) -> Result<(), MetricError> {
        let mut conn = self.db();
        let tx = conn.transaction().map_err(MetricsSqliteError::Sqlite)?;
        for res in results {
            res.validate()?;
            let quality_json =
                serde_json::to_string(&res.quality_flags).map_err(|_| MetricError::MarshalFailed)?;
            let payload_json = res
                .payload
                .as_ref()
                .map(|p| serde_json::to_string(p).map_err(|_| MetricError::MarshalFailed))
                .transpose()?;
            tx.execute(
                "INSERT INTO measurement_results
                     (id, indicator_instance_id, metric_version_id,
                      value, unit, status, calculated_at, calculated_by,
                      calculation_snapshot_hash, quality_flags_json,
                      valid_at, rectifies_result_id, payload_json)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    res.id,
                    res.indicator_instance_id,
                    res.metric_version_id,
                    res.value,
                    res.unit,
                    res.status.as_str(),
                    dt_to_str(res.calculated_at),
                    res.calculated_by,
                    res.calculation_snapshot_hash,
                    quality_json,
                    res.valid_at.map(dt_to_str),
                    res.rectifies_result_id,
                    payload_json,
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        }
        tx.commit().map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn get_result(&self, id: &str) -> Result<MeasurementResult, MetricError> {
        let row = self.db()
            .query_row(
                "SELECT id, indicator_instance_id, metric_version_id,
                        value, unit, status, calculated_at, calculated_by,
                        calculation_snapshot_hash, quality_flags_json,
                        valid_at, rectifies_result_id, payload_json
                 FROM measurement_results WHERE id = ?1",
                params![id],
                row_to_result,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        row.ok_or(MetricError::NotFound)
    }

    fn list_results_for_instance(
        &self,
        indicator_instance_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<MeasurementResult>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, indicator_instance_id, metric_version_id,
                        value, unit, status, calculated_at, calculated_by,
                        calculation_snapshot_hash, quality_flags_json,
                        valid_at, rectifies_result_id, payload_json
                 FROM measurement_results
                 WHERE indicator_instance_id = ?1
                 ORDER BY calculated_at DESC{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![indicator_instance_id], row_to_result)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }

    fn get_official_result(
        &self,
        indicator_instance_id: &str,
    ) -> Result<Option<MeasurementResult>, MetricError> {
        let row = self.db()
            .query_row(
                "SELECT id, indicator_instance_id, metric_version_id,
                        value, unit, status, calculated_at, calculated_by,
                        calculation_snapshot_hash, quality_flags_json,
                        valid_at, rectifies_result_id, payload_json
                 FROM measurement_results
                 WHERE indicator_instance_id = ?1 AND status = 'validated'
                 ORDER BY calculated_at DESC
                 LIMIT 1",
                params![indicator_instance_id],
                row_to_result,
            )
            .optional()
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(row)
    }

    fn update_result_status(
        &self,
        id: &str,
        status: &MeasurementStatus,
        _updated_by: &str,
    ) -> Result<(), MetricError> {
        let n = self.db()
            .execute(
                "UPDATE measurement_results SET status = ?1 WHERE id = ?2",
                params![status.as_str(), id],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        if n == 0 {
            return Err(MetricError::NotFound);
        }
        Ok(())
    }

    fn save_evidence_link(&self, link: &EvidenceLink) -> Result<(), MetricError> {
        if link.id.trim().is_empty()
            || link.measurement_result_id.trim().is_empty()
            || link.core_ref.trim().is_empty()
            || link.resource_id.trim().is_empty()
        {
            return Err(MetricError::MissingField);
        }
        self.db()
            .execute(
                "INSERT INTO evidence_links
                     (id, measurement_result_id, evidence_type, core_ref,
                      resource_id, correlation_id, hash, valid_at, linked_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    link.id,
                    link.measurement_result_id,
                    link.evidence_type.as_str(),
                    link.core_ref,
                    link.resource_id,
                    link.correlation_id,
                    link.hash,
                    link.valid_at.map(dt_to_str),
                    dt_to_str(link.linked_at),
                ],
            )
            .map_err(MetricsSqliteError::Sqlite)?;
        Ok(())
    }

    fn list_evidence_for_result(
        &self,
        measurement_result_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<EvidenceLink>, MetricError> {
        let lo = limit_offset(&opts);
        let conn = self.db();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, measurement_result_id, evidence_type, core_ref,
                        resource_id, correlation_id, hash, valid_at, linked_at
                 FROM evidence_links
                 WHERE measurement_result_id = ?1
                 ORDER BY linked_at{lo}",
            ))
            .map_err(MetricsSqliteError::Sqlite)?;
        let rows = stmt
            .query_map(params![measurement_result_id], row_to_evidence)
            .map_err(MetricsSqliteError::Sqlite)?;
        rows.map(|r| r.map_err(|e| MetricsSqliteError::Sqlite(e).into()))
            .collect()
    }
}

fn row_to_result(r: &rusqlite::Row<'_>) -> rusqlite::Result<MeasurementResult> {
    let status_s: String = r.get(5)?;
    let calculated_s: String = r.get(6)?;
    let quality_s: String = r.get(9)?;
    let valid_at_s: Option<String> = r.get(10)?;
    let payload_s: Option<String> = r.get(12)?;

    let status = MeasurementStatus::from_str(&status_s).unwrap_or(MeasurementStatus::Calculated);
    let calculated_at =
        crate::util::str_to_dt(&calculated_s).unwrap_or_else(|_| chrono::Utc::now());
    let quality_flags: Vec<String> = serde_json::from_str(&quality_s).unwrap_or_default();
    let valid_at = valid_at_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let payload = payload_s
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(MeasurementResult {
        id: r.get(0)?,
        indicator_instance_id: r.get(1)?,
        metric_version_id: r.get(2)?,
        value: r.get(3)?,
        unit: r.get(4)?,
        status,
        calculated_at,
        calculated_by: r.get(7)?,
        calculation_snapshot_hash: r.get(8)?,
        quality_flags,
        valid_at,
        rectifies_result_id: r.get(11)?,
        payload,
    })
}

fn row_to_evidence(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceLink> {
    use core_metrics::EvidenceType;

    let evidence_type_s: String = r.get(2)?;
    let valid_at_s: Option<String> = r.get(7)?;
    let linked_s: String = r.get(8)?;

    let evidence_type =
        EvidenceType::from_str(&evidence_type_s).unwrap_or(EvidenceType::MetricEvent);
    let valid_at = valid_at_s
        .as_deref()
        .and_then(|s| crate::util::str_to_dt(s).ok());
    let linked_at = crate::util::str_to_dt(&linked_s).unwrap_or_else(|_| chrono::Utc::now());

    Ok(EvidenceLink {
        id: r.get(0)?,
        measurement_result_id: r.get(1)?,
        evidence_type,
        core_ref: r.get(3)?,
        resource_id: r.get(4)?,
        correlation_id: r.get(5)?,
        hash: r.get(6)?,
        valid_at,
        linked_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::{Duration, Utc};
    use core_metrics::{
        CycleStatus, CycleType, EvaluationCycle, EvaluationCycleStore, EvidenceType,
        IndicatorInstance, IndicatorInstanceStore, InstanceStatus, MeasurementStatus,
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
        let inst = IndicatorInstance {
            id: "i-001".to_string(),
            metric_version_id: "v-001".to_string(),
            evaluation_cycle_id: "c-001".to_string(),
            org_unit_id: "uo:porto".to_string(),
            responsible_actor_id: "actor-001".to_string(),
            scope: None,
            status: InstanceStatus::InProgress,
            created_at: now,
            created_by: "admin".to_string(),
            closed_at: None,
        };
        s.save_instance(&inst).unwrap();
    }

    fn result(id: &str) -> MeasurementResult {
        MeasurementResult {
            id: id.to_string(),
            indicator_instance_id: "i-001".to_string(),
            metric_version_id: "v-001".to_string(),
            value: 85.5,
            unit: "percent".to_string(),
            status: MeasurementStatus::Calculated,
            calculated_at: Utc::now(),
            calculated_by: "system".to_string(),
            calculation_snapshot_hash: None,
            quality_flags: vec![],
            valid_at: None,
            rectifies_result_id: None,
            payload: None,
        }
    }

    #[test]
    fn save_and_get() {
        let s = store();
        seed(&s);
        s.save_result(&result("r-001")).unwrap();
        let got = s.get_result("r-001").unwrap();
        assert_eq!(got.value, 85.5);
        assert_eq!(got.status, MeasurementStatus::Calculated);
    }

    #[test]
    fn get_official_result_returns_validated() {
        let s = store();
        seed(&s);
        s.save_result(&result("r-001")).unwrap();
        // no validated result yet
        assert!(s.get_official_result("i-001").unwrap().is_none());
        // promote to validated
        s.update_result_status("r-001", &MeasurementStatus::Validated, "admin")
            .unwrap();
        let official = s.get_official_result("i-001").unwrap();
        assert!(official.is_some());
        assert!(official.unwrap().is_official());
    }

    #[test]
    fn list_results_for_instance() {
        let s = store();
        seed(&s);
        s.save_result(&result("r-001")).unwrap();
        s.save_result(&result("r-002")).unwrap();
        let list = s.list_results_for_instance("i-001", ListOptions::default()).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn quality_flags_roundtrip() {
        let s = store();
        seed(&s);
        let mut r = result("r-001");
        r.quality_flags = vec!["insufficient_data".to_string(), "estimated".to_string()];
        s.save_result(&r).unwrap();
        let got = s.get_result("r-001").unwrap();
        assert_eq!(got.quality_flags, vec!["insufficient_data", "estimated"]);
    }

    #[test]
    fn evidence_link_roundtrip() {
        let s = store();
        seed(&s);
        s.save_result(&result("r-001")).unwrap();
        let link = EvidenceLink {
            id: "ev-001".to_string(),
            measurement_result_id: "r-001".to_string(),
            evidence_type: EvidenceType::AuditEvent,
            core_ref: "core-audit".to_string(),
            resource_id: "audit-001".to_string(),
            correlation_id: None,
            hash: Some("sha256:abc".to_string()),
            valid_at: None,
            linked_at: Utc::now(),
        };
        s.save_evidence_link(&link).unwrap();
        let links = s.list_evidence_for_result("r-001", ListOptions::default()).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].core_ref, "core-audit");
        assert_eq!(links[0].evidence_type, EvidenceType::AuditEvent);
    }

    #[test]
    fn payload_roundtrip() {
        let s = store();
        seed(&s);
        let mut r = result("r-001");
        r.payload = Some(serde_json::json!({"rating": "Adequado", "score": 3}));
        s.save_result(&r).unwrap();
        let got = s.get_result("r-001").unwrap();
        let p = got.payload.unwrap();
        assert_eq!(p["rating"], "Adequado");
        assert_eq!(p["score"], 3);
    }

    #[test]
    fn save_results_batch_is_atomic() {
        let s = store();
        seed(&s);
        let batch = vec![result("r-001"), result("r-002"), result("r-003")];
        s.save_results_batch(&batch).unwrap();
        let list = s.list_results_for_instance("i-001", ListOptions::default()).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn get_missing_returns_not_found() {
        let s = store();
        assert!(matches!(s.get_result("nope"), Err(MetricError::NotFound)));
    }
}

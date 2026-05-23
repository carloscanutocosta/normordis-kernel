use crate::cycle::{CycleStatus, EvaluationCycle};
use crate::definition::{MetricDefinition, MetricDefinitionStatus};
use crate::error::MetricError;
use crate::instance::{IndicatorInstance, InstanceStatus};
use crate::pagination::ListOptions;
use crate::result::{EvidenceLink, MeasurementResult, MeasurementStatus};
use crate::target::TargetDefinition;
use crate::version::{MetricVersion, MetricVersionStatus};


// ── MetricDefinitionStore ──────────────────────────────────────────────────────

pub trait MetricDefinitionStore: Send + Sync {
    fn save_definition(&self, def: &MetricDefinition) -> Result<(), MetricError>;
    fn get_definition(&self, id: &str) -> Result<MetricDefinition, MetricError>;
    fn get_definition_by_code(&self, code: &str) -> Result<MetricDefinition, MetricError>;
    fn list_definitions(
        &self,
        opts: ListOptions,
        status: Option<&MetricDefinitionStatus>,
    ) -> Result<Vec<MetricDefinition>, MetricError>;
    fn update_definition_status(
        &self,
        id: &str,
        status: &MetricDefinitionStatus,
        updated_by: &str,
    ) -> Result<(), MetricError>;
}

// ── MetricVersionStore ─────────────────────────────────────────────────────────

pub trait MetricVersionStore: Send + Sync {
    fn save_version(&self, version: &MetricVersion) -> Result<(), MetricError>;
    fn get_version(&self, id: &str) -> Result<MetricVersion, MetricError>;
    fn list_versions_for_definition(
        &self,
        metric_definition_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<MetricVersion>, MetricError>;
    fn get_active_version_for_code(
        &self,
        metric_code: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<MetricVersion>, MetricError>;
    fn update_version_status(
        &self,
        id: &str,
        status: &MetricVersionStatus,
        updated_by: &str,
    ) -> Result<(), MetricError>;
}

// ── TargetDefinitionStore ──────────────────────────────────────────────────────

pub trait TargetDefinitionStore: Send + Sync {
    fn save_target(&self, target: &TargetDefinition) -> Result<(), MetricError>;
    fn get_target(&self, id: &str) -> Result<TargetDefinition, MetricError>;
    fn list_targets_for_version(
        &self,
        metric_version_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<TargetDefinition>, MetricError>;
    fn list_targets_for_version_and_scope(
        &self,
        metric_version_id: &str,
        scope_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<TargetDefinition>, MetricError>;
}

// ── EvaluationCycleStore ───────────────────────────────────────────────────────

pub trait EvaluationCycleStore: Send + Sync {
    fn save_cycle(&self, cycle: &EvaluationCycle) -> Result<(), MetricError>;
    fn get_cycle(&self, id: &str) -> Result<EvaluationCycle, MetricError>;
    fn get_cycle_by_code(&self, code: &str) -> Result<EvaluationCycle, MetricError>;
    fn list_cycles(
        &self,
        opts: ListOptions,
        status: Option<&CycleStatus>,
    ) -> Result<Vec<EvaluationCycle>, MetricError>;
    fn update_cycle_status(&self, id: &str, status: &CycleStatus) -> Result<(), MetricError>;
}

// ── IndicatorInstanceStore ─────────────────────────────────────────────────────

pub trait IndicatorInstanceStore: Send + Sync {
    fn save_instance(&self, instance: &IndicatorInstance) -> Result<(), MetricError>;
    fn get_instance(&self, id: &str) -> Result<IndicatorInstance, MetricError>;
    fn list_instances_for_cycle(
        &self,
        evaluation_cycle_id: &str,
        opts: ListOptions,
        status: Option<&InstanceStatus>,
    ) -> Result<Vec<IndicatorInstance>, MetricError>;
    fn list_instances_for_cycle_and_org_unit(
        &self,
        evaluation_cycle_id: &str,
        org_unit_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<IndicatorInstance>, MetricError>;
    fn update_instance_status(
        &self,
        id: &str,
        status: &InstanceStatus,
    ) -> Result<(), MetricError>;

    /// Persiste múltiplas instâncias atomicamente.
    ///
    /// A implementação por defeito faz chamadas individuais sem transacção —
    /// adaptadores SQLite devem sobrepor com `BEGIN`/`COMMIT`.
    fn save_instances_batch(&self, instances: &[IndicatorInstance]) -> Result<(), MetricError> {
        instances.iter().try_for_each(|i| self.save_instance(i))
    }
}

// ── MeasurementResultStore ─────────────────────────────────────────────────────

pub trait MeasurementResultStore: Send + Sync {
    fn save_result(&self, result: &MeasurementResult) -> Result<(), MetricError>;
    fn get_result(&self, id: &str) -> Result<MeasurementResult, MetricError>;
    fn list_results_for_instance(
        &self,
        indicator_instance_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<MeasurementResult>, MetricError>;
    fn get_official_result(
        &self,
        indicator_instance_id: &str,
    ) -> Result<Option<MeasurementResult>, MetricError>;
    fn update_result_status(
        &self,
        id: &str,
        status: &MeasurementStatus,
        updated_by: &str,
    ) -> Result<(), MetricError>;
    fn save_evidence_link(&self, link: &EvidenceLink) -> Result<(), MetricError>;
    fn list_evidence_for_result(
        &self,
        measurement_result_id: &str,
        opts: ListOptions,
    ) -> Result<Vec<EvidenceLink>, MetricError>;

    /// Persiste múltiplos resultados atomicamente.
    ///
    /// A implementação por defeito faz chamadas individuais sem transacção —
    /// adaptadores SQLite devem sobrepor com `BEGIN`/`COMMIT`.
    fn save_results_batch(&self, results: &[MeasurementResult]) -> Result<(), MetricError> {
        results.iter().try_for_each(|r| self.save_result(r))
    }
}

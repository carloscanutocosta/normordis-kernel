/// Camada de serviço de métricas — enforcement de invariantes de negócio.
///
/// [`MetricService`] é a **única entrada autorizada** para mutações de estado
/// de governação. Acesso directo aos stores individuais viola os invariantes
/// e pode corromper o estado de governação.
///
/// Invariantes garantidos:
/// 1. `MetricEvent.metric_code` referencia uma `MetricDefinition` activa.
/// 2. `IndicatorInstance` só pode ser criada para ciclos `Open`.
/// 3. `MeasurementResult` valida `EvidenceRequirement`s obrigatórios.
/// 4. Resultados `Validated` não podem ser sobrescritos — exigem rectificação.
/// 5. Rectificação cria novo resultado + marca o anterior como `Rectified`.
/// 6. Publicação de versão verifica ausência de sobreposição temporal.
/// 7. Fecho de ciclo verifica que todas as instâncias têm resultado oficial.
/// 8. Operações em lote são atómicas (os adaptadores SQLite usam transacção).
use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::changelog::{GovernanceChangeLog, GovernanceLogEntry};
use crate::cycle::CycleStatus;
use crate::definition::MetricDefinitionStatus;
use crate::emitter::MetricEmitter;
use crate::error::MetricError;
use crate::event::MetricEvent;
use crate::formula::FormulaEngine;
use crate::governance::{
    EvaluationCycleStore, IndicatorInstanceStore, MeasurementResultStore,
    MetricDefinitionStore, MetricVersionStore, TargetDefinitionStore,
};
use crate::instance::IndicatorInstance;
use crate::pagination::ListOptions;
use crate::result::{EvidenceLink, MeasurementResult, MeasurementStatus};
use crate::version::MetricVersionStatus;

// ── MetricServiceBuilder ──────────────────────────────────────────────────────

/// Constrói um [`MetricService`] com as dependências necessárias.
///
/// # Exemplo mínimo (store unificado)
/// ```rust,ignore
/// let store = Arc::new(MetricsSqliteStore::open(&config)?);
/// let svc = MetricServiceBuilder::from_unified(store, emitter).build();
/// ```
pub struct MetricServiceBuilder {
    definitions: Arc<dyn MetricDefinitionStore>,
    versions: Arc<dyn MetricVersionStore>,
    targets: Arc<dyn TargetDefinitionStore>,
    cycles: Arc<dyn EvaluationCycleStore>,
    instances: Arc<dyn IndicatorInstanceStore>,
    results: Arc<dyn MeasurementResultStore>,
    emitter: Arc<dyn MetricEmitter>,
    changelog: Option<Arc<dyn GovernanceChangeLog>>,
}

impl MetricServiceBuilder {
    pub fn new(
        definitions: Arc<dyn MetricDefinitionStore>,
        versions: Arc<dyn MetricVersionStore>,
        targets: Arc<dyn TargetDefinitionStore>,
        cycles: Arc<dyn EvaluationCycleStore>,
        instances: Arc<dyn IndicatorInstanceStore>,
        results: Arc<dyn MeasurementResultStore>,
        emitter: Arc<dyn MetricEmitter>,
    ) -> Self {
        Self { definitions, versions, targets, cycles, instances, results, emitter, changelog: None }
    }

    /// Regista o changelog de governação (opcional mas recomendado em produção).
    pub fn with_changelog(mut self, changelog: Arc<dyn GovernanceChangeLog>) -> Self {
        self.changelog = Some(changelog);
        self
    }

    /// Conveniência quando um único store implementa todos os traits.
    ///
    /// Também liga o changelog automaticamente.
    pub fn from_unified<S>(store: Arc<S>, emitter: Arc<dyn MetricEmitter>) -> Self
    where
        S: MetricDefinitionStore
            + MetricVersionStore
            + TargetDefinitionStore
            + EvaluationCycleStore
            + IndicatorInstanceStore
            + MeasurementResultStore
            + GovernanceChangeLog
            + 'static,
    {
        let cl: Arc<dyn GovernanceChangeLog> = store.clone();
        Self::new(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            emitter,
        )
        .with_changelog(cl)
    }

    pub fn build(self) -> MetricService {
        MetricService {
            definitions: self.definitions,
            versions: self.versions,
            targets: self.targets,
            cycles: self.cycles,
            instances: self.instances,
            results: self.results,
            emitter: self.emitter,
            changelog: self.changelog,
        }
    }
}

// ── MetricService ─────────────────────────────────────────────────────────────

pub struct MetricService {
    definitions: Arc<dyn MetricDefinitionStore>,
    versions: Arc<dyn MetricVersionStore>,
    targets: Arc<dyn TargetDefinitionStore>,
    cycles: Arc<dyn EvaluationCycleStore>,
    instances: Arc<dyn IndicatorInstanceStore>,
    results: Arc<dyn MeasurementResultStore>,
    emitter: Arc<dyn MetricEmitter>,
    changelog: Option<Arc<dyn GovernanceChangeLog>>,
}

impl MetricService {
    // ── Changelog (best-effort — não falha a operação principal) ─────────────

    fn log(
        &self,
        entity_type: &str,
        entity_id: &str,
        action: &str,
        from: Option<&str>,
        to: &str,
        changed_by: &str,
    ) {
        if let Some(cl) = &self.changelog {
            let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let entry = GovernanceLogEntry {
                id: format!("log.{entity_type}.{entity_id}.{action}.{ts}"),
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                action: action.to_string(),
                from_value: from.map(str::to_string),
                to_value: to.to_string(),
                changed_by: changed_by.to_string(),
                changed_at: Utc::now(),
            };
            let _ = cl.log(&entry);
        }
    }

    // ── Emissão operacional ───────────────────────────────────────────────────

    /// Emite um evento métrico após validar que o código referencia uma
    /// `MetricDefinition` activa.
    pub fn emit_event(&self, event: MetricEvent) -> Result<(), MetricError> {
        event.validate()?;
        let def = self.definitions.get_definition_by_code(&event.metric_code)?;
        if def.status != MetricDefinitionStatus::Active {
            return Err(MetricError::InvalidCriteria);
        }
        self.emitter.emit(event)
    }

    // ── Governação — MetricVersion ────────────────────────────────────────────

    /// Publica uma versão de métrica.
    ///
    /// Valida:
    /// - A versão está em `Draft`.
    /// - Nenhuma outra versão `Published` da mesma definição tem vigência
    ///   sobreposta (temporal overlap).
    pub fn publish_version(
        &self,
        version_id: &str,
        published_by: &str,
    ) -> Result<(), MetricError> {
        let version = self.versions.get_version(version_id)?;
        if !matches!(version.status, MetricVersionStatus::Draft) {
            return Err(MetricError::InvalidCriteria);
        }

        let siblings = self.versions.list_versions_for_definition(
            &version.metric_definition_id,
            ListOptions::unlimited(),
        )?;
        for v in &siblings {
            if v.id == version_id {
                continue;
            }
            if !matches!(v.status, MetricVersionStatus::Published) {
                continue;
            }
            if versions_overlap(v.valid_from, v.valid_to, version.valid_from, version.valid_to) {
                return Err(MetricError::Conflict);
            }
        }

        self.versions.update_version_status(
            version_id,
            &MetricVersionStatus::Published,
            published_by,
        )?;
        self.log(
            "metric_version",
            version_id,
            "status_changed",
            Some("draft"),
            "published",
            published_by,
        );
        Ok(())
    }

    // ── Governação — EvaluationCycle ─────────────────────────────────────────

    /// Fecha um ciclo de avaliação.
    ///
    /// Pré-condições:
    /// - O ciclo está `Open`.
    /// - Todas as instâncias do ciclo têm pelo menos um resultado oficial
    ///   (`Validated`). Se alguma instância não tiver resultado oficial,
    ///   devolve `Err(InvalidCriteria)`.
    ///
    /// A validação de quotas SIADAP é da responsabilidade do caller e deve
    /// ser feita antes de invocar este método.
    pub fn close_cycle(&self, cycle_id: &str, closed_by: &str) -> Result<(), MetricError> {
        let cycle = self.cycles.get_cycle(cycle_id)?;
        if cycle.status != CycleStatus::Open {
            return Err(MetricError::InvalidCriteria);
        }

        let instances =
            self.instances
                .list_instances_for_cycle(cycle_id, ListOptions::unlimited(), None)?;

        for inst in &instances {
            if self.results.get_official_result(&inst.id)?.is_none() {
                return Err(MetricError::InvalidCriteria);
            }
        }

        self.cycles.update_cycle_status(cycle_id, &CycleStatus::Closed)?;
        self.log(
            "evaluation_cycle",
            cycle_id,
            "status_changed",
            Some("open"),
            "closed",
            closed_by,
        );
        Ok(())
    }

    // ── Governação — IndicatorInstance ────────────────────────────────────────

    /// Cria uma instância de indicador (caso singular).
    pub fn create_instance(&self, instance: &IndicatorInstance) -> Result<(), MetricError> {
        instance.validate()?;

        let cycle = self.cycles.get_cycle(&instance.evaluation_cycle_id)?;
        if cycle.status != CycleStatus::Open {
            return Err(MetricError::InvalidCriteria);
        }
        let version = self.versions.get_version(&instance.metric_version_id)?;
        if !matches!(version.status, MetricVersionStatus::Published) {
            return Err(MetricError::InvalidCriteria);
        }

        self.instances.save_instance(instance)?;
        self.log(
            "indicator_instance",
            &instance.id,
            "created",
            None,
            instance.status.as_str(),
            &instance.created_by,
        );
        Ok(())
    }

    /// Cria múltiplas instâncias para um ciclo em operação atómica.
    ///
    /// Valida:
    /// - O ciclo está `Open`.
    /// - Todas as instâncias referenciam o mesmo `cycle_id`.
    /// - Todas as versões referenciadas estão `Published`.
    ///
    /// O adaptador SQLite persiste com transacção; a falha de qualquer
    /// INSERT reverte todas as inserções.
    pub fn open_instances_for_cycle(
        &self,
        cycle_id: &str,
        instances: &[IndicatorInstance],
    ) -> Result<(), MetricError> {
        let cycle = self.cycles.get_cycle(cycle_id)?;
        if cycle.status != CycleStatus::Open {
            return Err(MetricError::InvalidCriteria);
        }

        let mut checked_versions: HashMap<&str, ()> = HashMap::new();
        for inst in instances {
            inst.validate()?;
            if inst.evaluation_cycle_id != cycle_id {
                return Err(MetricError::InvalidCriteria);
            }
            if !checked_versions.contains_key(inst.metric_version_id.as_str()) {
                let version = self.versions.get_version(&inst.metric_version_id)?;
                if !matches!(version.status, MetricVersionStatus::Published) {
                    return Err(MetricError::InvalidCriteria);
                }
                checked_versions.insert(inst.metric_version_id.as_str(), ());
            }
        }

        self.instances.save_instances_batch(instances)
    }

    // ── Governação — MeasurementResult ───────────────────────────────────────

    /// Persiste um resultado de medição (caso singular com evidências).
    pub fn save_result(
        &self,
        result: &MeasurementResult,
        evidence: &[EvidenceLink],
    ) -> Result<(), MetricError> {
        result.validate()?;

        let version = self.versions.get_version(&result.metric_version_id)?;
        let mandatory: Vec<_> = version
            .evidence_requirements
            .iter()
            .filter(|r| r.mandatory)
            .collect();

        if !mandatory.is_empty() {
            for req in &mandatory {
                let satisfied =
                    evidence.iter().any(|l| l.evidence_type.as_str() == req.source_type);
                if !satisfied {
                    return Err(MetricError::MissingField);
                }
            }
        }

        self.results.save_result(result)?;
        for link in evidence {
            self.results.save_evidence_link(link)?;
        }
        self.log(
            "measurement_result",
            &result.id,
            "created",
            None,
            result.status.as_str(),
            &result.calculated_by,
        );
        Ok(())
    }

    /// Persiste múltiplos resultados em operação atómica (sem evidências inline).
    ///
    /// Evidências devem ser persistidas separadamente via `save_evidence_link`
    /// ou numa chamada subsequente.
    pub fn save_results_batch(
        &self,
        results: &[MeasurementResult],
    ) -> Result<(), MetricError> {
        for r in results {
            r.validate()?;
        }
        self.results.save_results_batch(results)
    }

    /// Valida um resultado calculado (`Calculated` → `Validated`).
    pub fn validate_result(&self, id: &str, validated_by: &str) -> Result<(), MetricError> {
        let result = self.results.get_result(id)?;
        if result.status != MeasurementStatus::Calculated {
            return Err(MetricError::InvalidCriteria);
        }
        self.results
            .update_result_status(id, &MeasurementStatus::Validated, validated_by)?;
        self.log(
            "measurement_result",
            id,
            "status_changed",
            Some("calculated"),
            "validated",
            validated_by,
        );
        Ok(())
    }

    /// Invalida um resultado calculado (`Calculated` → `Invalid`).
    pub fn invalidate_result(
        &self,
        id: &str,
        invalidated_by: &str,
    ) -> Result<(), MetricError> {
        let result = self.results.get_result(id)?;
        if !matches!(result.status, MeasurementStatus::Calculated) {
            return Err(MetricError::InvalidCriteria);
        }
        self.results
            .update_result_status(id, &MeasurementStatus::Invalid, invalidated_by)?;
        self.log(
            "measurement_result",
            id,
            "status_changed",
            Some("calculated"),
            "invalid",
            invalidated_by,
        );
        Ok(())
    }

    /// Rectifica um resultado validado.
    ///
    /// Cria novo resultado ligado ao original (`rectifies_result_id`) e
    /// marca o original como `Rectified`.
    pub fn rectify_result(
        &self,
        original_id: &str,
        mut new_result: MeasurementResult,
        evidence: &[EvidenceLink],
        rectified_by: &str,
    ) -> Result<(), MetricError> {
        let original = self.results.get_result(original_id)?;
        if original.status != MeasurementStatus::Validated {
            return Err(MetricError::InvalidCriteria);
        }
        if new_result.id == original_id {
            return Err(MetricError::InvalidCriteria);
        }

        new_result.rectifies_result_id = Some(original_id.to_string());
        new_result.status = MeasurementStatus::Calculated;

        self.results.save_result(&new_result)?;
        for link in evidence {
            self.results.save_evidence_link(link)?;
        }
        self.results
            .update_result_status(original_id, &MeasurementStatus::Rectified, rectified_by)?;
        self.log(
            "measurement_result",
            original_id,
            "status_changed",
            Some("validated"),
            "rectified",
            rectified_by,
        );
        self.log(
            "measurement_result",
            &new_result.id,
            "created",
            None,
            "calculated",
            rectified_by,
        );
        Ok(())
    }

    // ── Orquestração de cálculo ───────────────────────────────────────────────

    /// Calcula um resultado de medição usando o `FormulaEngine` fornecido.
    ///
    /// Orquestra:
    /// 1. Obtenção da instância e da versão activa.
    /// 2. Lookup do target para (versão, unidade orgânica).
    /// 3. Chamada ao `FormulaEngine` com o binding, eventos e target.
    /// 4. Construção do `MeasurementResult` (não persistido — usar `save_result`).
    ///
    /// O caller é responsável por fornecer os eventos relevantes e por
    /// persistir o resultado devolvido.
    pub fn calculate_result(
        &self,
        instance_id: &str,
        events: &[MetricEvent],
        formula_engine: &dyn FormulaEngine,
        result_id: &str,
        unit: &str,
        calculated_by: &str,
    ) -> Result<MeasurementResult, MetricError> {
        let instance = self.instances.get_instance(instance_id)?;
        let version = self.versions.get_version(&instance.metric_version_id)?;

        let binding = version
            .calculation_binding
            .as_ref()
            .ok_or(MetricError::InvalidCriteria)?;

        let targets = self.targets.list_targets_for_version_and_scope(
            &instance.metric_version_id,
            &instance.org_unit_id,
            ListOptions::first(1),
        )?;
        let target = targets.first();

        let value = formula_engine.calculate(binding, events, target)?;
        let effective_unit = target
            .map(|t| t.unit.clone())
            .unwrap_or_else(|| unit.to_string());

        Ok(MeasurementResult {
            id: result_id.to_string(),
            indicator_instance_id: instance_id.to_string(),
            metric_version_id: instance.metric_version_id,
            value,
            unit: effective_unit,
            status: MeasurementStatus::Calculated,
            calculated_at: Utc::now(),
            calculated_by: calculated_by.to_string(),
            calculation_snapshot_hash: None,
            quality_flags: vec![],
            valid_at: None,
            rectifies_result_id: None,
            payload: None,
        })
    }

    // ── Consultas delegadas ───────────────────────────────────────────────────

    pub fn get_official_result(
        &self,
        indicator_instance_id: &str,
    ) -> Result<Option<MeasurementResult>, MetricError> {
        self.results.get_official_result(indicator_instance_id)
    }
}

// ── Helpers privados ──────────────────────────────────────────────────────────

/// Devolve `true` se dois intervalos de vigência se sobrepõem.
///
/// `None` em `valid_to` significa "sem fim definido".
/// Intervalos adjacentes ([a, b] e [b, c]) NÃO se sobrepõem.
fn versions_overlap(
    a_from: DateTime<Utc>,
    a_to: Option<DateTime<Utc>>,
    b_from: DateTime<Utc>,
    b_to: Option<DateTime<Utc>>,
) -> bool {
    let a_ends_before_b_starts = a_to.map(|end| end <= b_from).unwrap_or(false);
    let b_ends_before_a_starts = b_to.map(|end| end <= a_from).unwrap_or(false);
    !a_ends_before_b_starts && !b_ends_before_a_starts
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::{MetricDefinition, MetricDefinitionStatus};
    use crate::emitter::InMemoryMetricRegistry;
    use crate::event::new_event;
    use crate::cycle::{CycleStatus, CycleType, EvaluationCycle};
    use crate::instance::{IndicatorInstance, InstanceStatus};
    use crate::pagination::ListOptions;
    use crate::result::{MeasurementResult, MeasurementStatus};
    use crate::target::TargetDefinition;
    use crate::version::{MetricVersion, MetricVersionStatus};
    use chrono::{Duration, Utc};
    use std::sync::Mutex;

    // ── stores em memória mínimos ─────────────────────────────────────────────

    struct InMemDefinitionStore(Mutex<Vec<MetricDefinition>>);
    impl InMemDefinitionStore {
        fn new(items: Vec<MetricDefinition>) -> Arc<Self> {
            Arc::new(Self(Mutex::new(items)))
        }
    }
    impl MetricDefinitionStore for InMemDefinitionStore {
        fn save_definition(&self, d: &MetricDefinition) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(d.clone());
            Ok(())
        }
        fn get_definition(&self, id: &str) -> Result<MetricDefinition, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|d| d.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn get_definition_by_code(&self, code: &str) -> Result<MetricDefinition, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|d| d.code == code)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_definitions(
            &self,
            _: ListOptions,
            status: Option<&MetricDefinitionStatus>,
        ) -> Result<Vec<MetricDefinition>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|d| status.map(|s| &d.status == s).unwrap_or(true))
                .cloned()
                .collect())
        }
        fn update_definition_status(
            &self,
            id: &str,
            s: &MetricDefinitionStatus,
            _: &str,
        ) -> Result<(), MetricError> {
            let mut v = self.0.lock().unwrap();
            v.iter_mut()
                .find(|d| d.id == id)
                .map(|d| d.status = s.clone())
                .ok_or(MetricError::NotFound)
        }
    }

    struct InMemVersionStore(Mutex<Vec<MetricVersion>>);
    impl InMemVersionStore {
        fn new(items: Vec<MetricVersion>) -> Arc<Self> {
            Arc::new(Self(Mutex::new(items)))
        }
    }
    impl MetricVersionStore for InMemVersionStore {
        fn save_version(&self, v: &MetricVersion) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(v.clone());
            Ok(())
        }
        fn get_version(&self, id: &str) -> Result<MetricVersion, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|v| v.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_versions_for_definition(
            &self,
            id: &str,
            _: ListOptions,
        ) -> Result<Vec<MetricVersion>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v.metric_definition_id == id)
                .cloned()
                .collect())
        }
        fn get_active_version_for_code(
            &self,
            _: &str,
            _: chrono::DateTime<Utc>,
        ) -> Result<Option<MetricVersion>, MetricError> {
            Ok(None)
        }
        fn update_version_status(
            &self,
            id: &str,
            s: &MetricVersionStatus,
            _: &str,
        ) -> Result<(), MetricError> {
            let mut v = self.0.lock().unwrap();
            v.iter_mut()
                .find(|v| v.id == id)
                .map(|v| v.status = s.clone())
                .ok_or(MetricError::NotFound)
        }
    }

    struct InMemTargetStore(Mutex<Vec<TargetDefinition>>);
    impl InMemTargetStore {
        fn new() -> Arc<Self> {
            Arc::new(Self(Mutex::new(vec![])))
        }
    }
    impl TargetDefinitionStore for InMemTargetStore {
        fn save_target(&self, t: &TargetDefinition) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(t.clone());
            Ok(())
        }
        fn get_target(&self, id: &str) -> Result<TargetDefinition, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|t| t.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_targets_for_version(
            &self,
            vid: &str,
            _: ListOptions,
        ) -> Result<Vec<TargetDefinition>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|t| t.metric_version_id == vid)
                .cloned()
                .collect())
        }
        fn list_targets_for_version_and_scope(
            &self,
            vid: &str,
            scope: &str,
            _: ListOptions,
        ) -> Result<Vec<TargetDefinition>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|t| t.metric_version_id == vid && t.scope_id == scope)
                .cloned()
                .collect())
        }
    }

    struct InMemCycleStore(Mutex<Vec<EvaluationCycle>>);
    impl InMemCycleStore {
        fn new(items: Vec<EvaluationCycle>) -> Arc<Self> {
            Arc::new(Self(Mutex::new(items)))
        }
    }
    impl EvaluationCycleStore for InMemCycleStore {
        fn save_cycle(&self, c: &EvaluationCycle) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(c.clone());
            Ok(())
        }
        fn get_cycle(&self, id: &str) -> Result<EvaluationCycle, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn get_cycle_by_code(&self, code: &str) -> Result<EvaluationCycle, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.code == code)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_cycles(
            &self,
            _: ListOptions,
            status: Option<&CycleStatus>,
        ) -> Result<Vec<EvaluationCycle>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|c| status.map(|s| &c.status == s).unwrap_or(true))
                .cloned()
                .collect())
        }
        fn update_cycle_status(&self, id: &str, s: &CycleStatus) -> Result<(), MetricError> {
            let mut v = self.0.lock().unwrap();
            v.iter_mut()
                .find(|c| c.id == id)
                .map(|c| c.status = s.clone())
                .ok_or(MetricError::NotFound)
        }
    }

    struct InMemInstanceStore(Mutex<Vec<IndicatorInstance>>);
    impl InMemInstanceStore {
        fn new() -> Arc<Self> {
            Arc::new(Self(Mutex::new(vec![])))
        }
    }
    impl IndicatorInstanceStore for InMemInstanceStore {
        fn save_instance(&self, i: &IndicatorInstance) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(i.clone());
            Ok(())
        }
        fn get_instance(&self, id: &str) -> Result<IndicatorInstance, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|i| i.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_instances_for_cycle(
            &self,
            cid: &str,
            _: ListOptions,
            status: Option<&InstanceStatus>,
        ) -> Result<Vec<IndicatorInstance>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|i| {
                    i.evaluation_cycle_id == cid
                        && status.map(|s| &i.status == s).unwrap_or(true)
                })
                .cloned()
                .collect())
        }
        fn list_instances_for_cycle_and_org_unit(
            &self,
            cid: &str,
            ou: &str,
            _: ListOptions,
        ) -> Result<Vec<IndicatorInstance>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|i| i.evaluation_cycle_id == cid && i.org_unit_id == ou)
                .cloned()
                .collect())
        }
        fn update_instance_status(
            &self,
            id: &str,
            s: &InstanceStatus,
        ) -> Result<(), MetricError> {
            let mut v = self.0.lock().unwrap();
            v.iter_mut()
                .find(|i| i.id == id)
                .map(|i| i.status = s.clone())
                .ok_or(MetricError::NotFound)
        }
    }

    struct InMemResultStore(Mutex<Vec<MeasurementResult>>, Mutex<Vec<EvidenceLink>>);
    impl InMemResultStore {
        fn new() -> Arc<Self> {
            Arc::new(Self(Mutex::new(vec![]), Mutex::new(vec![])))
        }
    }
    impl MeasurementResultStore for InMemResultStore {
        fn save_result(&self, r: &MeasurementResult) -> Result<(), MetricError> {
            self.0.lock().unwrap().push(r.clone());
            Ok(())
        }
        fn get_result(&self, id: &str) -> Result<MeasurementResult, MetricError> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == id)
                .cloned()
                .ok_or(MetricError::NotFound)
        }
        fn list_results_for_instance(
            &self,
            id: &str,
            _: ListOptions,
        ) -> Result<Vec<MeasurementResult>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.indicator_instance_id == id)
                .cloned()
                .collect())
        }
        fn get_official_result(
            &self,
            id: &str,
        ) -> Result<Option<MeasurementResult>, MetricError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|r| {
                    r.indicator_instance_id == id
                        && r.status == MeasurementStatus::Validated
                })
                .cloned())
        }
        fn update_result_status(
            &self,
            id: &str,
            s: &MeasurementStatus,
            _: &str,
        ) -> Result<(), MetricError> {
            let mut v = self.0.lock().unwrap();
            v.iter_mut()
                .find(|r| r.id == id)
                .map(|r| r.status = s.clone())
                .ok_or(MetricError::NotFound)
        }
        fn save_evidence_link(&self, l: &EvidenceLink) -> Result<(), MetricError> {
            self.1.lock().unwrap().push(l.clone());
            Ok(())
        }
        fn list_evidence_for_result(
            &self,
            id: &str,
            _: ListOptions,
        ) -> Result<Vec<EvidenceLink>, MetricError> {
            Ok(self
                .1
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.measurement_result_id == id)
                .cloned()
                .collect())
        }
    }

    // ── fixtures ──────────────────────────────────────────────────────────────

    fn active_def() -> MetricDefinition {
        MetricDefinition {
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
        }
    }

    fn published_version() -> MetricVersion {
        MetricVersion {
            id: "v-001".to_string(),
            metric_definition_id: "d-001".to_string(),
            version: "1.0".to_string(),
            status: MetricVersionStatus::Published,
            valid_from: Utc::now(),
            valid_to: None,
            formula_ref: "formula:v1".to_string(),
            calculation_binding: None,
            evidence_requirements: vec![],
            approval_ref: None,
            published_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
        }
    }

    fn open_cycle() -> EvaluationCycle {
        let now = Utc::now();
        EvaluationCycle {
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
        }
    }

    fn planned_cycle() -> EvaluationCycle {
        let mut c = open_cycle();
        c.id = "c-002".to_string();
        c.status = CycleStatus::Planned;
        c
    }

    fn make_instance(id: &str) -> IndicatorInstance {
        IndicatorInstance {
            id: id.to_string(),
            metric_version_id: "v-001".to_string(),
            evaluation_cycle_id: "c-001".to_string(),
            org_unit_id: "uo:porto".to_string(),
            responsible_actor_id: "actor-001".to_string(),
            scope: None,
            status: InstanceStatus::Pending,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            closed_at: None,
        }
    }

    fn make_result(id: &str, instance_id: &str, status: MeasurementStatus) -> MeasurementResult {
        MeasurementResult {
            id: id.to_string(),
            indicator_instance_id: instance_id.to_string(),
            metric_version_id: "v-001".to_string(),
            value: 85.0,
            unit: "percent".to_string(),
            status,
            calculated_at: Utc::now(),
            calculated_by: "system".to_string(),
            calculation_snapshot_hash: None,
            quality_flags: vec![],
            valid_at: None,
            rectifies_result_id: None,
            payload: None,
        }
    }

    fn make_service(
        defs: Vec<MetricDefinition>,
        versions: Vec<MetricVersion>,
        cycles: Vec<EvaluationCycle>,
    ) -> (
        MetricService,
        Arc<InMemResultStore>,
        Arc<InMemInstanceStore>,
        Arc<InMemoryMetricRegistry>,
    ) {
        let emitter = Arc::new(InMemoryMetricRegistry::new());
        let results = InMemResultStore::new();
        let instances = InMemInstanceStore::new();
        let targets = InMemTargetStore::new();
        let svc = MetricServiceBuilder::new(
            InMemDefinitionStore::new(defs),
            InMemVersionStore::new(versions),
            targets,
            InMemCycleStore::new(cycles),
            instances.clone(),
            results.clone(),
            emitter.clone(),
        )
        .build();
        (svc, results, instances, emitter)
    }

    // ── testes ────────────────────────────────────────────────────────────────

    #[test]
    fn emit_event_active_definition_succeeds() {
        let (svc, _, _, registry) = make_service(vec![active_def()], vec![], vec![]);
        let event = new_event("e-001", "proc.duration", 42.0, Some("days"), None);
        svc.emit_event(event).unwrap();
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn emit_event_unknown_code_returns_not_found() {
        let (svc, _, _, _) = make_service(vec![], vec![], vec![]);
        let event = new_event("e-001", "proc.duration", 42.0, None::<&str>, None);
        assert_eq!(svc.emit_event(event), Err(MetricError::NotFound));
    }

    #[test]
    fn emit_event_inactive_definition_returns_invalid_criteria() {
        let mut def = active_def();
        def.status = MetricDefinitionStatus::Suspended;
        let (svc, _, _, _) = make_service(vec![def], vec![], vec![]);
        let event = new_event("e-001", "proc.duration", 1.0, None::<&str>, None);
        assert_eq!(svc.emit_event(event), Err(MetricError::InvalidCriteria));
    }

    #[test]
    fn create_instance_for_open_cycle_succeeds() {
        let (svc, _, instances, _) =
            make_service(vec![], vec![published_version()], vec![open_cycle()]);
        svc.create_instance(&make_instance("i-001")).unwrap();
        assert_eq!(instances.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn create_instance_for_planned_cycle_returns_error() {
        let (svc, _, _, _) =
            make_service(vec![], vec![published_version()], vec![planned_cycle()]);
        let mut inst = make_instance("i-001");
        inst.evaluation_cycle_id = "c-002".to_string();
        assert_eq!(svc.create_instance(&inst), Err(MetricError::InvalidCriteria));
    }

    #[test]
    fn open_instances_for_cycle_batch_succeeds() {
        let (svc, _, instances, _) =
            make_service(vec![], vec![published_version()], vec![open_cycle()]);
        let batch = vec![make_instance("i-001"), make_instance("i-002"), make_instance("i-003")];
        svc.open_instances_for_cycle("c-001", &batch).unwrap();
        assert_eq!(instances.0.lock().unwrap().len(), 3);
    }

    #[test]
    fn open_instances_rejects_wrong_cycle() {
        let (svc, _, _, _) =
            make_service(vec![], vec![published_version()], vec![open_cycle()]);
        let mut inst = make_instance("i-001");
        inst.evaluation_cycle_id = "c-other".to_string();
        assert_eq!(
            svc.open_instances_for_cycle("c-001", &[inst]),
            Err(MetricError::InvalidCriteria)
        );
    }

    #[test]
    fn publish_version_prevents_overlap() {
        let emitter = Arc::new(InMemoryMetricRegistry::new());
        let now = Utc::now();
        let published = MetricVersion {
            id: "v-A".to_string(),
            metric_definition_id: "d-001".to_string(),
            version: "1.0".to_string(),
            status: MetricVersionStatus::Published,
            valid_from: now,
            valid_to: None, // extends forever
            formula_ref: "f:v1".to_string(),
            calculation_binding: None,
            evidence_requirements: vec![],
            approval_ref: None,
            published_at: None,
            created_at: now,
            created_by: "admin".to_string(),
        };
        let draft = MetricVersion {
            id: "v-B".to_string(),
            valid_from: now + Duration::days(30),
            status: MetricVersionStatus::Draft,
            version: "2.0".to_string(),
            ..published.clone()
        };
        let versions = InMemVersionStore::new(vec![published, draft]);
        let svc = MetricServiceBuilder::new(
            InMemDefinitionStore::new(vec![]),
            versions,
            InMemTargetStore::new(),
            InMemCycleStore::new(vec![]),
            InMemInstanceStore::new(),
            InMemResultStore::new(),
            emitter,
        )
        .build();
        // v-A published with no end → v-B overlaps
        assert_eq!(svc.publish_version("v-B", "admin"), Err(MetricError::Conflict));
    }

    #[test]
    fn publish_version_non_overlapping_succeeds() {
        let emitter = Arc::new(InMemoryMetricRegistry::new());
        let now = Utc::now();
        let published = MetricVersion {
            id: "v-A".to_string(),
            metric_definition_id: "d-001".to_string(),
            version: "1.0".to_string(),
            status: MetricVersionStatus::Published,
            valid_from: now - Duration::days(365),
            valid_to: Some(now),  // ends exactly now (adjacent)
            formula_ref: "f:v1".to_string(),
            calculation_binding: None,
            evidence_requirements: vec![],
            approval_ref: None,
            published_at: None,
            created_at: now,
            created_by: "admin".to_string(),
        };
        let draft = MetricVersion {
            id: "v-B".to_string(),
            valid_from: now,  // starts where A ends → adjacent, no overlap
            valid_to: None,
            status: MetricVersionStatus::Draft,
            version: "2.0".to_string(),
            ..published.clone()
        };
        let versions = InMemVersionStore::new(vec![published, draft]);
        let svc = MetricServiceBuilder::new(
            InMemDefinitionStore::new(vec![]),
            versions,
            InMemTargetStore::new(),
            InMemCycleStore::new(vec![]),
            InMemInstanceStore::new(),
            InMemResultStore::new(),
            emitter,
        )
        .build();
        assert!(svc.publish_version("v-B", "admin").is_ok());
    }

    #[test]
    fn close_cycle_requires_all_instances_have_official_result() {
        let (svc, results, instances, _) =
            make_service(vec![], vec![published_version()], vec![open_cycle()]);
        // seed two instances, only one with official result
        instances.0.lock().unwrap().push(make_instance("i-001"));
        instances.0.lock().unwrap().push(make_instance("i-002"));
        results.0.lock().unwrap().push(make_result("r-001", "i-001", MeasurementStatus::Validated));
        // i-002 has no official result → close_cycle must fail
        assert_eq!(svc.close_cycle("c-001", "director"), Err(MetricError::InvalidCriteria));
        // add official result for i-002
        results.0.lock().unwrap().push(make_result("r-002", "i-002", MeasurementStatus::Validated));
        assert!(svc.close_cycle("c-001", "director").is_ok());
    }

    #[test]
    fn rectify_result_creates_chain() {
        let (svc, results, _, _) =
            make_service(vec![], vec![published_version()], vec![]);
        results.0.lock().unwrap().push(make_result("r-001", "i-001", MeasurementStatus::Validated));
        let new_result = make_result("r-002", "i-001", MeasurementStatus::Calculated);
        svc.rectify_result("r-001", new_result, &[], "director").unwrap();
        let store = results.0.lock().unwrap();
        let r001 = store.iter().find(|r| r.id == "r-001").unwrap();
        let r002 = store.iter().find(|r| r.id == "r-002").unwrap();
        assert_eq!(r001.status, MeasurementStatus::Rectified);
        assert_eq!(r002.rectifies_result_id.as_deref(), Some("r-001"));
    }

    #[test]
    fn validate_result_only_from_calculated() {
        let (svc, results, _, _) = make_service(vec![], vec![], vec![]);
        results.0.lock().unwrap().push(make_result("r-001", "i-001", MeasurementStatus::Validated));
        assert_eq!(svc.validate_result("r-001", "director"), Err(MetricError::InvalidCriteria));

        results.0.lock().unwrap().push(make_result("r-002", "i-001", MeasurementStatus::Calculated));
        assert!(svc.validate_result("r-002", "director").is_ok());
    }

    #[test]
    fn versions_overlap_logic() {
        let t0 = Utc::now();
        let t1 = t0 + Duration::days(100);
        let t2 = t0 + Duration::days(200);
        let t3 = t0 + Duration::days(300);

        // Adjacent: [t0,t1] + [t1,t2] → no overlap
        assert!(!versions_overlap(t0, Some(t1), t1, Some(t2)));
        // Overlapping: [t0,t2] + [t1,t3] → overlap
        assert!(versions_overlap(t0, Some(t2), t1, Some(t3)));
        // Open-ended [t0,None] + [t1,t2] → overlap
        assert!(versions_overlap(t0, None, t1, Some(t2)));
        // Open-ended [t0,None] + [t0,None] → overlap
        assert!(versions_overlap(t0, None, t0, None));
        // Disjoint: [t0,t1] + [t2,t3] → no overlap
        assert!(!versions_overlap(t0, Some(t1), t2, Some(t3)));
    }
}

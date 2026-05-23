/// Hierarquia orgânica e agregação multi-nível para BSC/SIADAP.
///
/// A hierarquia de unidades orgânicas (UO) é gerida por `core-org` —
/// este módulo define apenas os traits de integração e a lógica de
/// agregação de resultados entre níveis.
use crate::error::MetricError;
use crate::formula::AggregationKind;
use crate::result::{MeasurementResult, MeasurementStatus};
use crate::governance::{IndicatorInstanceStore, MeasurementResultStore};
use crate::pagination::ListOptions;

// ── OrgHierarchyProvider ──────────────────────────────────────────────────────

/// Fornece a estrutura hierárquica de unidades orgânicas.
///
/// Implementado pelo adapter de `core-org` ou por um stub de teste.
/// Não está implementado em `core-metrics` — depende de dados externos.
pub trait OrgHierarchyProvider: Send + Sync {
    /// Retorna os IDs dos filhos directos de uma unidade orgânica.
    fn children_of(&self, org_unit_id: &str) -> Result<Vec<String>, MetricError>;

    /// Retorna o ID do pai directo, ou `None` para a raiz.
    fn parent_of(&self, org_unit_id: &str) -> Result<Option<String>, MetricError>;

    /// Retorna todos os ancestrais, do mais próximo ao mais distante.
    fn ancestors_of(&self, org_unit_id: &str) -> Result<Vec<String>, MetricError> {
        let mut ancestors = vec![];
        let mut current = org_unit_id.to_string();
        loop {
            match self.parent_of(&current)? {
                Some(parent) => {
                    ancestors.push(parent.clone());
                    current = parent;
                }
                None => break,
            }
        }
        Ok(ancestors)
    }

    /// Retorna todos os descendentes (sub-árvore completa).
    fn all_descendants_of(&self, org_unit_id: &str) -> Result<Vec<String>, MetricError> {
        let mut result = vec![];
        let mut queue = self.children_of(org_unit_id)?;
        while let Some(child) = queue.pop() {
            let grandchildren = self.children_of(&child)?;
            queue.extend(grandchildren);
            result.push(child);
        }
        Ok(result)
    }
}

// ── LevelAggregationService ───────────────────────────────────────────────────

/// Agrega resultados de medição entre níveis da hierarquia orgânica.
///
/// Caso de uso BSC: o resultado de um departamento é a média ponderada
/// dos resultados das suas sub-unidades.
///
/// Caso de uso SIADAP 1: o desempenho de um ministério agrega o
/// desempenho dos serviços que supervisiona.
pub struct LevelAggregationService<H, I, R>
where
    H: OrgHierarchyProvider,
    I: IndicatorInstanceStore,
    R: MeasurementResultStore,
{
    hierarchy: H,
    instances: I,
    results: R,
}

impl<H, I, R> LevelAggregationService<H, I, R>
where
    H: OrgHierarchyProvider,
    I: IndicatorInstanceStore,
    R: MeasurementResultStore,
{
    pub fn new(hierarchy: H, instances: I, results: R) -> Self {
        Self { hierarchy, instances, results }
    }

    /// Agrega os resultados validados dos filhos directos de uma UO.
    ///
    /// Retorna `None` se não houver filhos com resultados validados.
    pub fn aggregate_children(
        &self,
        parent_org_unit_id: &str,
        evaluation_cycle_id: &str,
        metric_version_id: &str,
        kind: AggregationKind,
    ) -> Result<Option<f64>, MetricError> {
        let children = self.hierarchy.children_of(parent_org_unit_id)?;
        if children.is_empty() {
            return Ok(None);
        }

        let mut values: Vec<f64> = vec![];
        for child_id in &children {
            let child_instances = self.instances.list_instances_for_cycle_and_org_unit(
                evaluation_cycle_id,
                child_id,
                ListOptions::unlimited(),
            )?;
            for inst in child_instances {
                if inst.metric_version_id != metric_version_id {
                    continue;
                }
                if let Some(official) = self.results.get_official_result(&inst.id)? {
                    values.push(official.value);
                }
            }
        }

        if values.is_empty() {
            return Ok(None);
        }

        let aggregated = apply_aggregation(&values, &kind);
        Ok(Some(aggregated))
    }

    /// Agrega recursivamente toda a sub-árvore abaixo de uma UO.
    pub fn aggregate_subtree(
        &self,
        root_org_unit_id: &str,
        evaluation_cycle_id: &str,
        metric_version_id: &str,
        kind: AggregationKind,
    ) -> Result<Option<f64>, MetricError> {
        let descendants = self.hierarchy.all_descendants_of(root_org_unit_id)?;
        if descendants.is_empty() {
            return Ok(None);
        }

        let mut values: Vec<f64> = vec![];
        for org_unit_id in &descendants {
            let instances = self.instances.list_instances_for_cycle_and_org_unit(
                evaluation_cycle_id,
                org_unit_id,
                ListOptions::unlimited(),
            )?;
            for inst in instances {
                if inst.metric_version_id != metric_version_id {
                    continue;
                }
                if let Some(official) = self.results.get_official_result(&inst.id)? {
                    values.push(official.value);
                }
            }
        }

        if values.is_empty() {
            return Ok(None);
        }

        Ok(Some(apply_aggregation(&values, &kind)))
    }

    /// Constrói um `MeasurementResult` sintético a partir da agregação de
    /// sub-unidades, para posterior persistência pelo `MeasurementResultStore`.
    pub fn build_aggregated_result(
        &self,
        id: &str,
        indicator_instance_id: &str,
        metric_version_id: &str,
        value: f64,
        unit: &str,
        calculated_by: &str,
    ) -> MeasurementResult {
        MeasurementResult {
            id: id.to_string(),
            indicator_instance_id: indicator_instance_id.to_string(),
            metric_version_id: metric_version_id.to_string(),
            value,
            unit: unit.to_string(),
            status: MeasurementStatus::Calculated,
            calculated_at: chrono::Utc::now(),
            calculated_by: calculated_by.to_string(),
            calculation_snapshot_hash: None,
            quality_flags: vec!["level_aggregation".to_string()],
            valid_at: None,
            rectifies_result_id: None,
            payload: None,
        }
    }
}

fn apply_aggregation(values: &[f64], kind: &AggregationKind) -> f64 {
    match kind {
        AggregationKind::Sum => values.iter().sum(),
        AggregationKind::Average => values.iter().sum::<f64>() / values.len() as f64,
        AggregationKind::Count => values.len() as f64,
        AggregationKind::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
        AggregationKind::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        AggregationKind::Last => *values.last().unwrap_or(&0.0),
        AggregationKind::First => *values.first().unwrap_or(&0.0),
        AggregationKind::WeightedAverage | AggregationKind::Ratio => {
            // Para agregação multi-nível, trata como média simples
            values.iter().sum::<f64>() / values.len() as f64
        }
    }
}

// ── Stub para testes ──────────────────────────────────────────────────────────

/// Implementação estática de `OrgHierarchyProvider` para testes.
pub struct StaticOrgHierarchy {
    /// Mapa `parent_id → Vec<child_id>`.
    pub tree: std::collections::HashMap<String, Vec<String>>,
}

impl StaticOrgHierarchy {
    pub fn new(edges: Vec<(&str, &str)>) -> Self {
        let mut tree: std::collections::HashMap<String, Vec<String>> = Default::default();
        for (parent, child) in edges {
            tree.entry(parent.to_string()).or_default().push(child.to_string());
        }
        Self { tree }
    }
}

impl OrgHierarchyProvider for StaticOrgHierarchy {
    fn children_of(&self, org_unit_id: &str) -> Result<Vec<String>, MetricError> {
        Ok(self.tree.get(org_unit_id).cloned().unwrap_or_default())
    }

    fn parent_of(&self, org_unit_id: &str) -> Result<Option<String>, MetricError> {
        for (parent, children) in &self.tree {
            if children.iter().any(|c| c == org_unit_id) {
                return Ok(Some(parent.clone()));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_hierarchy_children() {
        let h = StaticOrgHierarchy::new(vec![
            ("min:justica", "serv:dgaj"),
            ("min:justica", "serv:dgrs"),
            ("serv:dgaj", "uo:porto"),
        ]);
        let children = h.children_of("min:justica").unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"serv:dgaj".to_string()));
    }

    #[test]
    fn static_hierarchy_parent() {
        let h = StaticOrgHierarchy::new(vec![
            ("min:justica", "serv:dgaj"),
        ]);
        assert_eq!(h.parent_of("serv:dgaj").unwrap(), Some("min:justica".to_string()));
        assert_eq!(h.parent_of("min:justica").unwrap(), None);
    }

    #[test]
    fn ancestors_traversal() {
        let h = StaticOrgHierarchy::new(vec![
            ("root", "mid"),
            ("mid", "leaf"),
        ]);
        let anc = h.ancestors_of("leaf").unwrap();
        assert_eq!(anc, vec!["mid", "root"]);
    }

    #[test]
    fn all_descendants() {
        let h = StaticOrgHierarchy::new(vec![
            ("root", "a"),
            ("root", "b"),
            ("a", "a1"),
            ("a", "a2"),
        ]);
        let mut desc = h.all_descendants_of("root").unwrap();
        desc.sort();
        assert_eq!(desc, vec!["a", "a1", "a2", "b"]);
    }
}

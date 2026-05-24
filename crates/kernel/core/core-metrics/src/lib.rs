#![allow(clippy::should_implement_trait)]

pub mod changelog;
pub mod cycle;
pub mod definition;
pub mod formula;
pub mod governance;
pub mod instance;
pub mod org_hierarchy;
pub mod pagination;
pub mod result;
pub mod siadap;
pub mod target;
pub mod version;

mod emitter;
mod error;
mod event;
mod repository;
pub mod service;

// ── Eventos e emissão ──────────────────────────────────────────────────────────
pub use emitter::{FanoutEmitter, InMemoryMetricRegistry, MetricEmitter};
pub use event::{new_event, MetricEvent};

// ── Erros ─────────────────────────────────────────────────────────────────────
pub use error::{
    MetricError, CONFLICT, DATA_CORRUPTION, INVALID_CRITERIA, INVALID_NAME, INVALID_VALUE,
    MARSHAL_FAILED, METRICS_COMPONENT, MISSING_FIELD, NOT_FOUND, REPO_UNAVAILABLE,
};

// ── Store de eventos (contrato + tipos de filtro) ──────────────────────────────
pub use repository::{
    MetricListCriteria, MetricStore, StoreEmitter, DEFAULT_LIST_LIMIT, DEFAULT_NAMESPACE,
    MAX_LIST_LIMIT,
};

// StorageMetricStore depreciado: use metrics-sqlite::MetricsSqliteStore
#[allow(deprecated)]
pub use repository::StorageMetricStore;

// ── Paginação ──────────────────────────────────────────────────────────────────
pub use pagination::{ListOptions, Page};

// ── Motor de fórmulas ──────────────────────────────────────────────────────────
pub use formula::{AggregationKind, BasicFormulaEngine, FormulaEngine};

// ── Tipos de governação ────────────────────────────────────────────────────────
pub use cycle::{CycleStatus, CycleType, EvaluationCycle};
pub use definition::{MetricDefinition, MetricDefinitionStatus};
pub use governance::{
    EvaluationCycleStore, IndicatorInstanceStore, MeasurementResultStore, MetricDefinitionStore,
    MetricVersionStore, TargetDefinitionStore,
};
pub use instance::{IndicatorInstance, InstanceStatus};
pub use result::{EvidenceLink, EvidenceType, MeasurementResult, MeasurementStatus};
pub use target::{ScopeType, TargetDefinition, Threshold};
pub use version::{CalculationBinding, EvidenceRequirement, MetricVersion, MetricVersionStatus};

// ── Changelog de governação ────────────────────────────────────────────────────
pub use changelog::{GovernanceChangeLog, GovernanceLogEntry};

// ── Serviço ────────────────────────────────────────────────────────────────────
pub use service::{MetricService, MetricServiceBuilder};

// ── Hierarquia orgânica ────────────────────────────────────────────────────────
pub use org_hierarchy::{LevelAggregationService, OrgHierarchyProvider, StaticOrgHierarchy};

// ── SIADAP ─────────────────────────────────────────────────────────────────────
pub use siadap::{
    siadap2_weighted_score, siadap3_weighted_score, validate_score, validate_siadap1_quotas,
    validate_siadap2_quotas, validate_siadap3_quotas, validate_weights,
    IntermediaryEvaluationWindow, QuotaValidationReport, QuotaViolation, Siadap1EvaluationResult,
    Siadap1QuotaConfig, Siadap1Rating, Siadap2EvaluationResult, Siadap2QuotaConfig, Siadap2Rating,
    Siadap3EvaluationResult, Siadap3QuotaConfig, Siadap3Rating,
};

#[cfg(test)]
mod dependency_tests {
    #[test]
    fn core_metrics_nao_depende_de_sqlite() {
        let m = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        assert!(!m.contains("rusqlite") && !m.contains("adapter-sqlite"));
    }

    #[test]
    fn core_metrics_nao_depende_de_tauri() {
        assert!(
            !include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).contains("tauri")
        );
    }

    #[test]
    fn core_metrics_nao_depende_de_core_audit() {
        assert!(
            !include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
                .contains("core-audit")
        );
    }
}

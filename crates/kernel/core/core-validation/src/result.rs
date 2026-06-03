use crate::{ValidationContext, ValidationReport, ValidationSeverity, ValidationStatus};
use serde::{Deserialize, Serialize};

/// Resultado de execução de uma regra individual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOutcome {
    pub rule_id: String,
    pub status: ValidationStatus,
    pub message: Option<String>,
}

impl RuleOutcome {
    pub fn passed(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            status: ValidationStatus::Passed,
            message: None,
        }
    }

    pub fn failed(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            status: ValidationStatus::Failed,
            message: Some(message.into()),
        }
    }

    pub fn skipped(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            status: ValidationStatus::Skipped,
            message: None,
        }
    }

    pub fn not_applicable(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            status: ValidationStatus::NotApplicable,
            message: None,
        }
    }

    pub fn overridden(rule_id: impl Into<String>, justification: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            status: ValidationStatus::Overridden,
            message: Some(justification.into()),
        }
    }
}

/// Artefacto institucional de validação.
///
/// Representa o resultado completo de uma sessão de validação sobre um
/// artefacto identificado (`target_type` + `target_id`). Pode ser armazenado,
/// emitido como evento ou referenciado por core-audit como evidência de controlo.
///
/// Difere de `ValidationReport` em que:
/// - tem identidade própria (`validation_id`)
/// - tem contexto de execução (`context`)
/// - agrupa resultados por regra individual (`outcomes`)
/// - tem estado global calculado (`overall_status`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub validation_id: String,
    pub target_type: String,
    pub target_id: String,
    pub context: Option<ValidationContext>,
    pub overall_status: ValidationStatus,
    pub outcomes: Vec<RuleOutcome>,
}

impl ValidationResult {
    /// Constrói um `ValidationResult` com estado explícito e sem outcomes.
    ///
    /// Requer que `overall_status` seja declarado pelo chamador — não existe
    /// estado inicial implícito. Para construir a partir de um `ValidationReport`
    /// acumulado, usar `from_report` ou `from_report_with_checked`.
    pub fn new(
        validation_id: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        overall_status: ValidationStatus,
    ) -> Self {
        Self {
            validation_id: validation_id.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            context: None,
            overall_status,
            outcomes: Vec::new(),
        }
    }

    pub fn with_context(mut self, context: ValidationContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Constrói um `ValidationResult` a partir de um `ValidationReport` acumulado.
    ///
    /// Cada `ValidationIssue` do report é mapeado para um `RuleOutcome`:
    /// - `Error` → `Failed`
    /// - `Warning` → `Warning`
    /// - `Info` → `Passed`
    ///
    /// O estado global é determinado pelo issue de maior severidade.
    ///
    /// **Limitação:** `ValidationReport` é um acumulador de desvios — só regista
    /// regras que produziram issues. Regras que correram e passaram sem issues não
    /// aparecem nos `outcomes`. Para evidência completa de auditoria (incluindo
    /// regras que passaram), usar `from_report_with_checked`.
    pub fn from_report(
        validation_id: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        context: Option<ValidationContext>,
        report: &ValidationReport,
    ) -> Self {
        let (outcomes, overall_status) = outcomes_from_report(report);
        Self {
            validation_id: validation_id.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            context,
            overall_status,
            outcomes,
        }
    }

    /// Constrói um `ValidationResult` a partir de um `ValidationReport` acumulado,
    /// incluindo outcomes explícitos para regras que correram e passaram.
    ///
    /// `checked_rule_ids` deve conter os IDs de todas as regras que foram aplicadas
    /// ao artefacto. Regras em `checked_rule_ids` que não produziram issues no report
    /// são registadas como `RuleOutcome::passed(...)`. Regras que produziram issues
    /// já estão nos outcomes — não são duplicadas.
    ///
    /// Exemplo:
    /// ```no_run
    /// use core_validation::{ValidationResult, validators::nif};
    ///
    /// let report = nif::validate_nif("nif", "501964843");
    /// let result = ValidationResult::from_report_with_checked(
    ///     "val_001", "Pessoa", "p_abc", None, &report,
    ///     &["validation.nif.format", "validation.nif.checksum"],
    /// );
    /// // outcomes: [{nif.format, Passed}, {nif.checksum, Passed}]
    /// ```
    pub fn from_report_with_checked(
        validation_id: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        context: Option<ValidationContext>,
        report: &ValidationReport,
        checked_rule_ids: &[&str],
    ) -> Self {
        let (mut outcomes, overall_status) = outcomes_from_report(report);

        let issue_rule_ids: std::collections::HashSet<&str> =
            report.issues.iter().map(|i| i.rule_id.as_str()).collect();

        for &rule_id in checked_rule_ids {
            if !issue_rule_ids.contains(rule_id) {
                outcomes.push(RuleOutcome::passed(rule_id));
            }
        }

        Self {
            validation_id: validation_id.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            context,
            overall_status,
            outcomes,
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.overall_status.is_blocking()
    }

    pub fn allows_progression(&self) -> bool {
        self.overall_status.allows_progression()
    }
}

fn outcomes_from_report(report: &ValidationReport) -> (Vec<RuleOutcome>, ValidationStatus) {
    let outcomes: Vec<RuleOutcome> = report
        .issues
        .iter()
        .map(|issue| RuleOutcome {
            rule_id: issue.rule_id.clone(),
            status: match issue.severity {
                ValidationSeverity::Error => ValidationStatus::Failed,
                ValidationSeverity::Warning => ValidationStatus::Warning,
                ValidationSeverity::Info => ValidationStatus::Passed,
            },
            message: Some(issue.message.clone()),
        })
        .collect();

    let overall_status = if !report.is_valid() {
        ValidationStatus::Failed
    } else if report
        .issues
        .iter()
        .any(|i| i.severity == ValidationSeverity::Warning)
    {
        ValidationStatus::Warning
    } else {
        ValidationStatus::Passed
    };

    (outcomes, overall_status)
}

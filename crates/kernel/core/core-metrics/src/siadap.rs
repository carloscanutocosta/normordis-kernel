/// Módulo SIADAP — Sistema Integrado de Gestão e Avaliação do Desempenho
/// na Administração Pública (Lei n.º 66-B/2007 e alterações subsequentes).
///
/// Modela os três subsistemas:
/// - **SIADAP 1** — Serviços e organismos (QUAR)
/// - **SIADAP 2** — Dirigentes (superiores e intermédios)
/// - **SIADAP 3** — Trabalhadores
///
/// As quotas são configuráveis porque têm variado por alterações legais e
/// despachos ministeriais. Os valores padrão reflectem a Lei 66-B/2007.
use serde::{Deserialize, Serialize};

use crate::error::MetricError;

// ── SIADAP 1 ─────────────────────────────────────────────────────────────────

/// Menção qualitativa de desempenho de um serviço/organismo (SIADAP 1).
///
/// Baseada na avaliação global do QUAR (Quadro de Avaliação e
/// Responsabilização) — art.º 13.º e ss. da Lei 66-B/2007.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Siadap1Rating {
    /// Desempenho abaixo dos objectivos mínimos aceitáveis.
    Inadequado,
    /// Desempenho aceitável mas abaixo do esperado.
    Satisfatorio,
    /// Desempenho conforme os objectivos definidos.
    Bom,
    /// Desempenho superior ao esperado.
    MuitoBom,
    /// Desempenho de excelência — requer fundamentação.
    Excelente,
}

impl Siadap1Rating {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inadequado => "inadequado",
            Self::Satisfatorio => "satisfatorio",
            Self::Bom => "bom",
            Self::MuitoBom => "muito_bom",
            Self::Excelente => "excelente",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "inadequado" => Some(Self::Inadequado),
            "satisfatorio" => Some(Self::Satisfatorio),
            "bom" => Some(Self::Bom),
            "muito_bom" => Some(Self::MuitoBom),
            "excelente" => Some(Self::Excelente),
            _ => None,
        }
    }

    /// Score numérico para cálculo de médias ponderadas.
    pub fn score(&self) -> f64 {
        match self {
            Self::Inadequado => 1.0,
            Self::Satisfatorio => 2.0,
            Self::Bom => 3.0,
            Self::MuitoBom => 4.0,
            Self::Excelente => 5.0,
        }
    }

    /// Converte score numérico final para menção.
    ///
    /// Thresholds: <2.0 → Inadequado, <3.0 → Satisfatório,
    /// <4.0 → Bom, <4.5 → Muito Bom, ≥4.5 → Excelente.
    pub fn from_score(score: f64) -> Self {
        if score < 2.0 {
            Self::Inadequado
        } else if score < 3.0 {
            Self::Satisfatorio
        } else if score < 4.0 {
            Self::Bom
        } else if score < 4.5 {
            Self::MuitoBom
        } else {
            Self::Excelente
        }
    }

    /// Indica se a menção está sujeita a quota máxima.
    pub fn requires_quota(&self) -> bool {
        matches!(self, Self::MuitoBom | Self::Excelente)
    }
}

/// Configuração de quotas para SIADAP 1.
///
/// Valores padrão: max 25% Excelente, max 25% Muito Bom.
#[derive(Debug, Clone)]
pub struct Siadap1QuotaConfig {
    pub max_excelente: f64,
    pub max_muito_bom: f64,
}

impl Default for Siadap1QuotaConfig {
    fn default() -> Self {
        Self {
            max_excelente: 0.25,
            max_muito_bom: 0.25,
        }
    }
}

// ── SIADAP 2 ─────────────────────────────────────────────────────────────────

/// Menção qualitativa de desempenho de um dirigente (SIADAP 2).
///
/// Dois componentes: objectivos (ponderação definida por lei/despacho)
/// e competências de liderança/gestão. Escala de 4 níveis — art.º 37.º e ss.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Siadap2Rating {
    Inadequado,
    Adequado,
    MuitoBom,
    Excelente,
}

impl Siadap2Rating {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inadequado => "inadequado",
            Self::Adequado => "adequado",
            Self::MuitoBom => "muito_bom",
            Self::Excelente => "excelente",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "inadequado" => Some(Self::Inadequado),
            "adequado" => Some(Self::Adequado),
            "muito_bom" => Some(Self::MuitoBom),
            "excelente" => Some(Self::Excelente),
            _ => None,
        }
    }

    pub fn score(&self) -> f64 {
        match self {
            Self::Inadequado => 1.0,
            Self::Adequado => 2.0,
            Self::MuitoBom => 3.0,
            Self::Excelente => 4.0,
        }
    }

    pub fn from_score(score: f64) -> Self {
        if score < 2.0 {
            Self::Inadequado
        } else if score < 3.0 {
            Self::Adequado
        } else if score < 3.5 {
            Self::MuitoBom
        } else {
            Self::Excelente
        }
    }

    pub fn requires_quota(&self) -> bool {
        matches!(self, Self::MuitoBom | Self::Excelente)
    }
}

/// Configuração de quotas para SIADAP 2.
///
/// Valores padrão: max 25% para Muito Bom + Excelente combinados.
#[derive(Debug, Clone)]
pub struct Siadap2QuotaConfig {
    /// Quota combinada para Muito Bom + Excelente.
    pub max_muito_bom_excelente_combined: f64,
    /// Dentro da quota combinada, limite para Excelente.
    pub max_excelente: f64,
}

impl Default for Siadap2QuotaConfig {
    fn default() -> Self {
        Self {
            max_muito_bom_excelente_combined: 0.25,
            max_excelente: 0.10,
        }
    }
}

// ── SIADAP 3 ─────────────────────────────────────────────────────────────────

/// Menção qualitativa de desempenho de um trabalhador (SIADAP 3).
///
/// Dois componentes: objectivos (75%) e competências (25%), salvo despacho
/// ministerial em contrário. Escala de 5 níveis — art.º 55.º e ss.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Siadap3Rating {
    Inadequado,
    Adequado,
    BomDesempenho,
    MuitoBom,
    Excelente,
}

impl Siadap3Rating {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inadequado => "inadequado",
            Self::Adequado => "adequado",
            Self::BomDesempenho => "bom_desempenho",
            Self::MuitoBom => "muito_bom",
            Self::Excelente => "excelente",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "inadequado" => Some(Self::Inadequado),
            "adequado" => Some(Self::Adequado),
            "bom_desempenho" => Some(Self::BomDesempenho),
            "muito_bom" => Some(Self::MuitoBom),
            "excelente" => Some(Self::Excelente),
            _ => None,
        }
    }

    pub fn score(&self) -> f64 {
        match self {
            Self::Inadequado => 1.0,
            Self::Adequado => 2.0,
            Self::BomDesempenho => 3.0,
            Self::MuitoBom => 4.0,
            Self::Excelente => 5.0,
        }
    }

    /// Converte score numérico para menção SIADAP 3.
    ///
    /// Thresholds: <2.0 → Inadequado, <3.0 → Adequado,
    /// <4.0 → Bom Desempenho, <4.5 → Muito Bom, ≥4.5 → Excelente.
    pub fn from_score(score: f64) -> Self {
        if score < 2.0 {
            Self::Inadequado
        } else if score < 3.0 {
            Self::Adequado
        } else if score < 4.0 {
            Self::BomDesempenho
        } else if score < 4.5 {
            Self::MuitoBom
        } else {
            Self::Excelente
        }
    }

    pub fn requires_quota(&self) -> bool {
        matches!(self, Self::MuitoBom | Self::Excelente)
    }
}

/// Configuração de quotas para SIADAP 3.
///
/// Valores padrão conforme Lei 66-B/2007: max 25% Excelente,
/// max 25% Muito Bom (quotas separadas — art.º 75.º).
#[derive(Debug, Clone)]
pub struct Siadap3QuotaConfig {
    pub max_excelente: f64,
    pub max_muito_bom: f64,
}

impl Default for Siadap3QuotaConfig {
    fn default() -> Self {
        Self {
            max_excelente: 0.25,
            max_muito_bom: 0.25,
        }
    }
}

// ── Resultado de avaliação por trabalhador/dirigente ─────────────────────────

/// Resultado de avaliação individual para validação de quotas.
#[derive(Debug, Clone)]
pub struct Siadap3EvaluationResult {
    pub entity_id: String,
    pub rating: Siadap3Rating,
    /// Score final ponderado (objectivos × peso + competências × peso).
    pub final_score: f64,
}

#[derive(Debug, Clone)]
pub struct Siadap2EvaluationResult {
    pub entity_id: String,
    pub rating: Siadap2Rating,
    pub final_score: f64,
}

#[derive(Debug, Clone)]
pub struct Siadap1EvaluationResult {
    pub entity_id: String,
    pub rating: Siadap1Rating,
    pub final_score: f64,
}

// ── Relatório de validação de quotas ─────────────────────────────────────────

/// Relatório de validação de quotas para um grupo de avaliações.
#[derive(Debug, Clone)]
pub struct QuotaValidationReport {
    pub total: usize,
    pub violations: Vec<QuotaViolation>,
    pub valid: bool,
}

#[derive(Debug, Clone)]
pub struct QuotaViolation {
    pub rating_label: String,
    pub count: usize,
    pub percentage: f64,
    pub max_allowed: f64,
    pub excess_count: usize,
}

impl QuotaValidationReport {
    fn new(total: usize) -> Self {
        Self {
            total,
            violations: vec![],
            valid: true,
        }
    }

    fn check(&mut self, label: &str, count: usize, max_pct: f64) {
        if self.total == 0 {
            return;
        }
        let pct = count as f64 / self.total as f64;
        if pct > max_pct {
            let allowed = (max_pct * self.total as f64).floor() as usize;
            self.violations.push(QuotaViolation {
                rating_label: label.to_string(),
                count,
                percentage: pct,
                max_allowed: max_pct,
                excess_count: count.saturating_sub(allowed),
            });
            self.valid = false;
        }
    }
}

// ── Funções de cálculo de score ponderado ────────────────────────────────────

/// Calcula o score final SIADAP 3 a partir dos scores de objectivos e
/// competências com pesos configuráveis (padrão: 75%/25%).
pub fn siadap3_weighted_score(
    objectives_score: f64,
    competencies_score: f64,
    objectives_weight: f64,
) -> f64 {
    let comp_weight = 1.0 - objectives_weight;
    objectives_score * objectives_weight + competencies_score * comp_weight
}

/// Calcula o score final SIADAP 2 (objectivos + competências de liderança).
pub fn siadap2_weighted_score(
    objectives_score: f64,
    competencies_score: f64,
    objectives_weight: f64,
) -> f64 {
    siadap3_weighted_score(objectives_score, competencies_score, objectives_weight)
}

// ── Validação de quotas ───────────────────────────────────────────────────────

/// Valida as quotas SIADAP 3 para um conjunto de avaliações de uma UO/ciclo.
///
/// Retorna um relatório com as violações (se houver). Uma quota excedida não
/// impede o armazenamento — é da responsabilidade do serviço decidir se bloqueia.
pub fn validate_siadap3_quotas(
    results: &[Siadap3EvaluationResult],
    config: &Siadap3QuotaConfig,
) -> QuotaValidationReport {
    let total = results.len();
    let mut report = QuotaValidationReport::new(total);

    let excelente = results
        .iter()
        .filter(|r| r.rating == Siadap3Rating::Excelente)
        .count();
    let muito_bom = results
        .iter()
        .filter(|r| r.rating == Siadap3Rating::MuitoBom)
        .count();

    report.check("excelente", excelente, config.max_excelente);
    report.check("muito_bom", muito_bom, config.max_muito_bom);
    report
}

/// Valida as quotas SIADAP 2 (quota combinada Muito Bom + Excelente).
pub fn validate_siadap2_quotas(
    results: &[Siadap2EvaluationResult],
    config: &Siadap2QuotaConfig,
) -> QuotaValidationReport {
    let total = results.len();
    let mut report = QuotaValidationReport::new(total);

    let excelente = results
        .iter()
        .filter(|r| r.rating == Siadap2Rating::Excelente)
        .count();
    let muito_bom = results
        .iter()
        .filter(|r| r.rating == Siadap2Rating::MuitoBom)
        .count();
    let combined = excelente + muito_bom;

    report.check(
        "muito_bom+excelente",
        combined,
        config.max_muito_bom_excelente_combined,
    );
    report.check("excelente", excelente, config.max_excelente);
    report
}

/// Valida as quotas SIADAP 1 (serviços).
pub fn validate_siadap1_quotas(
    results: &[Siadap1EvaluationResult],
    config: &Siadap1QuotaConfig,
) -> QuotaValidationReport {
    let total = results.len();
    let mut report = QuotaValidationReport::new(total);

    let excelente = results
        .iter()
        .filter(|r| r.rating == Siadap1Rating::Excelente)
        .count();
    let muito_bom = results
        .iter()
        .filter(|r| r.rating == Siadap1Rating::MuitoBom)
        .count();

    report.check("excelente", excelente, config.max_excelente);
    report.check("muito_bom", muito_bom, config.max_muito_bom);
    report
}

/// Datas mandatórias de avaliação intercalar para SIADAP 3.
///
/// A avaliação intercalar ocorre entre 1 de Junho e 30 de Junho de cada ano
/// (para ciclos anuais de Janeiro a Dezembro). Os valores exactos podem ser
/// alterados por despacho ministerial.
#[derive(Debug, Clone)]
pub struct IntermediaryEvaluationWindow {
    pub cycle_year: u32,
    pub start_month: u32,
    pub start_day: u32,
    pub end_month: u32,
    pub end_day: u32,
}

impl IntermediaryEvaluationWindow {
    /// Janela padrão: 1 Junho – 30 Junho.
    pub fn standard(cycle_year: u32) -> Self {
        Self {
            cycle_year,
            start_month: 6,
            start_day: 1,
            end_month: 6,
            end_day: 30,
        }
    }

    pub fn contains(&self, date: chrono::NaiveDate) -> bool {
        let start = chrono::NaiveDate::from_ymd_opt(
            self.cycle_year as i32,
            self.start_month,
            self.start_day,
        );
        let end =
            chrono::NaiveDate::from_ymd_opt(self.cycle_year as i32, self.end_month, self.end_day);
        match (start, end) {
            (Some(s), Some(e)) => date >= s && date <= e,
            _ => false,
        }
    }
}

// ── Validação de invariantes SIADAP ──────────────────────────────────────────

/// Valida que um score ponderado está no intervalo válido [1.0, 5.0].
pub fn validate_score(score: f64) -> Result<(), MetricError> {
    if score.is_nan() || score.is_infinite() || !(1.0..=5.0).contains(&score) {
        return Err(MetricError::InvalidValue);
    }
    Ok(())
}

/// Valida que os pesos de objectivos e competências somam 1.0.
pub fn validate_weights(objectives_weight: f64) -> Result<(), MetricError> {
    if !(0.0..=1.0).contains(&objectives_weight) {
        return Err(MetricError::InvalidValue);
    }
    Ok(())
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn s3(id: &str, rating: Siadap3Rating) -> Siadap3EvaluationResult {
        let score = rating.score();
        Siadap3EvaluationResult {
            entity_id: id.to_string(),
            rating,
            final_score: score,
        }
    }

    #[test]
    fn quota_siadap3_no_violations() {
        let results = vec![
            s3("1", Siadap3Rating::Excelente),
            s3("2", Siadap3Rating::MuitoBom),
            s3("3", Siadap3Rating::BomDesempenho),
            s3("4", Siadap3Rating::BomDesempenho),
            s3("5", Siadap3Rating::Adequado),
        ];
        // 1/5 = 20% Excelente ≤ 25%; 1/5 = 20% Muito Bom ≤ 25%
        let report = validate_siadap3_quotas(&results, &Siadap3QuotaConfig::default());
        assert!(report.valid);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn quota_siadap3_excelente_exceeded() {
        let results = vec![
            s3("1", Siadap3Rating::Excelente),
            s3("2", Siadap3Rating::Excelente),
            s3("3", Siadap3Rating::BomDesempenho),
            s3("4", Siadap3Rating::BomDesempenho),
            // 2/4 = 50% Excelente > 25%
        ];
        let report = validate_siadap3_quotas(&results, &Siadap3QuotaConfig::default());
        assert!(!report.valid);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rating_label, "excelente");
        assert_eq!(report.violations[0].excess_count, 1);
    }

    #[test]
    fn siadap3_from_score() {
        assert_eq!(Siadap3Rating::from_score(1.5), Siadap3Rating::Inadequado);
        assert_eq!(Siadap3Rating::from_score(2.5), Siadap3Rating::Adequado);
        assert_eq!(Siadap3Rating::from_score(3.5), Siadap3Rating::BomDesempenho);
        assert_eq!(Siadap3Rating::from_score(4.2), Siadap3Rating::MuitoBom);
        assert_eq!(Siadap3Rating::from_score(4.8), Siadap3Rating::Excelente);
    }

    #[test]
    fn weighted_score_siadap3_default_weights() {
        // 75% objectivos (4.0) + 25% competências (2.0) = 3.5
        let score = siadap3_weighted_score(4.0, 2.0, 0.75);
        assert!((score - 3.5).abs() < 0.001);
        assert_eq!(
            Siadap3Rating::from_score(score),
            Siadap3Rating::BomDesempenho
        );
    }

    #[test]
    fn intermediary_window_contains() {
        use chrono::NaiveDate;
        let w = IntermediaryEvaluationWindow::standard(2026);
        assert!(w.contains(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));
        assert!(!w.contains(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()));
        assert!(!w.contains(NaiveDate::from_ymd_opt(2026, 5, 31).unwrap()));
    }

    #[test]
    fn siadap2_combined_quota_exceeded() {
        let results = vec![
            Siadap2EvaluationResult {
                entity_id: "1".into(),
                rating: Siadap2Rating::Excelente,
                final_score: 4.0,
            },
            Siadap2EvaluationResult {
                entity_id: "2".into(),
                rating: Siadap2Rating::MuitoBom,
                final_score: 3.0,
            },
            Siadap2EvaluationResult {
                entity_id: "3".into(),
                rating: Siadap2Rating::Adequado,
                final_score: 2.0,
            },
            // 2/3 = 66% combined > 25%
        ];
        let report = validate_siadap2_quotas(&results, &Siadap2QuotaConfig::default());
        assert!(!report.valid);
    }
}

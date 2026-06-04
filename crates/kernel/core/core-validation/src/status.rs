use serde::{Deserialize, Serialize};

/// Resultado de execução de uma validação.
///
/// Distingue entre falha de regra (`Failed`), impossibilidade de executar a
/// validação (`ExecutionError`), e estados funcionais como `Skipped`,
/// `NotApplicable` e `Overridden`. Esta distinção é relevante para evidência
/// COSO: cada estado produz uma leitura de controlo diferente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// A regra correu e o artefacto satisfaz os critérios.
    Passed,
    /// A regra correu e o artefacto não satisfaz os critérios (equivale a
    /// severidade `Error` / blocking).
    Failed,
    /// A regra correu e o artefacto satisfaz os critérios com reservas
    /// (equivale a severidade `Warning`).
    Warning,
    /// A regra foi intencionalmente ignorada por condição legítima.
    Skipped,
    /// A regra não se aplica ao artefacto em questão.
    NotApplicable,
    /// Uma validação bloqueante foi ultrapassada com justificação registada.
    Overridden,
    /// A validação não pôde ser executada (erro de infra-estrutura ou
    /// dependência em falta).
    ExecutionError,
}

impl ValidationStatus {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Failed)
    }

    pub fn allows_progression(&self) -> bool {
        !matches!(self, Self::Failed | Self::ExecutionError)
    }
}

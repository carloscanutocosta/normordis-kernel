use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AuditError;
use crate::policy::{DEFAULT_MAX_CONTROL_ID_CHARS, DEFAULT_MAX_CONTROL_NOTES_CHARS};

/// Resultado de uma execução de controlo.
///
/// Responde à pergunta central do COSO Monitoring Activities:
/// *"O controlo foi executado e com que resultado?"*
///
/// O valor [`Dispensed`] é fundamental para a governação — permite registar
/// formalmente que um controlo aplicável foi dispensado, o que é radicalmente
/// diferente de simplesmente não ter sido executado.
///
/// [`Dispensed`]: ControlExecutionResult::Dispensed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlExecutionResult {
    /// O controlo foi verificado e passou com sucesso.
    ///
    /// A condição que o controlo guarda foi satisfeita. Contribui positivamente
    /// para o indicador de conformidade no Balanced Scorecard.
    Passed,

    /// O controlo foi verificado mas falhou.
    ///
    /// A condição que o controlo guarda não foi satisfeita. Requer análise
    /// de causa raiz e registo de resposta. Contribui negativamente para o
    /// indicador de conformidade e pode disparar alertas de monitorização.
    Failed,

    /// O controlo era aplicável mas foi formalmente dispensado.
    ///
    /// Diferente de "não executado": existe uma decisão explícita e registada
    /// de não aplicar o controlo nesta ocorrência. O campo `notes` em
    /// [`ControlExecution`] deve conter a justificação da dispensa.
    ///
    /// É contabilizado separadamente nos dashboards de conformidade — a taxa
    /// de dispensa é, ela própria, um indicador de risco a monitorizar.
    ///
    /// [`ControlExecution`]: ControlExecution
    Dispensed,
}

/// Registo de execução de um controlo sobre um evento de auditoria.
///
/// # Enquadramento COSO
///
/// O `ControlExecution` materializa a ligação entre o **Registo de Controlos**
/// ([`ControlDefinition`]) e a **evidência** ([`AuditEvent`]):
///
/// ```text
/// ControlDefinition (catálogo)
///        ↓
/// ControlExecution (registo de execução)
///        ↓
/// AuditEvent (evidência verificável na cadeia de hashes)
///        ↓
/// Hash / Assinatura
/// ```
///
/// Esta cadeia permite responder, de forma auditável e verificável, às perguntas
/// COSO: *"O controlo foi executado?"*, *"Com que resultado?"* e *"Qual é a prova?"*
///
/// # Relação many-to-many
///
/// Um único [`AuditEvent`] pode referenciar múltiplos controlos — um evento de
/// emissão de um documento pode verificar `CTRL-AUTH-004`, `CTRL-TRACE-001` e
/// `CTRL-INT-001` em simultâneo. Cada verificação gera um `ControlExecution`
/// separado, todos ligados ao mesmo `event_id`.
///
/// O campo [`AuditEvent::control_id`] identifica o **controlo primário** — o
/// controlo mais directamente relacionado com a acção auditada. Os controlos
/// secundários são registados exclusivamente através de `ControlExecution`.
///
/// # Imutabilidade
///
/// As execuções são append-only. Não é possível alterar o resultado de uma
/// execução — apenas registar novas execuções ou anotar na cadeia de auditoria.
///
/// [`ControlDefinition`]: crate::control_definition::ControlDefinition
/// [`AuditEvent`]: crate::event::AuditEvent
/// [`AuditEvent::control_id`]: crate::event::AuditEvent::control_id
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlExecution {
    /// Identificador único da execução. Gerado como UUID v4.
    pub execution_id: String,

    /// Controlo que foi verificado. Chave estrangeira para [`ControlDefinition`].
    ///
    /// [`ControlDefinition`]: crate::control_definition::ControlDefinition
    pub control_id: String,

    /// Evento de auditoria que originou esta verificação. Chave estrangeira
    /// para [`AuditEvent`].
    ///
    /// [`AuditEvent`]: crate::event::AuditEvent
    pub event_id: String,

    /// Resultado da verificação do controlo.
    pub result: ControlExecutionResult,

    /// Instante em que a verificação foi registada, em UTC.
    pub executed_at_utc: DateTime<Utc>,

    /// Referência externa à evidência que suporta esta execução.
    ///
    /// Pode ser um hash de documento, um URI de ficheiro, um ID de registo
    /// externo, ou qualquer referência verificável fora do sistema de auditoria.
    /// Complementa a evidência implícita do `event_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,

    /// Notas adicionais sobre a execução.
    ///
    /// Obrigatório quando o resultado é [`Dispensed`] — deve conter a
    /// justificação formal da dispensa. Facultativo nos outros casos.
    ///
    /// [`Dispensed`]: ControlExecutionResult::Dispensed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ControlExecution {
    /// Cria um novo registo de execução com UUID v4 e timestamp `Utc::now()`.
    ///
    /// # Erros
    ///
    /// Devolve [`AuditError`] se qualquer campo violar as políticas de validação.
    pub fn new(
        control_id: impl Into<String>,
        event_id: impl Into<String>,
        result: ControlExecutionResult,
        evidence_ref: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, AuditError> {
        Self::with_id_and_time(
            Uuid::new_v4().to_string(),
            control_id,
            event_id,
            Utc::now(),
            result,
            evidence_ref,
            notes,
        )
    }

    /// Cria um registo de execução com `execution_id` e `executed_at_utc` explícitos.
    ///
    /// Destina-se a cenários de teste e migração. Em produção, use [`new`].
    ///
    /// [`new`]: ControlExecution::new
    #[allow(clippy::too_many_arguments)]
    pub fn with_id_and_time(
        execution_id: impl Into<String>,
        control_id: impl Into<String>,
        event_id: impl Into<String>,
        executed_at_utc: DateTime<Utc>,
        result: ControlExecutionResult,
        evidence_ref: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, AuditError> {
        let execution = Self {
            execution_id: execution_id.into(),
            control_id: control_id.into(),
            event_id: event_id.into(),
            result,
            executed_at_utc,
            evidence_ref,
            notes,
        };
        execution.validate()?;
        Ok(execution)
    }

    /// Valida os campos da execução.
    pub fn validate(&self) -> Result<(), AuditError> {
        if self.execution_id.trim().is_empty() || self.execution_id != self.execution_id.trim() {
            return Err(AuditError::OperationFailed);
        }
        if self.control_id.trim().is_empty()
            || self.control_id != self.control_id.trim()
            || self.control_id.chars().count() > DEFAULT_MAX_CONTROL_ID_CHARS
        {
            return Err(AuditError::InvalidControlId);
        }
        if self.event_id.trim().is_empty() || self.event_id != self.event_id.trim() {
            return Err(AuditError::OperationFailed);
        }
        if self.result == ControlExecutionResult::Dispensed && self.notes.is_none() {
            return Err(AuditError::InvalidControlExecution);
        }
        if let Some(notes) = &self.notes {
            if notes.chars().count() > DEFAULT_MAX_CONTROL_NOTES_CHARS {
                return Err(AuditError::InvalidControlExecution);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn passed_execution() -> ControlExecution {
        ControlExecution::with_id_and_time(
            "exec-1",
            "CTRL-AUTH-001",
            "event-1",
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            ControlExecutionResult::Passed,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn creates_valid_passed_execution() {
        let exec = passed_execution();
        assert_eq!(exec.control_id, "CTRL-AUTH-001");
        assert_eq!(exec.result, ControlExecutionResult::Passed);
    }

    #[test]
    fn dispensed_requires_notes() {
        let err = ControlExecution::with_id_and_time(
            "exec-2",
            "CTRL-VAL-003",
            "event-1",
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            ControlExecutionResult::Dispensed,
            None,
            None, // notes ausentes — inválido
        )
        .unwrap_err();

        assert_eq!(err, AuditError::InvalidControlExecution);
    }

    #[test]
    fn dispensed_with_notes_is_valid() {
        let exec = ControlExecution::with_id_and_time(
            "exec-3",
            "CTRL-VAL-003",
            "event-1",
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            ControlExecutionResult::Dispensed,
            None,
            Some("Processo simplificado aprovado por deliberação #2026-042.".to_string()),
        )
        .unwrap();

        assert_eq!(exec.result, ControlExecutionResult::Dispensed);
        assert!(exec.notes.is_some());
    }

    #[test]
    fn failed_execution_without_notes_is_valid() {
        let exec = ControlExecution::with_id_and_time(
            "exec-4",
            "CTRL-AUTH-001",
            "event-1",
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            ControlExecutionResult::Failed,
            None,
            None,
        )
        .unwrap();

        assert_eq!(exec.result, ControlExecutionResult::Failed);
    }

    #[test]
    fn new_generates_uuid() {
        let exec = ControlExecution::new(
            "CTRL-TRACE-001",
            "event-abc",
            ControlExecutionResult::Passed,
            None,
            None,
        )
        .unwrap();

        uuid::Uuid::parse_str(&exec.execution_id).unwrap();
    }
}

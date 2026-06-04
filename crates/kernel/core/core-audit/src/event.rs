use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::actor::AuditActor;
use crate::error::AuditError;
use crate::outcome::AuditOutcome;
use crate::policy::validate_event_policy;
use crate::target::AuditTarget;

/// Registo imutável de um acontecimento relevante para auditoria institucional.
///
/// # Enquadramento COSO
///
/// O `AuditEvent` é a unidade atómica de evidência do sistema de auditoria.
/// Responde directamente às perguntas fundamentais do framework COSO:
///
/// | Pergunta COSO            | Campo              |
/// |--------------------------|--------------------|
/// | Quem actuou?             | `actor`            |
/// | Sobre quê?               | `target`           |
/// | Que acção?               | `event_type`       |
/// | Quando?                  | `occurred_at_utc`  |
/// | Com que resultado?       | `outcome`          |
/// | Que controlo exerceu?    | `control_id`       |
/// | Qual é a evidência?      | hash na cadeia     |
///
/// # Imutabilidade e integridade
///
/// Uma vez gravado, o evento integra a cadeia de hashes do `AuditStore`. O
/// `record_hash` de cada evento depende do seu conteúdo **e** do `record_hash`
/// do evento anterior — qualquer adulteração posterior é detectada por
/// `verify_chain`. O evento em si não armazena o hash; este é calculado e
/// verificado pela camada de persistência.
///
/// # Compatibilidade de serialização
///
/// Os campos `outcome` e `control_id` são omitidos da serialização JSON quando
/// têm os seus valores por omissão (`NotApplicable` e `None`, respectivamente).
/// Isto garante que registos criados antes da introdução destes campos continuam
/// a verificar correctamente na cadeia de hashes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Identificador único do evento. Gerado automaticamente como UUID v4 pelo
    /// construtor [`new`]; pode ser explicitado via [`with_id_and_time`] para
    /// cenários de teste ou migração.
    ///
    /// [`new`]: AuditEvent::new
    /// [`with_id_and_time`]: AuditEvent::with_id_and_time
    pub event_id: String,

    /// Classificação semântica do acontecimento auditado.
    ///
    /// Segue convenção `domínio.entidade.acção` (ex.: `document.classification.changed`,
    /// `user.session.started`, `delegation.approval.granted`). Máximo de
    /// [`DEFAULT_MAX_EVENT_TYPE_CHARS`] caracteres.
    ///
    /// [`DEFAULT_MAX_EVENT_TYPE_CHARS`]: crate::policy::DEFAULT_MAX_EVENT_TYPE_CHARS
    pub event_type: String,

    /// Identidade de quem realizou a acção (pessoa, sistema, ou processo).
    pub actor: AuditActor,

    /// Entidade sobre a qual a acção incidiu.
    pub target: AuditTarget,

    /// Instante em que o acontecimento ocorreu, em UTC.
    ///
    /// Gerado automaticamente pelo construtor [`new`] via `Utc::now()`.
    /// Para garantir rastreabilidade, deve reflectir o momento real da operação
    /// e não o momento de gravação do evento.
    pub occurred_at_utc: DateTime<Utc>,

    /// Resultado observável da operação auditada.
    ///
    /// Responde à questão COSO: *"O controlo foi executado com sucesso?"*
    /// Omitido da serialização quando é [`NotApplicable`] para compatibilidade
    /// retroactiva com registos anteriores.
    ///
    /// [`NotApplicable`]: crate::outcome::AuditOutcome::NotApplicable
    #[serde(default, skip_serializing_if = "AuditOutcome::is_not_applicable")]
    pub outcome: AuditOutcome,

    /// Referência ao identificador do controlo COSO que este evento documenta.
    ///
    /// Chave estrangeira para o Registo de Controlos (Control Registry) externo
    /// ao kernel — tipicamente gerido por `domain-governance` ou equivalente.
    /// `None` indica que o evento não está associado a um controlo específico.
    ///
    /// Máximo de [`DEFAULT_MAX_CONTROL_ID_CHARS`] caracteres. Não pode ser uma
    /// string vazia ou com espaços nas extremidades.
    ///
    /// Omitido da serialização quando `None` para compatibilidade retroactiva.
    ///
    /// [`DEFAULT_MAX_CONTROL_ID_CHARS`]: crate::policy::DEFAULT_MAX_CONTROL_ID_CHARS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_id: Option<String>,

    /// Contexto adicional da operação, em formato JSON livre.
    ///
    /// Não deve conter dados sensíveis (passwords, tokens, chaves criptográficas,
    /// etc.); a política de validação rejeita eventos com tais campos.
    /// Máximo de [`DEFAULT_MAX_DETAILS_BYTES`] bytes serializados.
    ///
    /// [`DEFAULT_MAX_DETAILS_BYTES`]: crate::policy::DEFAULT_MAX_DETAILS_BYTES
    pub details_json: Option<Value>,
}

impl AuditEvent {
    /// Cria um novo evento com UUID v4 gerado automaticamente e timestamp `Utc::now()`.
    ///
    /// # Erros
    ///
    /// Devolve [`AuditError`] se qualquer campo violar as políticas de validação.
    /// Ver [`validate`] para o detalhe das regras.
    ///
    /// [`validate`]: AuditEvent::validate
    pub fn new(
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        outcome: AuditOutcome,
        control_id: Option<String>,
        details_json: Option<Value>,
    ) -> Result<Self, AuditError> {
        Self::with_id_and_time(
            Uuid::new_v4().to_string(),
            event_type,
            actor,
            target,
            Utc::now(),
            outcome,
            control_id,
            details_json,
        )
    }

    /// Cria um evento com `event_id` e `occurred_at_utc` explícitos.
    ///
    /// Destina-se a cenários de teste e de migração de dados onde é necessário
    /// controlar o identificador e o timestamp do evento. Em produção, use [`new`].
    ///
    /// [`new`]: AuditEvent::new
    #[allow(clippy::too_many_arguments)]
    pub fn with_id_and_time(
        event_id: impl Into<String>,
        event_type: impl Into<String>,
        actor: AuditActor,
        target: AuditTarget,
        occurred_at_utc: DateTime<Utc>,
        outcome: AuditOutcome,
        control_id: Option<String>,
        details_json: Option<Value>,
    ) -> Result<Self, AuditError> {
        let event = Self {
            event_id: event_id.into(),
            event_type: event_type.into(),
            actor,
            target,
            occurred_at_utc,
            outcome,
            control_id,
            details_json,
        };
        event.validate()?;
        Ok(event)
    }

    /// Valida todos os campos do evento segundo as políticas de `core-audit`.
    ///
    /// # Regras aplicadas
    ///
    /// - `event_id`: não vazio, sem espaços nas extremidades
    /// - `event_type`: não vazio, sem espaços nas extremidades, máx. [`DEFAULT_MAX_EVENT_TYPE_CHARS`] chars
    /// - `actor`: ver [`AuditActor::validate`]
    /// - `target`: ver [`AuditTarget::validate`]
    /// - `control_id`: se presente, não vazio, sem espaços nas extremidades, máx. [`DEFAULT_MAX_CONTROL_ID_CHARS`] chars
    /// - `details_json`: se presente, máx. [`DEFAULT_MAX_DETAILS_BYTES`] bytes, sem chaves sensíveis
    ///
    /// [`DEFAULT_MAX_EVENT_TYPE_CHARS`]: crate::policy::DEFAULT_MAX_EVENT_TYPE_CHARS
    /// [`DEFAULT_MAX_CONTROL_ID_CHARS`]: crate::policy::DEFAULT_MAX_CONTROL_ID_CHARS
    /// [`DEFAULT_MAX_DETAILS_BYTES`]: crate::policy::DEFAULT_MAX_DETAILS_BYTES
    pub fn validate(&self) -> Result<(), AuditError> {
        if self.event_id.trim().is_empty() || self.event_id != self.event_id.trim() {
            return Err(AuditError::OperationFailed);
        }
        if self.event_type.trim().is_empty() || self.event_type != self.event_type.trim() {
            return Err(AuditError::InvalidEventType);
        }
        self.actor.validate()?;
        self.target.validate()?;
        validate_event_policy(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn base_event() -> Result<AuditEvent, AuditError> {
        AuditEvent::with_id_and_time(
            "event-1",
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            None,
            None,
        )
    }

    #[test]
    fn creates_valid_event() {
        let event = base_event().unwrap();

        assert_eq!(event.event_id, "event-1");
        assert_eq!(event.event_type, "document.created");
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.control_id, None);
    }

    #[test]
    fn creates_event_with_control_id() {
        let event = AuditEvent::with_id_and_time(
            "event-2",
            "document.approved",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            Some("CTRL-014".to_string()),
            None,
        )
        .unwrap();

        assert_eq!(event.control_id, Some("CTRL-014".to_string()));
    }

    #[test]
    fn rejects_empty_event_type() {
        let err = AuditEvent::new(
            "",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err, AuditError::InvalidEventType);
    }

    #[test]
    fn rejects_empty_control_id() {
        let err = AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::Success,
            Some("".to_string()),
            None,
        )
        .unwrap_err();

        assert_eq!(err, AuditError::InvalidControlId);
    }

    #[test]
    fn rejects_control_id_with_leading_whitespace() {
        let err = AuditEvent::new(
            "document.created",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            AuditOutcome::Success,
            Some(" CTRL-001".to_string()),
            None,
        )
        .unwrap_err();

        assert_eq!(err, AuditError::InvalidControlId);
    }

    #[test]
    fn outcome_not_applicable_omitted_in_serialization() {
        let event = AuditEvent::with_id_and_time(
            "event-1",
            "system.started",
            AuditActor::new("system").unwrap(),
            AuditTarget::new("service", "kernel").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::NotApplicable,
            None,
            None,
        )
        .unwrap();

        let value = serde_json::to_value(&event).unwrap();
        assert!(!value.as_object().unwrap().contains_key("outcome"));
        assert!(!value.as_object().unwrap().contains_key("control_id"));
    }

    #[test]
    fn outcome_success_present_in_serialization() {
        let event = base_event().unwrap();
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["outcome"], serde_json::json!("success"));
    }

    #[test]
    fn control_id_present_in_serialization_when_some() {
        let event = AuditEvent::with_id_and_time(
            "event-1",
            "document.approved",
            AuditActor::new("user-1").unwrap(),
            AuditTarget::new("document", "doc-1").unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap(),
            AuditOutcome::Success,
            Some("CTRL-014".to_string()),
            None,
        )
        .unwrap();

        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["control_id"], serde_json::json!("CTRL-014"));
    }
}

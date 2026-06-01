use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::control_category::{ControlCategory, ControlSeverity};
use crate::error::AuditError;
use crate::policy::{
    DEFAULT_MAX_CONTROL_ID_CHARS, DEFAULT_MAX_CONTROL_NAME_CHARS,
    DEFAULT_MAX_CONTROL_REFERENCE_CHARS,
};

/// Definição de um controlo no Registo de Controlos NORMORDIS.
///
/// # Enquadramento COSO
///
/// O `ControlDefinition` é o **catálogo institucional dos controlos** que a
/// organização considera relevantes. Não é um motor de regras nem um workflow —
/// é o registo explícito que torna audível a resposta à pergunta:
///
/// > *"Que controlos estão definidos e quem é responsável pela sua implementação?"*
///
/// A ligação entre um controlo definido e a sua execução é estabelecida
/// por [`ControlExecution`] — que regista, para cada evento de auditoria,
/// quais os controlos que foram verificados e com que resultado.
///
/// # Estrutura do identificador
///
/// O `control_id` segue a convenção `CTRL-{CATEGORIA}-{NNN}`, por exemplo:
/// - `CTRL-AUTH-001` — Autenticação válida
/// - `CTRL-TRACE-001` — Evento auditável registado
/// - `CTRL-PRIV-003` — Minimização aplicada
///
/// # Versionamento
///
/// O campo `version` (semver) e o par `valid_from`/`valid_to` permitem
/// gerir a evolução dos controlos ao longo do tempo. Uma nova versão de um
/// controlo coexiste com versões anteriores através de `valid_to` nas versões
/// antigas. O campo `active` marca se o controlo está operacional.
///
/// [`ControlExecution`]: crate::control_execution::ControlExecution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlDefinition {
    /// Identificador único e canónico do controlo.
    ///
    /// Segue a convenção `CTRL-{CATEGORIA}-{NNN}`. É a chave estrangeira
    /// referenciada em [`AuditEvent::control_id`] e [`ControlExecution::control_id`].
    ///
    /// [`AuditEvent::control_id`]: crate::event::AuditEvent::control_id
    /// [`ControlExecution::control_id`]: crate::control_execution::ControlExecution::control_id
    pub control_id: String,

    /// Nome curto e descritivo do controlo (ex.: "Autenticação válida").
    pub name: String,

    /// Descrição detalhada do propósito e âmbito do controlo.
    ///
    /// Deve explicar o que o controlo verifica, não como o implementa.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Categoria funcional do controlo.
    ///
    /// Determina o prefixo canónico do `control_id` e agrupa controlos
    /// para efeitos de monitorização e reporting.
    pub category: ControlCategory,

    /// Nível de severidade — impacto potencial de uma falha deste controlo.
    pub severity: ControlSeverity,

    /// Proprietário responsável pelo controlo (pessoa, função ou unidade orgânica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Componentes do NORMORDIS responsáveis pela implementação técnica.
    ///
    /// Referências no formato `@core-audit`, `@core-security`, `domain-service`, etc.
    /// Um controlo pode ser implementado por múltiplos componentes.
    #[serde(default)]
    pub implemented_by: Vec<String>,

    /// Frameworks e normas que este controlo endereça.
    ///
    /// Exemplos: `"COSO"`, `"ISO 27001"`, `"ISO 15489"`, `"RGPD"`, `"eIDAS"`.
    #[serde(default)]
    pub references: Vec<String>,

    /// Versão do controlo em formato semver (ex.: `"1.0.0"`).
    pub version: String,

    /// Data de entrada em vigor deste controlo, em UTC.
    pub valid_from: DateTime<Utc>,

    /// Data de fim de vigência, em UTC. `None` indica vigência indefinida.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<DateTime<Utc>>,

    /// Indica se o controlo está operacional.
    ///
    /// Controlos inactivos permanecem no registo para fins históricos e de
    /// auditoria, mas não devem ser utilizados em novas [`ControlExecution`].
    ///
    /// [`ControlExecution`]: crate::control_execution::ControlExecution
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}

impl ControlDefinition {
    /// Valida os campos da definição segundo as políticas de `core-audit`.
    ///
    /// # Regras aplicadas
    ///
    /// - `control_id`: não vazio, sem espaços nas extremidades, máx. [`DEFAULT_MAX_CONTROL_ID_CHARS`]
    /// - `name`: não vazio, sem espaços nas extremidades, máx. [`DEFAULT_MAX_CONTROL_NAME_CHARS`]
    /// - `version`: não vazio
    /// - `valid_to`: se presente, deve ser posterior a `valid_from`
    /// - `implemented_by`: cada entrada não vazia, máx. [`DEFAULT_MAX_CONTROL_REFERENCE_CHARS`]
    /// - `references`: cada entrada não vazia, máx. [`DEFAULT_MAX_CONTROL_REFERENCE_CHARS`]
    pub fn validate(&self) -> Result<(), AuditError> {
        validate_id_field(&self.control_id)?;
        validate_name_field(&self.name)?;
        if self.version.trim().is_empty() {
            return Err(AuditError::InvalidControlDefinition);
        }
        if let (Some(valid_to), valid_from) = (self.valid_to, self.valid_from) {
            if valid_to <= valid_from {
                return Err(AuditError::InvalidControlDefinition);
            }
        }
        for entry in &self.implemented_by {
            validate_reference_field(entry)?;
        }
        for entry in &self.references {
            validate_reference_field(entry)?;
        }
        Ok(())
    }

    /// Devolve `true` se o controlo está vigente no instante `at`.
    ///
    /// Um controlo está vigente se `valid_from <= at` e
    /// `valid_to.is_none() || valid_to > at`.
    pub fn is_valid_at(&self, at: DateTime<Utc>) -> bool {
        self.valid_from <= at && self.valid_to.map_or(true, |t| t > at)
    }
}

fn validate_id_field(value: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() || value != value.trim() {
        return Err(AuditError::InvalidControlDefinition);
    }
    if value.chars().count() > DEFAULT_MAX_CONTROL_ID_CHARS {
        return Err(AuditError::InvalidControlDefinition);
    }
    Ok(())
}

fn validate_name_field(value: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() || value != value.trim() {
        return Err(AuditError::InvalidControlDefinition);
    }
    if value.chars().count() > DEFAULT_MAX_CONTROL_NAME_CHARS {
        return Err(AuditError::InvalidControlDefinition);
    }
    Ok(())
}

fn validate_reference_field(value: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() || value != value.trim() {
        return Err(AuditError::InvalidControlDefinition);
    }
    if value.chars().count() > DEFAULT_MAX_CONTROL_REFERENCE_CHARS {
        return Err(AuditError::InvalidControlDefinition);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn valid_definition() -> ControlDefinition {
        ControlDefinition {
            control_id: "CTRL-AUTH-001".to_string(),
            name: "Autenticação válida".to_string(),
            description: Some("Verifica que o utilizador está autenticado.".to_string()),
            category: ControlCategory::Auth,
            severity: ControlSeverity::High,
            owner: None,
            implemented_by: vec!["@core-security".to_string()],
            references: vec!["COSO".to_string(), "ISO 27001".to_string()],
            version: "1.0.0".to_string(),
            valid_from: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            valid_to: None,
            active: true,
        }
    }

    #[test]
    fn validates_correct_definition() {
        assert!(valid_definition().validate().is_ok());
    }

    #[test]
    fn rejects_empty_control_id() {
        let mut def = valid_definition();
        def.control_id = String::new();
        assert_eq!(def.validate().unwrap_err(), AuditError::InvalidControlDefinition);
    }

    #[test]
    fn rejects_empty_name() {
        let mut def = valid_definition();
        def.name = String::new();
        assert_eq!(def.validate().unwrap_err(), AuditError::InvalidControlDefinition);
    }

    #[test]
    fn rejects_empty_version() {
        let mut def = valid_definition();
        def.version = String::new();
        assert_eq!(def.validate().unwrap_err(), AuditError::InvalidControlDefinition);
    }

    #[test]
    fn rejects_valid_to_before_valid_from() {
        let mut def = valid_definition();
        def.valid_to = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        assert_eq!(def.validate().unwrap_err(), AuditError::InvalidControlDefinition);
    }

    #[test]
    fn accepts_valid_to_after_valid_from() {
        let mut def = valid_definition();
        def.valid_to = Some(Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap());
        assert!(def.validate().is_ok());
    }

    #[test]
    fn is_valid_at_checks_period() {
        let def = valid_definition();
        let before = Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap();
        let during = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();

        assert!(!def.is_valid_at(before));
        assert!(def.is_valid_at(during));
    }

    #[test]
    fn is_valid_at_respects_valid_to() {
        let mut def = valid_definition();
        def.valid_to = Some(Utc.with_ymd_and_hms(2026, 12, 31, 0, 0, 0).unwrap());

        let during = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();

        assert!(def.is_valid_at(during));
        assert!(!def.is_valid_at(after));
    }
}

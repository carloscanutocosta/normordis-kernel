use serde::{Deserialize, Serialize};

/// Contexto de execução de uma validação institucional.
///
/// Identifica quem validou, quando, em que âmbito e com que motor. Permite
/// que o `ValidationResult` seja reprodutível e auditável — condição exigida
/// pelo alinhamento COSO de informação e monitorização.
///
/// O timestamp deve ser uma string RFC 3339 (e.g. `"2026-06-03T14:00:00Z"`).
/// O chamador é responsável por fornecê-la; o core-validation não gera relógios.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationContext {
    /// Identificador do actor que desencadeou a validação (utilizador, serviço).
    pub actor_id: Option<String>,
    /// Âmbito da validação (ex: unidade orgânica, tenant, módulo).
    pub scope: Option<String>,
    /// Momento de execução em RFC 3339.
    pub timestamp_rfc3339: String,
    /// Versão do motor de validação (útil para reprodutibilidade).
    pub engine_version: Option<String>,
}

impl ValidationContext {
    pub fn new(timestamp_rfc3339: impl Into<String>) -> Self {
        Self {
            actor_id: None,
            scope: None,
            timestamp_rfc3339: timestamp_rfc3339.into(),
            engine_version: None,
        }
    }

    pub fn with_actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    pub fn with_engine_version(mut self, version: impl Into<String>) -> Self {
        self.engine_version = Some(version.into());
        self
    }
}

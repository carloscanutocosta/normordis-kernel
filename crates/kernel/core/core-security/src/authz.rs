//! Tipos de autorização contextual — ABAC/ReBAC.
//!
//! `AuthzRequest` e `AuthzDecision` complementam o `WriteInvariantContext` existente
//! com suporte a atributos de recurso (tipo, unidade orgânica, classificação, estado)
//! e contexto de acesso (canal, nível de autenticação exigido).
//!
//! ## Deny-by-default
//!
//! Quando nenhuma política é aplicável, o `code` é `"SEC.AUTHZ.NO_POLICY"` e o
//! `outcome` é `Deny`. Ausência de política nunca significa permissão implícita.
//!
//! ## Decisão explicável
//!
//! `AuthzDecision` sempre inclui `reason` e `code` legíveis — essencial para
//! auditoria, logging estruturado e defesa institucional perante auditores.
//!
//! ## Separação com WriteInvariantContext
//!
//! `WriteInvariantContext` é o contrato zero-trust mínimo para writes críticos.
//! `AuthzRequest` é o contrato rico para autorização contextual ABAC — ambos
//! coexistem; o serviço suporta os dois caminhos.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AuthLevel, OrgScope, ResourceClassification};

/// Atributos do recurso sobre o qual a operação actua.
///
/// Permite decisões baseadas em atributos (ABAC): tipo, unidade orgânica,
/// classificação e estado processual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAttributes {
    /// Tipo canónico do recurso (e.g., "document_instance", "process_file").
    pub resource_type: String,
    /// Unidade orgânica à qual o recurso pertence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_unit: Option<OrgScope>,
    /// Classificação de sensibilidade do recurso.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<ResourceClassification>,
    /// Estado processual/documental (e.g., "draft", "submitted", "approved").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl ResourceAttributes {
    pub fn of_type(resource_type: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            org_unit: None,
            classification: None,
            state: None,
        }
    }

    pub fn with_classification(mut self, c: ResourceClassification) -> Self {
        self.classification = Some(c);
        self
    }

    pub fn with_state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    pub fn with_org_unit(mut self, scope: OrgScope) -> Self {
        self.org_unit = Some(scope);
        self
    }

    /// Verdadeiro se a classificação é Restricted ou Secret — exige evidência reforçada.
    pub fn is_high_classification(&self) -> bool {
        matches!(
            self.classification,
            Some(ResourceClassification::Restricted) | Some(ResourceClassification::Secret)
        )
    }
}

/// Pedido de autorização contextual.
///
/// Engloba acção, recurso e contexto de acesso.
/// Passado a `SecurityService::authorize_contextual()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzRequest {
    /// Acção canónica pedida (e.g., "document.submit_for_decision").
    pub action: String,
    /// Identificador de correlação para rastreabilidade — obrigatório.
    pub correlation_id: String,
    /// Atributos do recurso sobre o qual a acção actua.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceAttributes>,
    /// Canal de acesso (e.g., "workspace", "api", "batch").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Nível de autenticação mínimo exigido por esta operação.
    /// `None` → qualquer nível é aceite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_auth_level: Option<AuthLevel>,
    /// Momento do pedido — usado para validar vigência temporal de delegações.
    pub at: DateTime<Utc>,
}

impl AuthzRequest {
    pub fn new(
        action: impl Into<String>,
        correlation_id: impl Into<String>,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            action: action.into(),
            correlation_id: correlation_id.into(),
            resource: None,
            channel: None,
            required_auth_level: None,
            at,
        }
    }

    pub fn with_resource(mut self, r: ResourceAttributes) -> Self {
        self.resource = Some(r);
        self
    }

    pub fn with_required_auth_level(mut self, level: AuthLevel) -> Self {
        self.required_auth_level = Some(level);
        self
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }
}

/// Resultado de uma decisão de autorização contextual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthzOutcome {
    Allow,
    Deny,
}

/// Nível de evidência associado à decisão de autorização.
///
/// Indica quanta evidência deve ser produzida no `core-audit`.
/// Operações sobre recursos sensíveis ou com autorização por delegação
/// explícita exigem evidência mais elevada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceLevel {
    /// Sem evidência requerida — operações de baixo risco ou bootstrap.
    None,
    /// Evidência normal — registo de ato institucional padrão.
    Normal,
    /// Evidência reforçada — registo detalhado; obrigatório para recursos sensíveis.
    Enhanced,
}

/// Decisão de autorização contextual — sempre explicável.
///
/// Inclui `outcome`, `reason`, `code` e `evidence_level` para suportar
/// auditoria, logging estruturado e defesa institucional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzDecision {
    pub outcome: AuthzOutcome,
    /// ID da política que determinou a decisão, se aplicável.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    /// Razão legível da decisão — para logs e auditoria.
    pub reason: String,
    /// Nível de evidência requerido por esta decisão.
    pub evidence_level: EvidenceLevel,
    /// Código estruturado para identificação programática.
    /// Exemplos: `"SEC.AUTHZ.ALLOW"`, `"SEC.AUTHZ.DENIED"`,
    /// `"SEC.AUTHZ.INSUFFICIENT_AUTH_LEVEL"`, `"SEC.AUTHZ.NO_POLICY"`.
    pub code: String,
}

impl AuthzDecision {
    pub fn allow(
        reason: impl Into<String>,
        policy_id: Option<String>,
        evidence_level: EvidenceLevel,
    ) -> Self {
        Self {
            outcome: AuthzOutcome::Allow,
            policy_id,
            reason: reason.into(),
            evidence_level,
            code: "SEC.AUTHZ.ALLOW".into(),
        }
    }

    pub fn deny(reason: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            outcome: AuthzOutcome::Deny,
            policy_id: None,
            reason: reason.into(),
            evidence_level: EvidenceLevel::Normal,
            code: code.into(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.outcome == AuthzOutcome::Allow
    }

    pub fn is_denied(&self) -> bool {
        self.outcome == AuthzOutcome::Deny
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn allow_decision() {
        let d = AuthzDecision::allow(
            "delegação activa",
            Some("POL-1".into()),
            EvidenceLevel::Normal,
        );
        assert!(d.is_allowed());
        assert!(!d.is_denied());
        assert_eq!(d.code, "SEC.AUTHZ.ALLOW");
        assert_eq!(d.policy_id, Some("POL-1".into()));
    }

    #[test]
    fn deny_decision() {
        let d = AuthzDecision::deny("sem delegação", "SEC.AUTHZ.NO_DELEGATION");
        assert!(d.is_denied());
        assert_eq!(d.code, "SEC.AUTHZ.NO_DELEGATION");
        assert!(d.policy_id.is_none());
    }

    #[test]
    fn resource_attributes_builder() {
        let r = ResourceAttributes::of_type("document_instance")
            .with_classification(ResourceClassification::Restricted)
            .with_state("draft")
            .with_org_unit(OrgScope::new("SF-1234"));
        assert!(r.is_high_classification());
        assert_eq!(r.state.as_deref(), Some("draft"));
    }

    #[test]
    fn resource_secret_is_high_classification() {
        let r = ResourceAttributes::of_type("sensitive")
            .with_classification(ResourceClassification::Secret);
        assert!(r.is_high_classification());
    }

    #[test]
    fn resource_internal_is_not_high_classification() {
        let r = ResourceAttributes::of_type("doc")
            .with_classification(ResourceClassification::Internal);
        assert!(!r.is_high_classification());
    }

    #[test]
    fn authz_request_builder() {
        let req = AuthzRequest::new("doc.sign", "corr-1", Utc::now())
            .with_required_auth_level(AuthLevel::Strong)
            .with_channel("workspace");
        assert_eq!(req.action, "doc.sign");
        assert_eq!(req.required_auth_level, Some(AuthLevel::Strong));
        assert_eq!(req.channel.as_deref(), Some("workspace"));
    }

    #[test]
    fn evidence_level_serde() {
        let encoded = serde_json::to_string(&EvidenceLevel::Enhanced).unwrap();
        assert_eq!(encoded, "\"enhanced\"");
        let decoded: EvidenceLevel = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, EvidenceLevel::Enhanced);
    }
}

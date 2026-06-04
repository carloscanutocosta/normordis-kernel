//! Contexto de segurança normalizado para integração com `core-audit`.
//!
//! `SecurityContext` é o bloco canónico que `core-security` produz e que
//! `core-audit` consome para enriquecer registos de atos institucionais
//! com informação de autorização verificável.
//!
//! ## Separação de responsabilidades
//!
//! ```text
//! core-security  → produz SecurityContext (quem, com que autoridade, em que âmbito)
//! core-audit     → consome SecurityContext e associa ao AuditAct
//! core-rh/org    → fornecem OrgScope e dados de vínculo
//! ```

use serde::{Deserialize, Serialize};

use crate::AuthLevel;

/// Âmbito orgânico de uma operação.
///
/// Tipicamente o `org_unit_id` do `core-org`, representado como string opaca
/// para evitar acoplamento directo entre `core-security` e `core-org`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgScope(pub String);

impl OrgScope {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OrgScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Referência opaca a uma sessão activa.
///
/// Não contém o token nem segredos — apenas o identificador de sessão
/// para correlação e revogação. O token concreto fica em `support-auth`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionRef(pub String);

impl SessionRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Contexto de segurança normalizado.
///
/// Produzido por `core-security` após `authorize()` ou `authorize_contextual()`.
/// Consumido por `core-audit` para enriquecer registos de atos com contexto
/// de autorização verificável — essencial para COSO e defesa institucional.
///
/// ## Campos opcionais
///
/// `session_id`, `policy_id`, `decision_id` e `org_scope` são opcionais porque
/// nem todos os contextos de execução os têm disponíveis (e.g., processos batch
/// sem sessão interactiva).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Identificador do sujeito autenticado (provém do token verificado).
    pub subject_id: String,
    /// Referência à sessão activa, se aplicável.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionRef>,
    /// Nível de autenticação com que o sujeito operou.
    pub auth_level: AuthLevel,
    /// ID da política que gerou a decisão de acesso, se aplicável.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    /// ID único da decisão de autorização para rastreabilidade.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<String>,
    /// Âmbito orgânico em que a operação ocorreu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_scope: Option<OrgScope>,
}

impl SecurityContext {
    /// Contexto mínimo — sujeito + nível de autenticação.
    pub fn minimal(subject_id: impl Into<String>, auth_level: AuthLevel) -> Self {
        Self {
            subject_id: subject_id.into(),
            session_id: None,
            auth_level,
            policy_id: None,
            decision_id: None,
            org_scope: None,
        }
    }

    /// Builder: adiciona sessão.
    pub fn with_session(mut self, session: SessionRef) -> Self {
        self.session_id = Some(session);
        self
    }

    /// Builder: adiciona âmbito orgânico.
    pub fn with_org_scope(mut self, scope: OrgScope) -> Self {
        self.org_scope = Some(scope);
        self
    }

    /// Builder: adiciona policy_id e decision_id após autorização.
    pub fn with_decision(mut self, policy_id: Option<String>, decision_id: Option<String>) -> Self {
        self.policy_id = policy_id;
        self.decision_id = decision_id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_context() {
        let ctx = SecurityContext::minimal("P123", AuthLevel::Normal);
        assert_eq!(ctx.subject_id, "P123");
        assert_eq!(ctx.auth_level, AuthLevel::Normal);
        assert!(ctx.session_id.is_none());
        assert!(ctx.org_scope.is_none());
    }

    #[test]
    fn builder_chain() {
        let ctx = SecurityContext::minimal("P123", AuthLevel::Reinforced)
            .with_session(SessionRef::new("S456"))
            .with_org_scope(OrgScope::new("SF-1234"))
            .with_decision(Some("SEC.DOC.SUBMIT".into()), Some("AUTHZ-789".into()));
        assert_eq!(ctx.session_id.unwrap().as_str(), "S456");
        assert_eq!(ctx.org_scope.unwrap().as_str(), "SF-1234");
        assert_eq!(ctx.policy_id.unwrap(), "SEC.DOC.SUBMIT");
        assert_eq!(ctx.decision_id.unwrap(), "AUTHZ-789");
    }

    #[test]
    fn serde_skip_nones() {
        let ctx = SecurityContext::minimal("P123", AuthLevel::Normal);
        let json = serde_json::to_string(&ctx).unwrap();
        // Campos None não devem aparecer no JSON
        assert!(!json.contains("session_id"));
        assert!(!json.contains("policy_id"));
        assert!(!json.contains("decision_id"));
        assert!(!json.contains("org_scope"));
    }

    #[test]
    fn org_scope_display() {
        let scope = OrgScope::new("SF-1234");
        assert_eq!(scope.to_string(), "SF-1234");
    }

    #[test]
    fn session_ref_as_str() {
        let s = SessionRef::new("sess-abc");
        assert_eq!(s.as_str(), "sess-abc");
    }
}

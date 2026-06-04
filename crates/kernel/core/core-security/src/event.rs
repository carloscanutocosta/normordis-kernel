//! Eventos de segurança estruturados para observabilidade e correlação de incidentes.
//!
//! `SecurityEvent` representa acontecimentos relevantes de segurança para
//! monitorização, SIEM e correlação de incidentes.
//!
//! ## O que registar vs. o que evitar
//!
//! Registar: autenticações, recusas de autorização, revogações, tentativas de escalada.
//! Não registar: passwords, tokens completos, dados pessoais excessivos, segredos.
//!
//! ## Separação com core-audit
//!
//! `SecurityEvent` é um evento técnico de segurança (monitorização/SIEM).
//! `AuditAct` (em core-audit) é um ato institucional permanente (evidência COSO).
//! Os dois complementam-se — não se substituem.

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SecurityError;

/// Tipo de evento de segurança.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventKind {
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationDenied,
    /// Autorização concedida para operação sensível — merece registo explícito.
    AuthorizationAllowedSensitive,
    SessionRevoked,
    MfaRequired,
    PolicyEvaluated,
    /// Tentativa de operar fora do âmbito autorizado.
    PrivilegeEscalationAttempt,
    TokenRefresh,
    KeyRotation,
    /// Violação de segregação de funções detectada.
    SodViolationDetected,
}

/// Evento de segurança estruturado.
///
/// Nunca deve conter: passwords, tokens completos, dados pessoais excessivos, segredos.
/// Alinhado com minimização de dados (RGPD art. 5.º, 1, c)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub kind: SecurityEventKind,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    /// ID do principal envolvido — nunca o token completo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Detalhe adicional — deve evitar dados sensíveis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl SecurityEvent {
    pub fn new(
        kind: SecurityEventKind,
        correlation_id: impl Into<String>,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            kind,
            correlation_id: correlation_id.into(),
            occurred_at,
            principal_id: None,
            operation: None,
            resource: None,
            details: None,
        }
    }

    pub fn with_principal(mut self, id: impl Into<String>) -> Self {
        self.principal_id = Some(id.into());
        self
    }

    pub fn with_operation(mut self, op: impl Into<String>) -> Self {
        self.operation = Some(op.into());
        self
    }

    pub fn with_resource(mut self, r: impl Into<String>) -> Self {
        self.resource = Some(r.into());
        self
    }

    pub fn with_details(mut self, d: impl Into<String>) -> Self {
        self.details = Some(d.into());
        self
    }
}

/// Port de publicação de eventos de segurança.
///
/// Implementações concretas podem publicar para SIEM, log estruturado,
/// fila de mensagens, `core-audit`, etc.
#[allow(async_fn_in_trait)]
pub trait SecurityEventPublisher {
    async fn publish(&self, event: &SecurityEvent) -> Result<(), SecurityError>;
}

/// Implementação nula — descarta todos os eventos sem custo.
pub struct NoopSecurityEventPublisher;

impl SecurityEventPublisher for NoopSecurityEventPublisher {
    async fn publish(&self, _: &SecurityEvent) -> Result<(), SecurityError> {
        Ok(())
    }
}

/// Publisher em memória — acumula eventos para testes de integração.
///
/// Usa `Arc<RwLock<…>>` para poder ser clonado e partilhado
/// entre o código sob teste e as asserções.
#[derive(Clone)]
pub struct InMemorySecurityEventPublisher {
    events: Arc<RwLock<Vec<SecurityEvent>>>,
}

impl InMemorySecurityEventPublisher {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn events(&self) -> Vec<SecurityEvent> {
        self.events.read().unwrap().clone()
    }

    pub fn len(&self) -> usize {
        self.events.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.read().unwrap().is_empty()
    }

    pub fn count_kind(&self, kind: &SecurityEventKind) -> usize {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| &e.kind == kind)
            .count()
    }
}

impl Default for InMemorySecurityEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityEventPublisher for InMemorySecurityEventPublisher {
    async fn publish(&self, event: &SecurityEvent) -> Result<(), SecurityError> {
        self.events.write().unwrap().push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn noop_nao_acumula() {
        let p = NoopSecurityEventPublisher;
        let evt = SecurityEvent::new(SecurityEventKind::AuthorizationDenied, "corr-1", Utc::now());
        p.publish(&evt).await.unwrap();
    }

    #[tokio::test]
    async fn inmemory_acumula_eventos() {
        let p = InMemorySecurityEventPublisher::new();
        let evt = SecurityEvent::new(
            SecurityEventKind::AuthenticationSuccess,
            "corr-1",
            Utc::now(),
        )
        .with_principal("user:alice")
        .with_operation("auth.login");
        p.publish(&evt).await.unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p.events()[0].principal_id.as_deref(), Some("user:alice"));
    }

    #[tokio::test]
    async fn count_kind_filtra_correctamente() {
        let p = InMemorySecurityEventPublisher::new();
        let now = Utc::now();
        p.publish(&SecurityEvent::new(
            SecurityEventKind::AuthorizationDenied,
            "c1",
            now,
        ))
        .await
        .unwrap();
        p.publish(&SecurityEvent::new(
            SecurityEventKind::AuthorizationDenied,
            "c2",
            now,
        ))
        .await
        .unwrap();
        p.publish(&SecurityEvent::new(
            SecurityEventKind::AuthenticationSuccess,
            "c3",
            now,
        ))
        .await
        .unwrap();
        assert_eq!(p.count_kind(&SecurityEventKind::AuthorizationDenied), 2);
        assert_eq!(p.count_kind(&SecurityEventKind::AuthenticationSuccess), 1);
    }

    #[test]
    fn builder_chain() {
        let evt = SecurityEvent::new(
            SecurityEventKind::PrivilegeEscalationAttempt,
            "c",
            Utc::now(),
        )
        .with_principal("user:eve")
        .with_operation("admin.delete_all")
        .with_resource("users")
        .with_details("tentativa bloqueada por deny-by-default");
        assert_eq!(evt.principal_id.as_deref(), Some("user:eve"));
        assert_eq!(evt.resource.as_deref(), Some("users"));
    }

    #[test]
    fn serde_kind() {
        let encoded = serde_json::to_string(&SecurityEventKind::SodViolationDetected).unwrap();
        assert_eq!(encoded, "\"sod_violation_detected\"");
    }
}

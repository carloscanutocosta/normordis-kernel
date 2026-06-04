//! Port de log de decisões de autorização (hexagonal).
//!
//! `SecurityAuditLog` regista cada decisão de `SecurityService::authorize()`,
//! quer seja concessão quer seja recusa, com contexto suficiente para auditoria
//! post-facto e rastreabilidade de acesso.
//!
//! ## Implementações disponíveis
//!
//! - `NoopSecurityAuditLog` — descarta tudo; sem overhead; adequada para testes unitários.
//! - `InMemoryAuditLog` — acumula entradas em `Vec` protegido por `RwLock`; útil em testes
//!   de integração que precisam de inspeccionar as decisões registadas.
//! - `SecuritySqliteStore` (no crate `security-sqlite`) — persiste na tabela
//!   `security_auth_decisions`.

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{authz::EvidenceLevel, SecurityError};

/// Resultado de uma decisão de autorização.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditDecision {
    Granted,
    Denied,
}

/// Entrada imutável de auditoria para uma decisão de `authorize()`.
///
/// `evidence_level` indica quanta evidência o `core-audit` deve produzir:
/// - `None` → concessão por bootstrap/baseline, sem risco — não é necessária evidência.
/// - `Normal` → delegação activa, registo institucional padrão.
/// - `Enhanced` → recurso Restricted/Secret ou operação de alta sensibilidade —
///   evidência detalhada e notificação obrigatórias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuthDecision {
    pub logged_at: DateTime<Utc>,
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub correlation_id: String,
    pub decision: AuditDecision,
    /// Razão da concessão: `"delegation:<id>"` | `"baseline"` | `"exempted"` | `"bootstrap"`.
    /// `None` quando `decision = Denied`.
    pub granted_by_kind: Option<String>,
    /// Motivo da recusa. `None` quando `decision = Granted`.
    pub deny_reason: Option<String>,
    /// Nível de evidência que o `core-audit` deve produzir para este ato.
    pub evidence_level: EvidenceLevel,
}

/// Port de persistência de decisões de autorização.
///
/// Chamado por `SecurityService::authorize()` após cada decisão.
/// A falha do log não deve impedir a operação — tratar como best-effort
/// ou registar em stderr consoante os requisitos do contexto.
#[allow(async_fn_in_trait)]
pub trait SecurityAuditLog {
    async fn record_decision(&self, entry: &SecurityAuthDecision) -> Result<(), SecurityError>;
}

/// Implementação nula — descarta todas as decisões sem custo.
pub struct NoopSecurityAuditLog;

impl SecurityAuditLog for NoopSecurityAuditLog {
    async fn record_decision(&self, _: &SecurityAuthDecision) -> Result<(), SecurityError> {
        Ok(())
    }
}

/// Log de auditoria em memória — útil em testes de integração.
///
/// Internamente usa `Arc<RwLock<…>>` pelo que pode ser clonado e partilhado
/// entre o serviço e o código de teste sem perder o acesso às entradas.
#[derive(Clone)]
pub struct InMemoryAuditLog {
    entries: Arc<RwLock<Vec<SecurityAuthDecision>>>,
}

impl InMemoryAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn entries(&self) -> Vec<SecurityAuthDecision> {
        self.entries.read().unwrap().clone()
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }
}

impl Default for InMemoryAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityAuditLog for InMemoryAuditLog {
    async fn record_decision(&self, entry: &SecurityAuthDecision) -> Result<(), SecurityError> {
        self.entries.write().unwrap().push(entry.clone());
        Ok(())
    }
}

//! Ports de integração de terceiros para `core-security`.
//!
//! Estes contratos permitem que `core-security` consuma dados de `core-org`,
//! `core-rh` e `core-audit` sem dependência directa nessas crates.
//!
//! ## Padrão hexagonal
//!
//! Os ports definem o *que* core-security precisa; a implementação concreta
//! fica em bridges/adapters (e.g., `rh-security-bridge`, `audit-security-bridge`).
//!
//! ## Implementações nulas
//!
//! Cada port tem uma implementação nula (`Noop*`) que permite usar o
//! `SecurityService` sem as integrações, útil em desenvolvimento e testes unitários.
//!
//! | Noop | Comportamento padrão |
//! |------|----------------------|
//! | `NoopOrgScopeValidator` | Sempre válido (permissivo) |
//! | `NoopSodHistoryProvider` | Sem histórico (nenhuma violação SoD) |

use chrono::{DateTime, Utc};

use crate::{OrgScope, SecurityError};

// ── OrgScopeValidator ─────────────────────────────────────────────────────────

/// Port: verificação de âmbito orgânico do principal.
///
/// Implementado por adaptadores que consultam `core-org` / `core-rh`
/// para validar se o principal tem vínculo activo numa unidade orgânica.
///
/// ## Quando usar
///
/// Chamar antes de autorizar operações que exijam `same_org_scope`:
/// o principal deve pertencer à mesma unidade orgânica que o recurso.
#[allow(async_fn_in_trait)]
pub trait OrgScopeValidator {
    /// Verdadeiro se `principal_id` tem vínculo activo em `org_scope` em `now`.
    async fn is_principal_in_scope(
        &self,
        principal_id: &str,
        org_scope: &OrgScope,
        now: DateTime<Utc>,
    ) -> Result<bool, SecurityError>;
}

/// Implementação nula — sempre considera o principal dentro do âmbito.
///
/// Adequada para bootstrap, testes e cenários sem integração com core-org.
/// Atenção: não valida âmbito real; usar apenas em ambientes controlados.
pub struct NoopOrgScopeValidator;

impl OrgScopeValidator for NoopOrgScopeValidator {
    async fn is_principal_in_scope(
        &self,
        _principal_id: &str,
        _org_scope: &OrgScope,
        _now: DateTime<Utc>,
    ) -> Result<bool, SecurityError> {
        Ok(true)
    }
}

// ── SodHistoryProvider ────────────────────────────────────────────────────────

/// Port: histórico de acções de um principal sobre um recurso.
///
/// Implementado por adaptadores que consultam `core-audit` para obter
/// as acções praticadas pelo principal sobre o recurso especificado.
/// Usado por `check_sod()` para detectar violações de segregação de funções.
///
/// ## Quando usar
///
/// Antes de autorizar uma acção sensível (e.g., `document.approve`),
/// verificar se o principal já praticou acções conflituosas (e.g., `document.create`)
/// sobre o mesmo recurso.
#[allow(async_fn_in_trait)]
pub trait SodHistoryProvider {
    /// Devolve acções já praticadas por `principal_id` sobre `resource_id` até `now`.
    async fn previous_actions(
        &self,
        principal_id: &str,
        resource_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<String>, SecurityError>;
}

/// Implementação nula — sem histórico, sem violações SoD.
///
/// Adequada para contextos sem integração com core-audit.
/// Com esta implementação, `check_sod()` nunca detecta violações.
pub struct NoopSodHistoryProvider;

impl SodHistoryProvider for NoopSodHistoryProvider {
    async fn previous_actions(
        &self,
        _principal_id: &str,
        _resource_id: &str,
        _now: DateTime<Utc>,
    ) -> Result<Vec<String>, SecurityError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn noop_org_sempre_in_scope() {
        let v = NoopOrgScopeValidator;
        let result = v
            .is_principal_in_scope("user:alice", &OrgScope::new("SF-1234"), Utc::now())
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn noop_sod_sem_historico() {
        let p = NoopSodHistoryProvider;
        let actions = p
            .previous_actions("user:alice", "doc:123", Utc::now())
            .await
            .unwrap();
        assert!(actions.is_empty());
    }
}

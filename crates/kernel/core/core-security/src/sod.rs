//! Segregação de funções (Separation of Duties — SoD).
//!
//! Define regras que impedem o mesmo sujeito de praticar acções conflituosas
//! sobre o mesmo recurso: quem cria não pode aprovar; quem submeteu não pode
//! validar; quem administra o sistema não pode alterar evidência auditável.
//!
//! ## Uso
//!
//! ```ignore
//! use core_security::sod::{SodRule, check_sod};
//! use core_security::SecurityError;
//!
//! let rules = vec![SodRule::no_self_approval("SOD.DOC.001")];
//! let previous = vec!["document.create".to_string()];
//!
//! if let Some(v) = check_sod(&rules, "document.approve", &previous) {
//!     // Violação detectada — emitir SecurityEvent::SodViolationDetected
//!     // e negar a operação se override_allowed = false
//! }
//! ```
//!
//! ## Regras com override
//!
//! Algumas regras permitem `override_allowed = true` — casos excepcionais
//! em que a violação pode ser ultrapassada com justificação documentada
//! e autorização superior. O override deve sempre gerar evidência reforçada.

use serde::{Deserialize, Serialize};

/// Regra de segregação de funções.
///
/// Uma regra bloqueia `blocked_action` quando o sujeito já praticou
/// `conflicts_with` sobre o mesmo recurso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SodRule {
    /// Identificador único da regra (e.g., "SOD.DOC.001").
    pub rule_id: String,
    /// Descrição legível — aparece em logs e notificações de violação.
    pub description: String,
    /// Acção que está a ser pedida agora (acção bloqueada).
    pub blocked_action: String,
    /// Acção anterior que conflitua com a pedida.
    pub conflicts_with: String,
    /// Se `true`, o conflito pode ser ultrapassado com justificação
    /// e autorização superior — deve gerar evidência reforçada.
    pub override_allowed: bool,
}

impl SodRule {
    /// Regra padrão: quem criou não pode aprovar o mesmo recurso.
    pub fn no_self_approval(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            description: "Quem cria não pode aprovar o mesmo recurso".into(),
            blocked_action: "document.approve".into(),
            conflicts_with: "document.create".into(),
            override_allowed: false,
        }
    }

    /// Regra padrão: quem submeteu não pode aprovar o mesmo recurso.
    pub fn no_submit_and_approve(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            description: "Quem submeteu não pode aprovar o mesmo recurso".into(),
            blocked_action: "document.approve".into(),
            conflicts_with: "document.submit".into(),
            override_allowed: false,
        }
    }

    /// Regra padrão: administrador de sistema não pode alterar evidência auditável.
    pub fn no_admin_alters_audit(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            description: "Administrador de sistema não pode alterar evidência auditável".into(),
            blocked_action: "audit.alter_evidence".into(),
            conflicts_with: "system.admin".into(),
            override_allowed: false,
        }
    }
}

/// Violação de uma regra de segregação de funções detectada.
#[derive(Debug, Clone)]
pub struct SodViolation {
    pub rule_id: String,
    pub description: String,
    /// Se `true`, o conflito pode ser ultrapassado — mas deve gerar
    /// evidência reforçada e justificação documentada.
    pub override_allowed: bool,
}

impl std::fmt::Display for SodViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SoD [{}]: {} (override_allowed={})",
            self.rule_id, self.description, self.override_allowed
        )
    }
}

/// Verifica se a acção pedida conflitua com acções anteriores do sujeito.
///
/// `previous_actions` deve conter as acções já praticadas pelo mesmo sujeito
/// sobre o mesmo recurso (provêm tipicamente do `core-audit`).
///
/// Devolve a primeira violação encontrada, ou `None` se não há conflito.
/// A ordem das regras determina qual a violação reportada em caso de múltiplos conflitos.
pub fn check_sod(
    rules: &[SodRule],
    action: &str,
    previous_actions: &[String],
) -> Option<SodViolation> {
    for rule in rules {
        if rule.blocked_action == action
            && previous_actions.iter().any(|a| a == &rule.conflicts_with)
        {
            return Some(SodViolation {
                rule_id: rule.rule_id.clone(),
                description: rule.description.clone(),
                override_allowed: rule.override_allowed,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_self_approval_bloqueia_aprovacao_apos_criacao() {
        let rules = vec![SodRule::no_self_approval("SOD-001")];
        let previous = vec!["document.create".to_string()];
        let result = check_sod(&rules, "document.approve", &previous);
        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.rule_id, "SOD-001");
        assert!(!v.override_allowed);
    }

    #[test]
    fn no_submit_and_approve_bloqueia_aprovacao_apos_submissao() {
        let rules = vec![SodRule::no_submit_and_approve("SOD-002")];
        let previous = vec!["document.submit".to_string()];
        let result = check_sod(&rules, "document.approve", &previous);
        assert!(result.is_some());
    }

    #[test]
    fn sem_conflito_retorna_none() {
        let rules = vec![SodRule::no_self_approval("SOD-001")];
        let previous = vec!["document.read".to_string()];
        assert!(check_sod(&rules, "document.approve", &previous).is_none());
    }

    #[test]
    fn accao_diferente_nao_conflitua() {
        let rules = vec![SodRule::no_self_approval("SOD-001")];
        let previous = vec!["document.create".to_string()];
        assert!(check_sod(&rules, "document.read", &previous).is_none());
    }

    #[test]
    fn sem_accoes_anteriores_nao_conflitua() {
        let rules = vec![SodRule::no_self_approval("SOD-001")];
        assert!(check_sod(&rules, "document.approve", &[]).is_none());
    }

    #[test]
    fn sem_regras_nao_conflitua() {
        let previous = vec!["document.create".to_string()];
        assert!(check_sod(&[], "document.approve", &previous).is_none());
    }

    #[test]
    fn override_allowed_registado() {
        let rule = SodRule {
            rule_id: "SOD-OVERRIDE".into(),
            description: "Regra com override".into(),
            blocked_action: "doc.sign".into(),
            conflicts_with: "doc.validate".into(),
            override_allowed: true,
        };
        let previous = vec!["doc.validate".to_string()];
        let result = check_sod(&[rule], "doc.sign", &previous).unwrap();
        assert!(result.override_allowed);
    }

    #[test]
    fn primeira_regra_conflituosa_e_reportada() {
        let rules = vec![
            SodRule {
                rule_id: "SOD-A".into(),
                description: "Primeira".into(),
                blocked_action: "doc.approve".into(),
                conflicts_with: "doc.create".into(),
                override_allowed: false,
            },
            SodRule {
                rule_id: "SOD-B".into(),
                description: "Segunda".into(),
                blocked_action: "doc.approve".into(),
                conflicts_with: "doc.create".into(),
                override_allowed: true,
            },
        ];
        let previous = vec!["doc.create".to_string()];
        let v = check_sod(&rules, "doc.approve", &previous).unwrap();
        assert_eq!(v.rule_id, "SOD-A");
    }

    #[test]
    fn display_inclui_rule_id_e_override() {
        let v = SodViolation {
            rule_id: "SOD-001".into(),
            description: "Quem cria não pode aprovar".into(),
            override_allowed: false,
        };
        let s = v.to_string();
        assert!(s.contains("SOD-001"));
        assert!(s.contains("override_allowed=false"));
    }
}

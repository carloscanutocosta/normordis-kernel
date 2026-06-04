//! Delegações temporárias de permissão e revogação de políticas.
//!
//! ## Condições de delegação (`DelegationCondition`)
//!
//! O campo `conditions: Option<String>` armazena opcionalmente um JSON com
//! restrições adicionais que a operação deve satisfazer para a delegação ser
//! considerada activa. As condições são avaliadas em `matches_delegation_with_attrs()`
//! no motor de autorização.
//!
//! ### Condições suportadas
//!
//! ```json
//! {
//!   "required_state": ["draft", "review"],
//!   "required_classification": ["internal", "restricted"],
//!   "required_org_unit": "SF-1234"
//! }
//! ```
//!
//! Se o campo for `Some` mas o JSON for inválido, a delegação é tratada como
//! inactiva (princípio de deny-by-default: condição malformada = nega).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{OrgScope, ResourceClassification, SecurityError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DelegationId(pub String);

impl DelegationId {
    pub fn new(id: impl Into<String>) -> Result<Self, SecurityError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SecurityError::MissingField("delegation_id".into()));
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationRequest {
    pub policy_id: String,
    pub revoked_by: String,
    pub reason: Option<String>,
}

impl RevocationRequest {
    pub fn validate(&self) -> Result<(), SecurityError> {
        if self.policy_id.trim().is_empty() {
            return Err(SecurityError::MissingField("policy_id".into()));
        }
        if self.revoked_by.trim().is_empty() {
            return Err(SecurityError::MissingField("revoked_by".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub granted_by: String,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    /// Condições adicionais opcionais (JSON, CEL, etc.).
    /// **Reservado para uso futuro — não avaliado pelo motor de autorização.**
    /// Qualquer valor aqui é armazenado e devolvido mas não tem efeito na decisão de acesso.
    pub conditions: Option<String>,
    /// Delegação-pai de que este pedido deriva, para rastreabilidade de cadeia
    /// e revogação em cascata. `None` = delegação raiz (concedida directamente
    /// por um principal do sistema ou em bootstrap).
    ///
    /// Quando criado via `SecurityService::grant_delegation()`, este campo é
    /// preenchido automaticamente pelo serviço com base na delegação do granter.
    pub granted_via: Option<DelegationId>,
}

impl DelegationRequest {
    pub fn validate(&self) -> Result<(), SecurityError> {
        if self.principal.trim().is_empty() {
            return Err(SecurityError::MissingField("principal".into()));
        }
        if self.operation.trim().is_empty() {
            return Err(SecurityError::MissingField("operation".into()));
        }
        if self.granted_by.trim().is_empty() {
            return Err(SecurityError::MissingField("granted_by".into()));
        }
        if self.valid_to <= self.valid_from {
            return Err(SecurityError::InvalidDelegation(
                "valid_to não pode ser anterior ou igual a valid_from".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub delegation_id: DelegationId,
    pub principal: String,
    pub operation: String,
    pub resource: Option<String>,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    /// Condições adicionais opcionais (JSON, CEL, etc.).
    /// **Reservado para uso futuro — não avaliado pelo motor de autorização.**
    pub conditions: Option<String>,
    pub revoked: bool,
    /// Delegação-pai da qual esta deriva. `None` = delegação raiz.
    /// Usado para revogação em cascata: revogar o pai revoga automaticamente
    /// todos os descendentes.
    pub granted_via: Option<DelegationId>,
}

impl Delegation {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        !self.revoked && now >= self.valid_from && now < self.valid_to
    }
}

// ── DelegationCondition ───────────────────────────────────────────────────────

/// Condições opcionais de uma delegação, avaliadas no motor de autorização.
///
/// Serializadas como JSON no campo `conditions: Option<String>` de `Delegation`.
/// Se o JSON não for parseável, a delegação é tratada como inactiva
/// (deny-by-default para condições malformadas).
///
/// ## Campos
///
/// Todos os campos são opcionais — uma `DelegationCondition` com todos `None`
/// não impõe qualquer restrição (equivale a ausência de condição).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DelegationCondition {
    /// O recurso deve estar num destes estados (e.g., `["draft", "review"]`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_state: Option<Vec<String>>,
    /// O recurso deve ter uma destas classificações.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_classification: Option<Vec<ResourceClassification>>,
    /// O recurso deve pertencer a esta unidade orgânica.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_org_unit: Option<OrgScope>,
}

impl DelegationCondition {
    /// Parse a partir do JSON armazenado em `Delegation.conditions`.
    /// Devolve `None` se a string for inválida — o caller deve tratar como deny.
    pub fn parse(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }

    /// Serializa para JSON para armazenar em `DelegationRequest.conditions`.
    pub fn to_json(&self) -> Result<String, SecurityError> {
        serde_json::to_string(self)
            .map_err(|e| SecurityError::OperationFailed(format!("conditions serialization: {e}")))
    }

    /// Avalia a condição contra os atributos do recurso pedido.
    ///
    /// - Se não há restrições (`all None`) → sempre satisfeita.
    /// - Se há restrições mas `attrs` é `None` → nega (condição não verificável).
    /// - Se `attrs` presente → verifica estado, classificação e âmbito orgânico.
    pub fn evaluate(&self, attrs: Option<&crate::ResourceAttributes>) -> bool {
        let has_requirement = self.required_state.is_some()
            || self.required_classification.is_some()
            || self.required_org_unit.is_some();

        if !has_requirement {
            return true;
        }

        let Some(attrs) = attrs else {
            return false; // condições exigem atributos, mas não foram fornecidos
        };

        if let Some(states) = &self.required_state {
            match &attrs.state {
                None => return false,
                Some(s) if !states.contains(s) => return false,
                _ => {}
            }
        }

        if let Some(classes) = &self.required_classification {
            match &attrs.classification {
                None => return false,
                Some(c) if !classes.contains(c) => return false,
                _ => {}
            }
        }

        if let Some(required_ou) = &self.required_org_unit {
            match &attrs.org_unit {
                None => return false,
                Some(ou) if ou != required_ou => return false,
                _ => {}
            }
        }

        true
    }
}

#[cfg(test)]
mod condition_tests {
    use super::*;
    use crate::{OrgScope, ResourceAttributes, ResourceClassification};

    fn attrs(state: &str) -> ResourceAttributes {
        crate::ResourceAttributes::of_type("doc").with_state(state)
    }

    #[test]
    fn sem_restricoes_sempre_passa() {
        let cond = DelegationCondition::default();
        assert!(cond.evaluate(None));
        assert!(cond.evaluate(Some(&attrs("any"))));
    }

    #[test]
    fn required_state_satisfeito() {
        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into(), "review".into()]),
            ..Default::default()
        };
        assert!(cond.evaluate(Some(&attrs("draft"))));
        assert!(cond.evaluate(Some(&attrs("review"))));
        assert!(!cond.evaluate(Some(&attrs("approved"))));
    }

    #[test]
    fn required_state_sem_attrs_nega() {
        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            ..Default::default()
        };
        assert!(!cond.evaluate(None));
    }

    #[test]
    fn required_classification_satisfeito() {
        let cond = DelegationCondition {
            required_classification: Some(vec![ResourceClassification::Internal]),
            ..Default::default()
        };
        let internal = ResourceAttributes::of_type("doc")
            .with_classification(ResourceClassification::Internal);
        let secret =
            ResourceAttributes::of_type("doc").with_classification(ResourceClassification::Secret);
        assert!(cond.evaluate(Some(&internal)));
        assert!(!cond.evaluate(Some(&secret)));
    }

    #[test]
    fn required_org_unit_satisfeito() {
        let cond = DelegationCondition {
            required_org_unit: Some(OrgScope::new("SF-1234")),
            ..Default::default()
        };
        let in_scope = ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-1234"));
        let out_scope = ResourceAttributes::of_type("doc").with_org_unit(OrgScope::new("SF-9999"));
        assert!(cond.evaluate(Some(&in_scope)));
        assert!(!cond.evaluate(Some(&out_scope)));
    }

    #[test]
    fn parse_json_valido() {
        let json = r#"{"required_state":["draft"]}"#;
        let cond = DelegationCondition::parse(json).unwrap();
        assert_eq!(cond.required_state, Some(vec!["draft".to_string()]));
    }

    #[test]
    fn parse_json_invalido_retorna_none() {
        assert!(DelegationCondition::parse("not-json").is_none());
        assert!(DelegationCondition::parse("").is_none());
    }

    #[test]
    fn roundtrip_json() {
        let cond = DelegationCondition {
            required_state: Some(vec!["draft".into()]),
            required_classification: Some(vec![ResourceClassification::Internal]),
            required_org_unit: Some(OrgScope::new("SF-1234")),
        };
        let json = cond.to_json().unwrap();
        let back = DelegationCondition::parse(&json).unwrap();
        assert_eq!(cond, back);
    }
}

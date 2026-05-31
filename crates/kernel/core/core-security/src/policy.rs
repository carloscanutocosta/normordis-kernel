//! Tipos e validação de políticas soberanas de segurança.

use serde::{Deserialize, Serialize};

use crate::SecurityError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyMode {
    Baseline,
    Strict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub code: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: String,
    pub version: String,
    pub mode: PolicyMode,
    pub rules: Vec<Rule>,
}

pub fn validate_policy(p: &Policy) -> Result<(), SecurityError> {
    if p.policy_id.trim().is_empty() {
        return Err(SecurityError::MissingField("policy_id".into()));
    }
    if p.version.trim().is_empty() {
        return Err(SecurityError::MissingField("version".into()));
    }
    if p.rules.is_empty() {
        return Err(SecurityError::InvalidPolicy(
            "rules deve conter pelo menos uma regra".into(),
        ));
    }
    for (i, r) in p.rules.iter().enumerate() {
        if r.code.trim().is_empty() {
            return Err(SecurityError::MissingField(format!("rules[{i}].code")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline_policy() -> Policy {
        Policy {
            policy_id: "pol-1".into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Baseline,
            rules: vec![Rule {
                code: "MIN-AUTH".into(),
                enabled: true,
                description: None,
            }],
        }
    }

    #[test]
    fn valid_baseline() {
        assert!(validate_policy(&baseline_policy()).is_ok());
    }

    #[test]
    fn missing_policy_id() {
        let mut p = baseline_policy();
        p.policy_id = "".into();
        assert!(matches!(
            validate_policy(&p),
            Err(SecurityError::MissingField(_))
        ));
    }

    #[test]
    fn empty_rules() {
        let mut p = baseline_policy();
        p.rules = vec![];
        assert!(matches!(
            validate_policy(&p),
            Err(SecurityError::InvalidPolicy(_))
        ));
    }

    #[test]
    fn strict_accepts_all_enabled() {
        let p = Policy {
            policy_id: "pol-strict".into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Strict,
            rules: vec![
                Rule {
                    code: "RULE-A".into(),
                    enabled: true,
                    description: None,
                },
                Rule {
                    code: "RULE-B".into(),
                    enabled: true,
                    description: None,
                },
            ],
        };
        assert!(validate_policy(&p).is_ok());
    }

    #[test]
    fn strict_accepts_disabled_rule_as_exemption() {
        // Regra disabled em strict = isenção explícita (ExemptedByRule no SecurityService).
        // O campo `enabled=false` é uma decisão intencional, não um erro de configuração.
        let p = Policy {
            policy_id: "pol-strict".into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Strict,
            rules: vec![
                Rule {
                    code: "AUTH".into(),
                    enabled: true,
                    description: None,
                },
                Rule {
                    code: "audit.read".into(),
                    enabled: false,
                    description: None,
                },
            ],
        };
        assert!(validate_policy(&p).is_ok());
    }
}

//! Tipos e validação de políticas soberanas de segurança.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SecurityError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyMode {
    /// Apenas operações com rule `enabled = true` exigem delegação.
    /// Operações não governadas por regra são permitidas por omissão.
    /// Adequado para desenvolvimento e ambientes controlados.
    Baseline,
    /// Toda e qualquer operação exige delegação explícita.
    /// Implementa deny-by-default real. Recomendado em produção.
    Strict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub code: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Política soberana de segurança.
///
/// ## Validade temporal
///
/// `valid_from` e `valid_to` permitem políticas com vigência limitada.
/// Uma política sem estes campos (ambos `None`) está sempre activa enquanto
/// não for revogada. O motor de autorização filtra automaticamente políticas
/// fora da janela temporal.
///
/// ## Modo e deny-by-default
///
/// `Strict` implementa deny-by-default real: sem delegação, nega sempre.
/// `Baseline` é permissivo para operações não governadas por regra explícita.
/// Em produção, usar `Strict` para garantir zero confiança implícita.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: String,
    pub version: String,
    pub mode: PolicyMode,
    pub rules: Vec<Rule>,
    /// Início da vigência. `None` = activa desde sempre.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_from: Option<DateTime<Utc>>,
    /// Fim da vigência (exclusivo). `None` = nunca expira.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_to: Option<DateTime<Utc>>,
}

impl Policy {
    /// Verdadeiro se a política está temporalmente activa em `now`.
    ///
    /// Políticas sem `valid_from`/`valid_to` estão sempre activas (sem restrição temporal).
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        let from_ok = self.valid_from.is_none_or(|f| now >= f);
        let to_ok = self.valid_to.is_none_or(|t| now < t);
        from_ok && to_ok
    }
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
    if let (Some(from), Some(to)) = (p.valid_from, p.valid_to) {
        if to <= from {
            return Err(SecurityError::InvalidPolicy(
                "valid_to deve ser posterior a valid_from".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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
            valid_from: None,
            valid_to: None,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 4, 12, 0, 0).unwrap()
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
            valid_from: None,
            valid_to: None,
        };
        assert!(validate_policy(&p).is_ok());
    }

    #[test]
    fn strict_accepts_disabled_rule_as_exemption() {
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
            valid_from: None,
            valid_to: None,
        };
        assert!(validate_policy(&p).is_ok());
    }

    #[test]
    fn valid_to_invalido_nega() {
        let t = now();
        let p = Policy {
            policy_id: "pol-1".into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Baseline,
            rules: vec![Rule {
                code: "A".into(),
                enabled: true,
                description: None,
            }],
            valid_from: Some(t),
            valid_to: Some(t), // igual → inválido
        };
        assert!(matches!(
            validate_policy(&p),
            Err(SecurityError::InvalidPolicy(_))
        ));
    }

    #[test]
    fn is_active_at_sem_janela() {
        let p = baseline_policy();
        assert!(p.is_active_at(now()), "sem valid_from/to → sempre activa");
    }

    #[test]
    fn is_active_at_dentro_da_janela() {
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 12, 31, 0, 0, 0).unwrap();
        let p = Policy {
            valid_from: Some(start),
            valid_to: Some(end),
            ..baseline_policy()
        };
        assert!(p.is_active_at(now()));
    }

    #[test]
    fn is_active_at_antes_da_janela() {
        let start = Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2027, 12, 31, 0, 0, 0).unwrap();
        let p = Policy {
            valid_from: Some(start),
            valid_to: Some(end),
            ..baseline_policy()
        };
        assert!(!p.is_active_at(now()));
    }

    #[test]
    fn is_active_at_apos_expirar() {
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let p = Policy {
            valid_from: Some(start),
            valid_to: Some(end),
            ..baseline_policy()
        };
        assert!(!p.is_active_at(now()), "now() > valid_to → expirada");
    }
}

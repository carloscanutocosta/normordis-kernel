//! Níveis de autenticação para operações do NORMORDIS.
//!
//! Usado em `SecurityContext` para indicar o nível de confiança da sessão actual,
//! e em `AuthzRequest` para declarar o nível mínimo exigido por uma operação.

use serde::{Deserialize, Serialize};

/// Nível de autenticação da identidade verificada.
///
/// Ordenado do menos restritivo (Low) para o mais restritivo (Strong),
/// permitindo comparação directa: `AuthLevel::Normal >= AuthLevel::Low`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthLevel {
    /// Consulta de dados públicos ou internos não sensíveis; sem login obrigatório.
    Low,
    /// Login institucional com sessão válida — nível padrão.
    Normal,
    /// MFA activo — para acções sensíveis.
    Reinforced,
    /// Assinatura digital ou certificado qualificado — para actos formais.
    Strong,
}

impl std::fmt::Display for AuthLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::Reinforced => write!(f, "reinforced"),
            Self::Strong => write!(f, "strong"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_total() {
        assert!(AuthLevel::Low < AuthLevel::Normal);
        assert!(AuthLevel::Normal < AuthLevel::Reinforced);
        assert!(AuthLevel::Reinforced < AuthLevel::Strong);
    }

    #[test]
    fn level_satisfacao() {
        let required = AuthLevel::Reinforced;
        assert!(
            AuthLevel::Normal < required,
            "normal não satisfaz reinforced"
        );
        assert!(
            AuthLevel::Reinforced >= required,
            "reinforced satisfaz reinforced"
        );
        assert!(AuthLevel::Strong >= required, "strong satisfaz reinforced");
    }

    #[test]
    fn display() {
        assert_eq!(AuthLevel::Low.to_string(), "low");
        assert_eq!(AuthLevel::Normal.to_string(), "normal");
        assert_eq!(AuthLevel::Reinforced.to_string(), "reinforced");
        assert_eq!(AuthLevel::Strong.to_string(), "strong");
    }

    #[test]
    fn serde_roundtrip() {
        let encoded = serde_json::to_string(&AuthLevel::Reinforced).unwrap();
        assert_eq!(encoded, "\"reinforced\"");
        let decoded: AuthLevel = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, AuthLevel::Reinforced);
    }
}

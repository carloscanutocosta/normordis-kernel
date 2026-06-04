//! Classificação de sensibilidade de recursos.
//!
//! Usado em `ResourceAttributes` para indicar quão sensível é o recurso
//! sobre o qual uma operação actua, informando a decisão de evidência em `AuthzDecision`.

use serde::{Deserialize, Serialize};

/// Classificação de sensibilidade de um recurso.
///
/// Ordenada do menos sensível (Public) para o mais sensível (Secret).
/// Permite comparação: `ResourceClassification::Restricted > ResourceClassification::Internal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceClassification {
    /// Informação pública — sem restrições de acesso.
    Public,
    /// Informação interna — acesso limitado a colaboradores autenticados.
    Internal,
    /// Informação restrita — acesso controlado, evidência normal.
    Restricted,
    /// Informação secreta — acesso muito limitado, evidência reforçada obrigatória.
    Secret,
}

impl std::fmt::Display for ResourceClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Internal => write!(f, "internal"),
            Self::Restricted => write!(f, "restricted"),
            Self::Secret => write!(f, "secret"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_total() {
        assert!(ResourceClassification::Public < ResourceClassification::Internal);
        assert!(ResourceClassification::Internal < ResourceClassification::Restricted);
        assert!(ResourceClassification::Restricted < ResourceClassification::Secret);
    }

    #[test]
    fn display() {
        assert_eq!(ResourceClassification::Secret.to_string(), "secret");
        assert_eq!(ResourceClassification::Internal.to_string(), "internal");
    }

    #[test]
    fn serde_roundtrip() {
        let encoded = serde_json::to_string(&ResourceClassification::Restricted).unwrap();
        assert_eq!(encoded, "\"restricted\"");
        let decoded: ResourceClassification = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, ResourceClassification::Restricted);
    }
}

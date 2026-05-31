//! Identidade verificada de principal de segurança.
//!
//! `VerifiedPrincipal` não pode ser construído a partir de uma `String` arbitrária —
//! o caller deve escolher explicitamente entre `human()` (identidade humana autenticada)
//! e `system()` (principal técnico declarado). Isto torna visível, em cada call-site,
//! se a identidade é verificada ou declarada.
//!
//! ## Integração com support-auth
//!
//! A camada de autenticação (support-auth) deve converter o token verificado num
//! `VerifiedPrincipal::human(user_id)` antes de entregar ao `SecurityService`.
//! O campo `user_id` deve provir directamente do claim do token — nunca de input do caller.
//!
//! ## Principals técnicos
//!
//! Workers e daemons usam `VerifiedPrincipal::system("daemon:apid")`. A nomeação
//! explícita (`system`) torna estes bypass auditáveis por grep.

use serde::{Deserialize, Serialize};

/// Tipo de principal — distingue identidade humana verificada de principal técnico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrincipalKind {
    /// Identidade humana proveniente de token autenticado.
    Human,
    /// Principal técnico declarado (daemon, worker, processo interno).
    System,
}

/// Identidade de segurança verificada.
///
/// Só pode ser construída via [`VerifiedPrincipal::human`] ou [`VerifiedPrincipal::system`].
/// Não existe construtor a partir de `String` arbitrária.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedPrincipal {
    id: String,
    kind: PrincipalKind,
}

impl VerifiedPrincipal {
    /// Cria um principal humano a partir de um ID de utilizador verificado.
    ///
    /// O `id` deve provir de um token autenticado validado por `support-auth`.
    /// A responsabilidade de verificação é do caller — este construtor não valida o token.
    pub fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: PrincipalKind::Human,
        }
    }

    /// Cria um principal técnico do sistema (daemon, worker, processo interno).
    ///
    /// Explicitamente marcado como `System` para que greps de auditoria detectem
    /// todos os pontos onde a verificação de identidade é contornada.
    pub fn system(name: impl Into<String>) -> Self {
        Self {
            id: name.into(),
            kind: PrincipalKind::System,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> &PrincipalKind {
        &self.kind
    }

    pub fn is_human(&self) -> bool {
        matches!(self.kind, PrincipalKind::Human)
    }

    pub fn is_system(&self) -> bool {
        matches!(self.kind, PrincipalKind::System)
    }
}

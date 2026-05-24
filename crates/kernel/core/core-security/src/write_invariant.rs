//! Invariante zero-trust mínima para writes críticos.

use serde::{Deserialize, Serialize};

use crate::{SecurityError, VerifiedPrincipal};

/// Contexto mínimo exigido em todo write crítico.
///
/// `principal` é um [`VerifiedPrincipal`] — não aceita strings arbitrárias.
/// O caller deve escolher explicitamente `VerifiedPrincipal::human(id)` (identidade
/// proveniente de token autenticado) ou `VerifiedPrincipal::system(name)` (principal
/// técnico declarado).
///
/// Enquanto não existir identidade autenticada real de ponta a ponta, daemons e workers
/// usam `VerifiedPrincipal::system("daemon:apid")`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteInvariantContext {
    pub operation: String,
    pub correlation_id: String,
    pub principal: VerifiedPrincipal,
}

pub fn validate_write_invariant(ctx: &WriteInvariantContext) -> Result<(), SecurityError> {
    if ctx.operation.trim().is_empty() {
        return Err(SecurityError::MissingField("operation".into()));
    }
    if ctx.correlation_id.trim().is_empty() {
        return Err(SecurityError::InvariantViolated(format!(
            "writes críticos exigem correlation_id (operation={})",
            ctx.operation
        )));
    }
    if ctx.principal.id().trim().is_empty() {
        return Err(SecurityError::InvariantViolated(format!(
            "writes críticos exigem principal identificado (operation={})",
            ctx.operation
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerifiedPrincipal;

    fn ctx(op: &str, corr: &str, principal: VerifiedPrincipal) -> WriteInvariantContext {
        WriteInvariantContext {
            operation: op.into(),
            correlation_id: corr.into(),
            principal,
        }
    }

    #[test]
    fn valid_human() {
        assert!(validate_write_invariant(&ctx(
            "save.policy",
            "corr-1",
            VerifiedPrincipal::human("user:alice")
        ))
        .is_ok());
    }

    #[test]
    fn valid_system() {
        assert!(validate_write_invariant(&ctx(
            "bootstrap.init",
            "corr-sys",
            VerifiedPrincipal::system("daemon:apid")
        ))
        .is_ok());
    }

    #[test]
    fn missing_operation() {
        assert!(matches!(
            validate_write_invariant(&ctx("", "corr-1", VerifiedPrincipal::human("u"))),
            Err(SecurityError::MissingField(_))
        ));
    }

    #[test]
    fn missing_correlation_id() {
        assert!(matches!(
            validate_write_invariant(&ctx("op", "", VerifiedPrincipal::human("u"))),
            Err(SecurityError::InvariantViolated(_))
        ));
    }

    #[test]
    fn missing_principal_id() {
        // Só possível via system("") — human("") também produziria o mesmo erro
        assert!(matches!(
            validate_write_invariant(&ctx("op", "corr", VerifiedPrincipal::system(""))),
            Err(SecurityError::InvariantViolated(_))
        ));
    }
}

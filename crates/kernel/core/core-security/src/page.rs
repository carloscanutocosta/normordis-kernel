//! Paginação de listagens no repositório de segurança.

/// Opções de paginação para `list_active_policies` e `list_delegations`.
///
/// Passar `None` nas funções que as aceitam retorna todos os resultados sem limite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListOptions {
    /// Número máximo de resultados.
    pub limit: usize,
    /// Número de resultados a ignorar (0-indexed).
    pub offset: usize,
}

impl ListOptions {
    /// Página 1-indexed com `per_page` resultados por página.
    pub fn page(page: usize, per_page: usize) -> Self {
        Self {
            limit: per_page,
            offset: page.saturating_sub(1) * per_page,
        }
    }

    /// Primeiros `n` resultados.
    pub fn first(n: usize) -> Self {
        Self {
            limit: n,
            offset: 0,
        }
    }
}

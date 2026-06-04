//! Tipos de paginação reutilizáveis em queries de `core-org`.

/// Parâmetros de paginação. `limit` é clamped a [1, 1000].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgPage {
    pub limit: u32,
    pub offset: u32,
}

impl OrgPage {
    pub fn new(limit: u32, offset: u32) -> Self {
        Self {
            limit: limit.clamp(1, 1000),
            offset,
        }
    }

    pub fn first(limit: u32) -> Self {
        Self::new(limit, 0)
    }
}

/// Resultado paginado genérico.
#[derive(Debug)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    /// Total de registos que satisfazem o filtro (independente da página).
    pub total: u64,
    pub page: OrgPage,
}

impl<T> PagedResult<T> {
    pub fn new(items: Vec<T>, total: u64, page: OrgPage) -> Self {
        Self { items, total, page }
    }

    pub fn has_more(&self) -> bool {
        (self.page.offset as u64 + self.items.len() as u64) < self.total
    }
}

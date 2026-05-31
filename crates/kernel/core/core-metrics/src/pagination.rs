/// Opções de paginação para queries de listagem.
#[derive(Debug, Clone)]
pub struct ListOptions {
    /// Número máximo de itens a retornar. 0 → sem limite (usar com cautela).
    pub limit: u32,
    pub offset: u32,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}

impl ListOptions {
    pub fn new(limit: u32, offset: u32) -> Self {
        Self { limit, offset }
    }

    /// Constrói opções para a página `page` (0-indexed) com tamanho `size`.
    pub fn page(page: u32, size: u32) -> Self {
        Self {
            limit: size,
            offset: page * size,
        }
    }

    /// Primeira página com o limite dado.
    pub fn first(limit: u32) -> Self {
        Self { limit, offset: 0 }
    }

    /// Sem limite — retorna todos os registos. Usar apenas para exports ou
    /// conjuntos conhecidamente pequenos.
    pub fn unlimited() -> Self {
        Self {
            limit: 0,
            offset: 0,
        }
    }
}

/// Resultado paginado.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub limit: u32,
    pub offset: u32,
    /// `true` se pode haver mais itens além deste lote.
    pub has_more: bool,
}

impl<T> Page<T> {
    pub fn from_items(items: Vec<T>, opts: &ListOptions) -> Self {
        let limit = opts.limit;
        let has_more = limit > 0 && items.len() as u32 == limit;
        Self {
            items,
            limit,
            offset: opts.offset,
            has_more,
        }
    }

    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            limit: self.limit,
            offset: self.offset,
            has_more: self.has_more,
        }
    }
}

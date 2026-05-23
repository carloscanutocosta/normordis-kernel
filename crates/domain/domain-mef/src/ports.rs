use chrono::{DateTime, Utc};

use crate::{DiplomaRef, MefCode, MefEntry, MefError, UpsertMefEntryRequest};

/// Port de acesso à classificação MEF.
///
/// Leitura: disponível em modo read-only (acesso a platform.db).
/// Escrita: requer acesso read-write (gestão administrativa da tabela MEF).
pub trait MefRepository {
    type Error: From<MefError>;

    // ─── Leitura ─────────────────────────────────────────────────────────────

    /// Entradas actualmente activas (effective_to IS NULL), ordenadas por código.
    fn get_current(&self) -> Result<Vec<MefEntry>, Self::Error>;

    /// Entradas activas num instante passado — permite reconstituir o contexto
    /// exacto de um documento finalizado em `timestamp`.
    fn get_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<MefEntry>, Self::Error>;

    /// Versão actual de uma entrada pelo código. None se não existir ou se
    /// tiver sido desactivada.
    fn get_entry(&self, code: &MefCode) -> Result<Option<MefEntry>, Self::Error>;

    /// Todo o historial de uma entrada (todas as versões), do mais recente
    /// para o mais antigo.
    fn get_history(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error>;

    /// Caminho hierárquico desde a raiz até ao código (inclusivo), usando a
    /// versão actualmente activa de cada nó.
    ///
    /// O primeiro elemento é a raiz; o último é `code`.
    fn resolve_path(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error>;

    // ─── Escrita ──────────────────────────────────────────────────────────────

    /// Insere ou actualiza uma entrada MEF com registo de auditoria e diploma.
    ///
    /// Se existir uma versão activa para o código, fecha-a (effective_to = agora)
    /// e cria uma nova linha com effective_from = agora.
    /// Idempotente quanto ao conteúdo: se label, parent e is_usable forem iguais,
    /// não cria nova versão.
    fn upsert_entry(&self, request: &UpsertMefEntryRequest) -> Result<(), Self::Error>;

    /// Desactiva uma entrada (marca effective_to = agora). Idempotente.
    ///
    /// Usado quando um código é abolido por diploma. O `diploma` documentará
    /// o instrumento legal que determinou a abolição.
    fn deactivate_entry(
        &self,
        code: &MefCode,
        changed_by: &str,
        change_reason: Option<&str>,
        diploma: Option<&DiplomaRef>,
    ) -> Result<(), Self::Error>;
}

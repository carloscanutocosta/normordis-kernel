use crate::{ExportError, ExportReceipt, ExportSnapshot};

/// Port de persistência de recibos de exportação.
///
/// Implementações devem garantir que `save_receipt` é atómico:
/// snapshot e audit event são escritos juntos ou nenhum é escrito.
///
/// `load_snapshot` deve re-validar o snapshot antes de o devolver —
/// invariante zero-trust: dados lidos da BD são tratados como não confiáveis.
pub trait ExportSnapshotPort {
    fn save_receipt(&self, receipt: &ExportReceipt) -> Result<(), ExportError>;
    fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<ExportSnapshot>, ExportError>;
    /// Devolve snapshots de um sujeito por ordem decrescente de `exported_at`.
    ///
    /// `limit` — número máximo de resultados (usar ≤ 1000 em produção).
    /// `offset` — paginação baseada em cursor de linha.
    fn list_for_subject(
        &self,
        subject_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ExportSnapshot>, ExportError>;
}

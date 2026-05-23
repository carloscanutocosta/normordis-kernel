# MAN — ingest-scanner

## Objectivo

Serviço de ingestão de documentos digitalizados. Orquestra a limpeza do scan e a persistência como documento custodiado, com configuração de scanner plugável.

---

## Contrato público

```rust
/// Tipo de documento criado por este serviço.
pub const SCANNED_DOCUMENT_KIND: &str = "scanned";

/// Pedido de ingestão de scan.
pub struct ScanIngestRequest {
    pub filename: String,
    pub data: Vec<u8>,
    pub mime_type: Option<String>,   // ex: "application/pdf" | "image/tiff"
    pub scanned_by: String,
    pub unit_id: Option<String>,     // unidade orgânica responsável
    pub notes: Option<String>,
}

/// Erros de ingestão.
pub enum ScanIngestError {
    EmptyData,
    InvalidFilename,
    CleaningFailed(String),
    PersistenceFailed(String),
}

/// Trait plugável de limpeza de scan.
pub trait Scanner: Send + Sync {
    fn clean(&self, data: &[u8], mime_type: Option<&str>) -> Result<Vec<u8>, ScanIngestError>;
}

/// Implementação padrão — devolve os dados sem transformação.
pub struct AlwaysCleanScanner;
impl Scanner for AlwaysCleanScanner {
    fn clean(&self, data: &[u8], _mime_type: Option<&str>) -> Result<Vec<u8>, ScanIngestError> {
        Ok(data.to_vec())
    }
}

/// Configuração de ingestão (scanner e opções).
pub struct ScanIngestConfig<S: Scanner> {
    pub scanner: S,
}

pub fn scanned_document_ingest_config<S: Scanner>(scanner: S) -> ScanIngestConfig<S>;

/// Ingestão principal: limpa o scan e persiste via custody_repo.
pub fn ingest_scanned_document<R: DocumentCustodyRepository, S: Scanner>(
    request: &ScanIngestRequest,
    custody_repo: &R,
    config: &ScanIngestConfig<S>,
) -> Result<String, ScanIngestError>; // devolve document_id
```

---

## Como usar

### Com `AlwaysCleanScanner` (sem processamento)

```rust
use ingest_scanner::{
    ingest_scanned_document, scanned_document_ingest_config,
    AlwaysCleanScanner, ScanIngestRequest,
};

let config = scanned_document_ingest_config(AlwaysCleanScanner);

let document_id = ingest_scanned_document(
    &ScanIngestRequest {
        filename: "declaracao-2025-001.pdf".into(),
        data: std::fs::read("scan.pdf")?,
        mime_type: Some("application/pdf".into()),
        scanned_by: "operador-1".into(),
        unit_id: Some("u-dgf".into()),
        notes: Some("Digitalizado em 2025-05-22".into()),
    },
    &custody_repo,
    &config,
)?;

println!("Documento custodiado: {document_id}");
```

### Com scanner personalizado (ex.: remoção de páginas em branco)

```rust
use ingest_scanner::{Scanner, ScanIngestError, scanned_document_ingest_config};

struct BlankPageRemover;
impl Scanner for BlankPageRemover {
    fn clean(&self, data: &[u8], mime_type: Option<&str>) -> Result<Vec<u8>, ScanIngestError> {
        // lógica de remoção de páginas em branco
        remove_blank_pages(data).map_err(|e| ScanIngestError::CleaningFailed(e.to_string()))
    }
}

let config = scanned_document_ingest_config(BlankPageRemover);
```

---

## Invariantes

- `ScanIngestRequest::data` não pode estar vazio — devolve `ScanIngestError::EmptyData`.
- `ScanIngestRequest::filename` não pode estar vazio — devolve `ScanIngestError::InvalidFilename`.
- O documento criado tem `kind = SCANNED_DOCUMENT_KIND` ("scanned").
- A limpeza é aplicada antes da persistência — os bytes persistidos são os bytes limpos.

---

## Limites actuais

- Sem OCR — o conteúdo textual do scan não é extraído.
- Sem classificação automática MEF — o caller deve fornecer o código se necessário.
- `AlwaysCleanScanner` não transforma nada — é apenas um placeholder.
- Sem suporte a multipage TIFF com páginas individuais.

---

## ToDo

- [ ] Integração com motor OCR (ex.: tesseract-rs) como scanner opcional.
- [ ] Classificação automática por heurística de conteúdo.
- [ ] Suporte a TIFF multipage com conversão para PDF.

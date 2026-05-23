# MAN — documental-sqlite

## Objectivo

Persistência SQLite para o domínio documental. Implementa os cinco ports documentais sobre uma base de dados única, com deduplicação content-addressed de blobs (SHA-256) e verificação de integridade na leitura.

---

## Contrato público

```rust
pub struct DocumentalSqliteStore {
    conn: rusqlite::Connection,
}

impl DocumentalSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, DocumentalError>;
}

/// Migrações exportadas para uso externo.
pub const DOCUMENTAL_SQLITE_MIGRATIONS: &[&str];
```

`DocumentalSqliteStore` implementa os seguintes ports:

```rust
impl TemplateRepository for DocumentalSqliteStore { ... }
impl DocumentCustodyRepository for DocumentalSqliteStore { ... }
impl NdfArchive for DocumentalSqliteStore { ... }
impl DocumentEventLog for DocumentalSqliteStore { ... }
impl AttachmentStore for DocumentalSqliteStore { ... }
```

---

## Schema (2 migrações)

**Migração 1 — Schema base:**

| Tabela | Conteúdo |
|---|---|
| `doc_templates` | Templates com conteúdo e metadados |
| `doc_custody` | Documentos em custódia com status e hash |
| `doc_ndf_archive` | Documentos NDF arquivados com hash SHA-256 |
| `doc_events` | Eventos documentais (audit trail imutável) |

**Migração 2 — Blobs de anexos:**

```sql
CREATE TABLE IF NOT EXISTS attachment_blobs (
    Hash       TEXT PRIMARY KEY,   -- SHA-256 hex do conteúdo
    MimeType   TEXT NOT NULL,
    Data       BLOB NOT NULL,
    SizeBytes  INTEGER NOT NULL,
    StoredAt   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS doc_attachments (
    AttachmentId TEXT PRIMARY KEY,
    DocumentId   TEXT NOT NULL,
    BlobHash     TEXT NOT NULL REFERENCES attachment_blobs(Hash),
    Filename     TEXT NOT NULL,
    AttachedAt   TEXT NOT NULL,
    AttachedBy   TEXT NOT NULL
);
```

---

## Deduplicação content-addressed

Ao guardar um anexo, o store calcula o SHA-256 do conteúdo e verifica se o blob já existe:

```rust
// Pseudo-código da lógica interna
let hash = sha256_hex(data);
if !blob_exists(conn, &hash) {
    insert_blob(conn, hash, mime_type, data);
}
insert_attachment(conn, attachment_id, document_id, hash, filename, ...);
```

Na leitura, o hash é verificado contra o conteúdo retornado — se não coincidir, devolve `DocumentalError::IntegrityViolation`.

---

## Como usar

```rust
use documental_sqlite::DocumentalSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;

let store = DocumentalSqliteStore::open(
    &SqliteRelationalConfig::read_write_create("docs.db")
)?;

// Guardar template
store.save_template(&DocTemplate {
    id: "tmpl-oficio-v1".into(),
    name: "Ofício AT v1".into(),
    content: include_str!("template.typ").into(),
    ..Default::default()
})?;

// Guardar anexo (deduplicado automaticamente)
let attachment_id = store.save_attachment(&AttachmentRequest {
    document_id: "doc-1".into(),
    filename: "mapa.pdf".into(),
    mime_type: "application/pdf".into(),
    data: pdf_bytes,
    attached_by: "user-1".into(),
})?;
```

---

## Invariantes

- A integridade SHA-256 é verificada em cada leitura de blob — `IntegrityViolation` se o hash não coincidir.
- Blobs são imutáveis — nunca actualizados, apenas inseridos ou referenciados.
- Eventos documentais são append-only — sem DELETE ou UPDATE nas tabelas de eventos.
- As migrações são idempotentes.

---

## Limites actuais

- `conn` não é protegido por `Mutex` — não thread-safe por defeito. Use `Arc<Mutex<DocumentalSqliteStore>>` no caller.
- Sem encriptação dos blobs.
- Sem purge de blobs órfãos (sem referências em `doc_attachments`).

---

## ToDo

- [ ] Wrapping em `Arc<Mutex<>>` interno para thread-safety transparente.
- [ ] Job de purge de blobs órfãos.
- [ ] Suporte a streaming de blobs grandes (sem carregar tudo em memória).

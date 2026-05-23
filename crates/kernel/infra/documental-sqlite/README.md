# documental-sqlite

Adaptador SQLite para o domínio documental — templates, custódia de documentos, arquivo NDF, eventos e anexos com deduplicação content-addressed.

## Objectivo

Implementa os ports documentais (`TemplateRepository`, `DocumentCustodyRepository`, `NdfArchive`, `DocumentEventLog`, `AttachmentStore`) sobre uma única base de dados SQLite com verificação de integridade SHA-256.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite` e dos ports definidos no domínio documental (core-documental ou equivalente).

## Responsabilidade

- Persistir templates de documentos.
- Gerir a custódia de documentos ao longo do seu ciclo de vida.
- Arquivar documentos NDF com integridade SHA-256.
- Registar eventos documentais (audit trail).
- Armazenar anexos com deduplicação por hash SHA-256 (`attachment_blobs`).

## Não-responsabilidade

- Não gera PDFs — use `documentos-pdf` ou `pdf-pipeline`.
- Não numera documentos — use `numerador-sqlite`.
- Não encripta os blobs (armazenamento em texto/binário claro).

## Exemplo mínimo

```rust
use documental_sqlite::DocumentalSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;

let config = SqliteRelationalConfig::read_write_create("docs.db");
let store = DocumentalSqliteStore::open(&config)?;
store.save_template(&template)?;
```

## Validação

```sh
cargo test -p documental-sqlite
```

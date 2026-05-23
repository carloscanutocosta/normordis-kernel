# Manual do modulo exports-sqlite

## Objetivo

`exports-sqlite` e o adapter SQLite para `ExportSnapshotPort`. Persiste `ExportReceipt`
num ficheiro SQLite write-once e permite abertura em modo read-only para consumidores.
Re-valida snapshots ao carregar (principio zero-trust: o ficheiro SQLite nao e confiavel
por definicao).

## Contrato publico

```rust
ExportsSqliteStore
ExportsSqliteError
EXPORTS_SQLITE_MIGRATIONS
```

### Construtores

```rust
// Cria ou abre em leitura-escrita; aplica migracoes de schema.
ExportsSqliteStore::open_write_create(config: &SqliteRelationalConfig)
    -> Result<Self, ExportsSqliteError>

// Abre em modo read-only (PRAGMA query_only = ON).
// Qualquer tentativa de escrita falha com erro SQLite.
ExportsSqliteStore::open_readonly(path: impl Into<PathBuf>)
    -> Result<Self, ExportsSqliteError>
```

### ExportSnapshotPort implementado

```rust
fn save_receipt(&self, receipt: &ExportReceipt) -> Result<(), ExportError>
fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<ExportSnapshot>, ExportError>
fn list_for_subject(&self, subject_id: &str, limit: usize, offset: usize)
    -> Result<Vec<ExportSnapshot>, ExportError>
```

## Schema SQLite

```sql
export_snapshots (
    snapshot_id          TEXT PRIMARY KEY,
    exported_at          TEXT NOT NULL,     -- ISO 8601
    source_kind          TEXT NOT NULL,
    source_subject_id    TEXT NOT NULL,
    source_version       TEXT NOT NULL,
    manifest_algorithm   TEXT NOT NULL,
    manifest_hash        TEXT NOT NULL,
    manifest_item_count  INTEGER NOT NULL,
    document_package_json TEXT NOT NULL,   -- JSON serializado
    meta_json            TEXT,             -- opcional
    saved_at             TEXT NOT NULL
)

export_audit_events (
    event_id     TEXT PRIMARY KEY,
    snapshot_id  TEXT NOT NULL REFERENCES export_snapshots(snapshot_id),
    event_type   TEXT NOT NULL,
    event_json   TEXT NOT NULL,
    saved_at     TEXT NOT NULL
)
```

Indices: `idx_exports_subject (source_subject_id, exported_at)`,
`idx_export_audit_snapshot (snapshot_id)`.

## Invariantes

- `save_receipt` e atomico: snapshot e audit event sao escritos juntos numa transacao
  `BEGIN/COMMIT`. Em caso de falha, `ROLLBACK` garante que nenhum dos dois fica gravado.
- `save_receipt` e idempotente: usa `INSERT OR IGNORE`. Um segundo `save_receipt` com
  o mesmo `receipt` nao produz erro nem duplica registos.
- `open_readonly` define `PRAGMA query_only = ON` — qualquer escrita falha ao nivel
  do SQLite, mesmo que o caller tente aceder diretamente a conexao.
- `load_snapshot` e `list_for_subject` re-validam cada snapshot com
  `validate_export_snapshot`, incluindo re-computacao do hash do manifesto. Um snapshot
  com hash adulterado falha com `ExportsSqliteError::Infra`.
- `list_for_subject` aceita `limit` e `offset` para paginacao; sem limite o SQLite
  devolveria todos os registos em memoria.

## Politica de leitura zero-trust

O ficheiro SQLite pode estar acessivel a outros processos locais. A re-validacao do
hash do manifesto no load garante que um snapshot modificado diretamente no ficheiro
(fora do adapter) nao passa despercebido.

## Erros

`ExportsSqliteError` envolve erros de infra internos que nao sao expostos diretamente
no contrato de `ExportSnapshotPort`. O adapter mapeia todos os seus erros para
`ExportError::InvalidPackage(mensagem)` antes de devolver ao caller.

| Variante                      | Situacao                                               |
|-------------------------------|--------------------------------------------------------|
| `Sqlite(rusqlite::Error)`     | Erro nativo SQLite (IO, constraint, lock)              |
| `Infra(String)`               | Erro de abertura, configuracao ou migracao do adapter  |
| `Json(String)`                | Falha a serializar/desserializar JSON de um campo      |
| `InvalidDateTime(String)`     | Timestamp armazenado nao parseavel para `DateTime<Utc>`|
| `SnapshotNotFound(String)`    | Reservado; nao usado atualmente                        |

## Limitacoes atuais

- Conexao sincronizada por processo; sem pool de conexoes.
- `list_for_subject` usa paginacao por `offset` — pode ser lenta para offsets grandes
  em tabelas com muitos registos. Sem cursor opaco ou anchor por `snapshot_id`.
- Sem vacuum automatico; o caller e responsavel pela manutencao do ficheiro SQLite.
- Sem migracao de schema automatica apos schema v1; novas migracoes exigem extensao
  de `EXPORTS_SQLITE_MIGRATIONS`.

## ToDo

- Cursor opaco ou anchor por `snapshot_id` para paginacao estavel.
- Suporte a vacuum controlado.
- Verificacao de integridade `PRAGMA integrity_check` na abertura (opcional).

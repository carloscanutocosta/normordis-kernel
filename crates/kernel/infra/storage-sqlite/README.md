# storage-sqlite

Implementação SQLite de `support-storage` — key-value store JSON sem encriptação, adequada para dados auditáveis legíveis directamente.

## Objectivo

Fornece `PlainJsonSqliteStorage`, uma implementação do trait `Storage` sobre SQLite com namespace e chave, usando uma tabela `kv_json` simples e sem encriptação. Adequada para audit trails e configurações visíveis em texto claro.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite` e `support-storage`.

## Responsabilidade

- Persistir valores JSON por `(Namespace, Key)` com timestamp de actualização.
- Suportar upsert atómico (`put_json`), insert-if-absent (`put_json_if_absent`), leitura (`get_json`) e eliminação (`delete`).
- Ser thread-safe via `Arc<Mutex<Connection>>`.

## Não-responsabilidade

- Não encripta dados — para dados sensíveis, use `SqliteRawStorage` com SQLCipher.
- Não suporta queries por namespace (listar todas as chaves de um namespace).
- Não notifica alterações (sem pub/sub).

## Exemplo mínimo

```rust
use storage_sqlite::PlainJsonSqliteStorage;
use adapter_sqlite::SqliteRelationalConfig;
use support_storage::{StorageNamespace, StorageKey};
use serde_json::json;

let config = SqliteRelationalConfig::read_write_create("audit.db");
let storage = PlainJsonSqliteStorage::open(&config)?;

let ns = StorageNamespace::new("audit.events").unwrap();
let key = StorageKey::new("ev-1").unwrap();
storage.put_json(&ns, &key, &json!({"event_type": "document.created"}))?;
```

## Validação

```sh
cargo test -p storage-sqlite
```

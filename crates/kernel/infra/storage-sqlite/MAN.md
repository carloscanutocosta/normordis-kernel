# MAN — storage-sqlite

## Objectivo

Implementação SQLite sem encriptação do trait `Storage` de `support-storage`. Armazena valores JSON indexados por namespace e chave, com timestamp de última actualização. Thread-safe via `Arc<Mutex<Connection>>`.

---

## Contrato público

```rust
pub struct PlainJsonSqliteStorage {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl PlainJsonSqliteStorage {
    /// Abre (ou cria) a base de dados e executa a migração.
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, StorageError>;
}
```

`PlainJsonSqliteStorage` implementa `support_storage::Storage`:

```rust
impl Storage for PlainJsonSqliteStorage {
    /// Insere ou substitui o valor. Actualiza UpdatedAtUtc.
    fn put_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<(), StorageError>;

    /// Insere apenas se a chave não existir. Devolve true se inseriu.
    fn put_json_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<bool, StorageError>;

    /// Lê o valor. None se a chave não existir.
    fn get_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageValue>, StorageError>;

    /// Elimina a chave. Idempotente se não existir.
    fn delete(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<(), StorageError>;
}
```

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS kv_json (
    Namespace    TEXT NOT NULL,
    Key          TEXT NOT NULL,
    ValueJson    TEXT NOT NULL,
    UpdatedAtUtc TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (Namespace, Key)
);
```

---

## Como usar

```rust
use storage_sqlite::PlainJsonSqliteStorage;
use support_storage::{Storage, StorageNamespace, StorageKey};
use serde_json::json;

let storage = PlainJsonSqliteStorage::open(&config)?;

let ns = StorageNamespace::new("audit.events").unwrap();
let key = StorageKey::new("ev-2025-001").unwrap();

// Inserir
storage.put_json(&ns, &key, &json!({
    "event_type": "document.signed",
    "actor": "user-1",
    "document_id": "doc-42"
}))?;

// Ler
if let Some(value) = storage.get_json(&ns, &key)? {
    println!("{}", value["event_type"]);
}

// Insert-if-absent (útil para deduplicação de eventos)
let inserted = storage.put_json_if_absent(&ns, &key, &json!({"v": 1}))?;
println!("Inserido: {inserted}"); // false — já existia

// Eliminar
storage.delete(&ns, &key)?;
```

### Com Clone (thread-safe)

```rust
// PlainJsonSqliteStorage implementa Clone via Arc<Mutex<>>
let storage2 = storage.clone();
std::thread::spawn(move || {
    storage2.put_json(&ns, &key, &json!({"from": "thread"})).unwrap();
});
```

---

## Invariantes

- `put_json` usa `INSERT OR REPLACE` — o timestamp `UpdatedAtUtc` é sempre actualizado.
- `put_json_if_absent` usa `INSERT OR IGNORE` — não actualiza se já existir.
- Namespaces são completamente isolados — a mesma chave em namespaces diferentes não colide.
- `delete` é idempotente — não devolve erro se a chave não existir.
- Thread-safe: o `Mutex` protege todas as operações de leitura e escrita.

---

## Limites actuais

- Sem suporte a listagem de chaves por namespace.
- Sem suporte a TTL (expiração automática de entradas).
- Sem encriptação — não adequado para dados pessoais ou segredos.
- Sem transacções multi-operação expostas ao exterior.

---

## ToDo

- [ ] Operação `list_keys(namespace)` para enumerar chaves.
- [ ] TTL opcional por chave.
- [ ] Versão encriptada (`EncryptedJsonSqliteStorage`) com SQLCipher.

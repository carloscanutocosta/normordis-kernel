# exports-sqlite

Adapter SQLite write-once / read-only para `core-exports`.

## Responsabilidade

- Persistir `ExportReceipt` (snapshot + audit event) de forma atómica num ficheiro SQLite.
- Abrir o mesmo ficheiro em modo read-only para consumidores que apenas leem exports.
- Re-validar snapshots ao carregar — invariante zero-trust: dados lidos da BD são tratados como não confiáveis.

## Não responsabilidade

- Não contém lógica de negócio — é um adapter de persistência puro.
- Não decide se um export é autorizado.
- Não gere o ciclo de vida do ficheiro SQLite (criação de diretorias, backups, retenção).

## Exemplo mínimo

```rust
use exports_sqlite::ExportsSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;

// Produção: escrever recibos
let store = ExportsSqliteStore::open_write_create(
    &SqliteRelationalConfig::read_write_create(path)
)?;
store.save_receipt(&receipt)?;

// Consumidor: leitura segura
let ro = ExportsSqliteStore::open_readonly(path)?;
let snapshot = ro.load_snapshot("exp:config_profile:dev:1.0.0:abcd1234efgh5678")?;
let lista = ro.list_for_subject("dev", 50, 0)?;
```

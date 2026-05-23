# address-sqlite

Adapter SQLite para lookup de moradas por codigo postal.

## Objetivo

Materializar a capability `support-address` sobre a tabela SQLite `platform_reference_postal_code`, usando `adapter-sqlite` como ponte de infraestrutura.

## Responsabilidade

- Abrir conexoes SQLite atraves de `adapter-sqlite`.
- Consultar candidatos de morada por codigo postal.
- Mapear rows SQLite para `support_address::AddressCandidate`.

## Nao responsabilidade

- Definir regras de dominio sobre moradas.
- Escolher qual candidato deve ser usado.
- Implementar UI, geocodificacao ou edicao de referencias.

## Exemplo minimo

```rust
use adapter_sqlite::SqliteOptions;
use address_sqlite::SqliteAddressStore;

let store = SqliteAddressStore::open(&SqliteOptions::read_only("platform.db"))?;
let candidates = store.lookup_postal_code("4700-001")?;
# Ok::<(), address_sqlite::AddressSqliteError>(())
```

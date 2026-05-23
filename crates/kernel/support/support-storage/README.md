# support-storage

Contratos de storage logico protegido do Mini-Kernel RS.

## Objetivo

Fornecer a camada de storage logico JSON cifrado, sem conhecer SQLite, ficheiros,
Tauri, UI ou dominio.

Pipeline canonico:

```text
StorageValue
-> JsonStorageCodec
-> StorageAad
-> support_crypto::EncryptedPayload
-> support_crypto::StorageEnvelope
-> RawStorage
```

## Responsabilidade

- Validar `StorageNamespace` e `StorageKey`.
- Serializar/deserializar `StorageValue` com `JsonStorageCodec`.
- Proteger bytes com `support-crypto`.
- Definir `RawStorage` para backends fisicos.
- Definir `Storage` para consumidores logicos.
- Suportar escrita condicional `put_*_if_absent` para registos append-only.
- Fornecer `MemoryStorage` para testes e uso em memoria.

## Nao responsabilidade

- Nao implementa SQLite.
- Nao depende de `adapter-sqlite`.
- Nao depende de `infra-secrets`.
- Nao cria SQL, tabelas, migrations, WAL, queues ou transacoes.
- Nao contem logica de dominio.

## Testes

```text
cargo test -p support-storage
```

Teste de stress concorrente de 5 minutos:

```text
cargo test -p support-storage --test stress_tests -- --ignored --nocapture
```

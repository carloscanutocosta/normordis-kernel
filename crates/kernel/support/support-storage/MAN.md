# MAN.md

## Nome

support-storage

## Posicao arquitetural

```text
crates/kernel/support/support-storage
```

Pertence a `kernel/support` porque define contratos transversais e headless para
storage logico protegido.

## Contrato publico

- `StorageNamespace`
- `StorageKey`
- `StorageValue`
- `StorageCodec`
- `JsonStorageCodec`
- `CryptoStorageProtector`
- `RawStorage`
- `Storage`
- `ProtectedStorage`
- `MemoryStorage`
- `StorageError`

Operacoes relevantes:

```rust
Storage::put_json
Storage::put_json_if_absent
Storage::get_json
Storage::delete
RawStorage::put_raw
RawStorage::put_raw_if_absent
```

`put_*_if_absent` devolve `true` quando gravou e `false` quando a chave ja
existia. Backends de producao devem implementar esta operacao de forma atomica.

## Pipeline

```text
StorageValue
-> JsonStorageCodec
-> StorageAad
-> EncryptedPayload
-> StorageEnvelope
-> RawStorage
```

`support-storage` reutiliza os tipos reais de `support-crypto` e nao cria
payload criptografico proprio.

## Regras de seguranca

- Backends `RawStorage` guardam `StorageEnvelope`, nunca `StorageValue`.
- AAD canonico usa `StorageAad::new(namespace, key, "value")`.
- Erros publicos nao expõem valores JSON, plaintext, ciphertext, chaves ou AAD.
- `MemoryStorage` e apenas backend fisico em memoria e guarda envelopes cifrados.

## Fora de ambito

- SQLite e outros adapters fisicos.
- Tauri/UI.
- Filesystem.
- Gestao concreta de segredos.
- Logica de dominio.
- Transacoes, WAL, retries, queues e checkpoints.

## Limitacoes atuais

- API logica inicial apenas JSON.
- Sem listagem/paginacao de chaves.
- Sem scan/listagem nativa.

## ToDo

- Avaliar API de listagem e namespaces.
- Definir politicas de migracao/rotacao coordenadas com `support-crypto`.

## Teste de stress

Existe um teste ignorado por defeito para 5 minutos de leituras/escritas
concorrentes em `MemoryStorage`:

```text
cargo test -p support-storage --test stress_tests -- --ignored --nocapture
```

## Ultima revisao

2026-05-11

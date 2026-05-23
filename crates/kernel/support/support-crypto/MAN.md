# MAN.md

## Nome

support-crypto

## Posicao arquitetural

```text
crates/kernel/support/support-crypto
```

Este crate pertence a `kernel/support` porque fornece uma primitive tecnica
transversal, headless e reutilizavel.

## Objetivo

Fornecer cifragem autenticada, derivacao segura de chaves e contratos minimos
para dados cifrados em repouso.

O objetivo imediato e deixar a capacidade criptografica pronta para
`support-storage`, que podera cifrar payloads antes da persistencia em adapters
como `adapter-sqlite` ou um futuro `adapter-postgres`.

## Motivacao

Evitar implementacao ad-hoc de cifragem e concentrar num ponto comum a politica
minima de seguranca criptografica do workspace.

## Ambito

- derivacao de chave com `Argon2id`
- cifragem autenticada com `XChaCha20-Poly1305`
- payload versionado e serializavel
- validacao de envelope cifrado
- AAD canonico para storage
- contratos minimos `KeyProvider` e `KeyResolver`
- chaves externas com `SecretKey` e `KeyId`
- envelope de storage com `StorageEnvelope`
- politica minima com `CryptoPolicy`
- conversao segura para `support_errors::MiniError`

## Fora de ambito

- TLS para dados em transito
- armazenamento de chaves no sistema operativo
- politica de desbloqueio/unlock da app
- decisao de quais campos devem ser cifrados
- PKI e certificados
- assinatura digital
- UI

## Principio de neutralidade de UI/UX

Esta biblioteca e agnostica de UI/UX e nao depende de qualquer framework visual.
Pode ser consumida por aplicacoes desktop, web, CLI, servicos locais ou testes
automatizados.

## Contrato publico

### Tipos publicos

- `KdfConfig`
- `EncryptedPayload`
- `SecretKey`
- `KeyId`
- `StorageAad`
- `StorageEnvelope`
- `KeyProvider`
- `KeyResolver`
- `StaticKeyProvider`
- `CryptoPolicy`

### Funcoes / metodos publicos

- `encrypt_bytes_with_passphrase()`
- `decrypt_bytes_with_passphrase()`
- `encrypt_text_with_passphrase()`
- `decrypt_text_with_passphrase()`
- `encrypt_bytes_with_key()`
- `decrypt_bytes_with_key()`
- `encrypt_text_with_key()`
- `decrypt_text_with_key()`
- `derive_key_from_passphrase()`
- `validate_encrypted_payload()`

### Constantes publicas

- `CURRENT_CRYPTO_VERSION`
- `CURRENT_ALGORITHM`
- `CURRENT_KDF`
- `EXTERNAL_KEY`
- `KEY_LENGTH_BYTES`
- `SALT_LENGTH_BYTES`
- `NONCE_LENGTH_BYTES`

### Erros publicos

- `CryptoError`

`CryptoError` implementa `to_mini_error()` e `From<CryptoError> for MiniError`.
As mensagens publicas resultantes nao incluem payload, passphrase, chaves,
nonce, salt, AAD ou detalhes internos.

## StorageAad

Contexto canonico para cifragem de storage:

```rust
pub struct StorageAad {
    pub namespace: String,
    pub record_id: String,
    pub field: String,
}
```

Formato canonico:

```text
mini-kernel:v1:storage:<namespace>:<record_id>:<field>
```

Os segmentos nao podem ser vazios, conter whitespace ou conter `:`.

## KeyProvider e KeyResolver

`KeyProvider` obtem a chave atual para escrita:

```rust
pub trait KeyProvider {
    fn current_key(&self) -> Result<SecretKey, MiniError>;
}
```

`KeyResolver` resolve a chave necessaria para leitura/decifragem:

```rust
pub trait KeyResolver {
    fn key_for_id(&self, key_id: Option<&str>) -> Result<SecretKey, MiniError>;
}
```

Implementacoes concretas devem viver no runtime/bootstrap ou em adapters de
segredos futuros. `StaticKeyProvider` existe para testes, bootstrap controlado e
cenarios locais simples, nao para substituir keyring em producao regulada.

## KeyId

Identificador estavel de chave, pensado para rotacao futura.

Regras minimas:

- nao vazio
- sem whitespace
- sem `:`

## StorageEnvelope

Envelope serializavel para transporte/persistencia de payload cifrado com o
contexto AAD canonico:

```rust
pub struct StorageEnvelope {
    pub aad: StorageAad,
    pub payload: EncryptedPayload,
}
```

## CryptoPolicy

Politica minima aplicavel por consumidores:

- `CryptoPolicy::permissive()`
- `CryptoPolicy::storage_default()`

A politica de storage exige `key_id` e AAD canonico antes de aceitar um envelope
para persistencia.

## Invariantes

- a versao publica atual e `1`
- o algoritmo de cifragem publico atual e `XChaCha20-Poly1305`
- o KDF publico atual e `Argon2id`
- payloads com chaves externas usam `kdf.algorithm = "ExternalKey"`
- o nonce tem 24 bytes
- a chave tem 32 bytes
- qualquer adulteracao do payload ou do AAD deve falhar na decifragem
- payloads devem ser validados antes da decifragem
- mensagens publicas de erro criptografico devem ser seguras para fronteiras
- AAD de storage deve usar `StorageAad`
- payloads de storage devem usar `key_id`
- `StorageEnvelope` deve ser validado com `CryptoPolicy::storage_default()`
  antes de persistencia

## Regras de uso

- usar esta biblioteca para cifrar dados sensiveis em repouso
- usar AAD sempre que existir um contexto logico estavel
- para storage de producao, usar chaves externas via `SecretKey`/`KeyProvider`
  em vez de guardar semantica de passphrase na camada de persistencia
- para storage, usar AAD estavel no formato
  `mini-kernel:v1:storage:<namespace>:<record_id>:<field>`
- nao usar esta biblioteca como substituto de TLS
- nao expor selecao arbitraria de algoritmos ao consumidor final
- nao guardar passphrases, chaves ou plaintext em logs
- nao cifrar diretamente no `adapter-sqlite`; cifrar na camada de storage ou
  servico que conhece o contexto semantico

## Dependencias permitidas

- `serde`
- `thiserror`
- `argon2`
- `chacha20poly1305`
- `rand`
- `base64`
- `zeroize` (feature `derive`)
- `support-errors`

## Dependencias proibidas

- frameworks UI
- gestao de certificados
- bibliotecas de transporte

## Persistencia

Nao tem persistencia propria.
O payload cifrado e devolvido como estrutura serializavel para persistencia
noutras camadas.

Para storage em SQLite/PostgreSQL, a recomendacao inicial e persistir
`StorageEnvelope` como JSON ou como envelope equivalente. O adapter deve
persistir bytes/texto ja cifrados; a decisao de cifrar pertence a
`support-storage` ou a uma camada superior com contexto.

## Seguranca e integridade

- derivacao de chave com `Argon2id`
- cifragem autenticada com `XChaCha20-Poly1305`
- usa nonce aleatorio por operacao
- suporta AAD para reforcar integridade contextual
- suporta `key_id` para preparar rotacao de chaves e resolucao futura
- suporta envelopes de storage e politica minima para impedir persistencia de
  payloads sem contexto
- `SecretKey` implementa `ZeroizeOnDrop`
- o buffer temporario de 32 bytes em `derive_key_from_passphrase` e envolvido
  em `Zeroizing<>`
- `SecretKey` nao implementa `Clone` nem `PartialEq`
- `SecretKey` implementa `Debug` como `[REDACTED]`
- `StaticKeyProvider` redige a chave em `Debug` e limpa o buffer interno ao
  sair de ambito
- responsabilidade do consumidor: a `passphrase` e recebida como `&str`; a
  zeragem da string original e responsabilidade da camada chamadora

## Compatibilidade e versionamento

Esta biblioteca segue SemVer.
Qualquer alteracao breaking exige atualizacao de versao major e revisao deste
`MAN.md`.

## Exemplos de uso

### Exemplo 1: passphrase

```txt
encrypt_text_with_passphrase("segredo", "passphrase", Some(b"doc:1"))
```

### Exemplo 2: chave externa para storage

```txt
encrypt_text_with_key("segredo", &key, Some(&aad_bytes), Some(&key_id))
```

## Estrutura interna

```text
src/
  aad.rs
  constants.rs
  crypto.rs
  error.rs
  key.rs
  lib.rs
  payload.rs
  policy.rs
tests/
MAN.md
README.md
CHANGELOG.md
```

## Notas de implementacao

A implementacao atual fixa as escolhas criptograficas seguras por omissao em
vez de expor configuracao excessiva ao consumidor.

## Estado

Operacional como primitive criptografica do Mini-Kernel RS.

## Limitacoes atuais

- Nao gere keyring, secrets do SO ou rotacao efetiva de chaves.
- Define `KeyProvider`/`KeyResolver`, mas nao fornece implementacao concreta de
  keyring do sistema operativo.
- Nao cifra automaticamente estruturas de storage.
- Nao fornece envelope binario compacto; o contrato atual e serializavel via
  serde.
- Nao implementa recifragem automatica nem migracao de algoritmos antigos.

## ToDo

- Avaliar rotacao de chaves e recifragem por namespace.
- Adicionar property tests/fuzzing para envelopes invalidos.
- Avaliar schema JSON formal para `EncryptedPayload` e `StorageEnvelope`.
- Implementar provider concreto de secrets/keyring fora deste crate.

## Ultima revisao

2026-05-11

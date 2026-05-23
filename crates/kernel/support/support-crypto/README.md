# support-crypto

Crate headless de suporte criptografico do Mini-Kernel RS.

Fornece derivacao segura de chaves, cifragem autenticada para dados em repouso
e contratos minimos para storage cifrado. Deve ser usado por componentes como
`support-storage` para cifrar payloads antes da persistencia em adapters
concretos.

## Objetivo

Fornecer uma API pequena e segura para cifrar e decifrar bytes/texto com `Argon2id` e `XChaCha20-Poly1305`.

## Responsabilidade

- Derivar chaves a partir de passphrase com `Argon2id`.
- Cifrar bytes/texto com `XChaCha20-Poly1305`.
- Produzir envelope `EncryptedPayload` serializavel e versionado.
- Validar envelopes cifrados antes de decifrar.
- Fornecer `StorageAad` canonico para cifragem de storage.
- Definir `KeyProvider` e `KeyResolver` minimos para futura integracao com
  storage/runtime.
- Suportar chaves externas via `SecretKey`, `KeyId` e `StaticKeyProvider`.
- Fornecer `StorageEnvelope` e `CryptoPolicy` para aplicar requisitos minimos
  antes de persistir dados cifrados.
- Converter `CryptoError` para `MiniError` seguro.
- Suportar AAD para integridade contextual.

## Nao responsabilidade

- Nao gere segredos no sistema operativo.
- Nao decide passphrase, unlock flow ou keyring.
- Nao persiste dados.
- Nao cifra automaticamente SQLite.
- Nao substitui TLS/HTTPS para dados em transito.
- Nao depende de UI/Tauri.

## Estado

Primitiva operacional para futuros componentes de storage cifrado. A gestao
real de chaves continua fora deste crate.

## Contrato público

Consultar:

- `MAN.md`

## Exemplo mínimo de uso

```rust
use support_crypto::{
    decrypt_text_with_key, encrypt_text_with_key, KeyId, SecretKey, StorageAad,
};

fn example() -> Result<(), support_crypto::CryptoError> {
    let key = SecretKey::new([7; support_crypto::KEY_LENGTH_BYTES]);
    let key_id = KeyId::new("local-main-v1")?;
    let aad = StorageAad::new("documents", "doc-1", "body")?;
    let aad_bytes = aad.aad_bytes();
    let payload = encrypt_text_with_key("segredo", &key, Some(&aad_bytes), Some(&key_id))?;
    let text = decrypt_text_with_key(&payload, &key, Some(&aad_bytes))?;

    assert_eq!(text, "segredo");
    Ok(())
}
```

## Estrutura

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

## Testes

Correr `cargo test -p support-crypto`.

## Versionamento

Esta biblioteca segue SemVer.

## Notas

- Biblioteca agnóstica de UI/UX
- Pensada para dados em repouso

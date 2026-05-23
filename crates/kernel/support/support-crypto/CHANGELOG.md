# CHANGELOG

## [Unreleased]

### Added

- crate `support-crypto`
- derivação de chave com `Argon2id`
- cifragem autenticada com `XChaCha20-Poly1305`
- payload versionado e serializável
- testes de roundtrip, adulteração e AAD
- `MAN.md`
- modularização interna em `aad`, `constants`, `crypto`, `error`, `key`,
  `payload` e `policy`
- `KeyId`, `KeyResolver` e `StaticKeyProvider`
- cifragem/decifragem com chave externa via `encrypt_*_with_key` e
  `decrypt_*_with_key`
- `StorageEnvelope` e `CryptoPolicy::storage_default()`
- erro canonico `MINI.CRYPTO.POLICY_VIOLATION`

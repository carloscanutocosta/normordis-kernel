# ADR-NK-004 — Escritas cifradas em storage SQLite

Estado: Aceite  
Âmbito: normordis-kernel · Storage · Criptografia · Exigência legal  
Autor: Carlos Costa  
Data: 2026-05-11  
Versão: v0.1.0  
Origem: ADR-MINIAPPS-005-encrypted-db-writes (mini-apps-rusty)

---

## Contexto

O `normordis-kernel` persiste dados locais em SQLite através de
`crates/kernel/infra/adapter-sqlite`. Por exigência legal, dados sensíveis
persistidos devem ser cifrados em repouso. Esta regra é um requisito
arquitectural — não uma opção de cada app consumidora.

SQLite fornece persistência robusta mas não decide quais campos são sensíveis
nem aplica cifra. Essas decisões pertencem às camadas com conhecimento do
contrato de storage.

---

## Decisão

As escritas de dados sensíveis devem ser cifradas antes de chegarem ao adapter
concreto de persistência. A arquitectura é:

```
support-storage
  decide envelope de storage e aplica cifragem quando o contrato exigir

support-crypto
  fornece primitive criptográfica, AAD canónico e contratos de chave

adapter-sqlite
  persiste envelopes já cifrados — não decide semântica sensível
```

`adapter-sqlite` mantém responsabilidade de robustez SQLite (writer queue,
transacções, retry/backoff, observabilidade). Não cifra automaticamente todas
as escritas.

---

## Primitivos criptográficos

`crates/kernel/support/support-crypto` é a primitive canónica para:

- cifra autenticada com `XChaCha20-Poly1305`;
- derivação de chave com `Argon2id`;
- envelope `EncryptedPayload` versionado;
- AAD canónico para storage;
- contrato `KeyProvider` para obter a chave actual.

---

## AAD de storage

Formato canónico para dados cifrados em storage:

```
normordis-kernel:v1:storage:<namespace>:<record_id>:<field>
```

Objectivo: impedir troca válida de ciphertext entre namespaces, registos ou
campos. Segmentos não devem ser vazios nem conter whitespace ou `:`.

---

## Key management

`support-crypto` não decide onde guardar chaves. O contrato mínimo:

```rust
pub trait KeyProvider {
    fn current_key(&self) -> Result<SecretKey, MiniError>;
}
```

Implementações concretas vivem no runtime/bootstrap ou em adapters de segredos.
Em Windows, `crates/kernel/infra/secrets` usa DPAPI (`windows-sys`) para
protecção local. Blobs DPAPI não são portáveis entre computadores — migração
entre máquinas requer fluxo explícito de exportação ou recifragem.

Para portabilidade cross-machine, o modelo canónico é `portable-passphrase-v1`:
chave de storage aleatória protegida por passphrase/recovery secret, persistida
como `ProtectedSecret`.

---

## Fronteiras de segurança

Não devem aparecer em erros públicos:

- passphrases, chaves, plaintext, ciphertext, nonce, salt, AAD completo;
- detalhes internos do algoritmo.

Erros de criptografia são convertidos para `MiniError` com mensagens públicas
seguras (ver [ADR-NK-003](ADR-NK-003-support-errors.md)).

---

## Consequências

### Positivas

- cifragem como requisito transversal, não opção ad hoc;
- `adapter-sqlite` livre de semântica de dados sensíveis;
- AAD canónico reduz risco de reutilização indevida de ciphertext.

### Negativas

- disciplina na criação de namespaces, record IDs e campos;
- key management é uma decisão arquitectural explícita;
- migrações necessárias quando dados antigos não cifrados existirem.

---

## Referências

- [ADR-NK-002](ADR-NK-002-crate-layers.md)
- [ADR-NK-003](ADR-NK-003-support-errors.md)
- `crates/kernel/support/support-crypto/MAN.md`
- `security/CRYPTOGRAPHY.md`

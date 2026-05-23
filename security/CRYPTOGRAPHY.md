# Design Criptográfico — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Documentar as decisões de design criptográfico do `normordis-kernel`:
algoritmos escolhidos, justificações, política de zeroização e gestão de
material de chave.

## Princípios

1. **Sem implementações próprias de primitivos.** Toda a criptografia usa
   crates estabelecidas do ecossistema RustCrypto ou equivalente auditado.
2. **Zeroização obrigatória.** Material de chave e dados sensíveis em memória
   são zeroizados via `zeroize` antes de serem libertados.
3. **Aleatoriedade de sistema.** `OsRng` do crate `rand` é a única fonte de
   aleatoriedade; não são usados PRNGs determinísticos para material de chave.
4. **Algoritmos modernos.** Preferência por algoritmos com resistência pós-
   quântica conhecida ou com suporte amplo na indústria.

---

## Algoritmos em uso

### Assinatura digital

| Algoritmo | Crate | Uso |
|-----------|-------|-----|
| Ed25519 | `ed25519-dalek` | Assinatura de documentos, registos de auditoria |
| ECDSA P-256 | `p256` (RustCrypto) | Assinatura com chaves de infra existente |
| ECDSA P-384 | `p384` (RustCrypto) | Assinatura de alta segurança |
| RSA-PSS / RSA-OAEP | `rsa` | Compatibilidade com infra PKI existente |

**Ed25519 é o algoritmo preferido** para novas assinaturas no ecossistema
NORMORDIS. RSA e ECDSA P-256/P-384 mantêm-se por compatibilidade.

### Cifra simétrica

| Algoritmo | Crate | Uso |
|-----------|-------|-----|
| ChaCha20-Poly1305 | `chacha20poly1305` | Cifra autenticada de dados em repouso |

ChaCha20-Poly1305 é preferido a AES-GCM por ser resistente a timing attacks
em plataformas sem instrução AES-NI (ex: ARM sem extensões).

### Hashing

| Algoritmo | Crate | Uso |
|-----------|-------|-----|
| SHA-256 | `sha2` | Integridade de manifests, encadeamento de auditoria |
| SHA-512 | `sha2` | Hashing de alta segurança quando necessário |

### Derivação de chave e passwords

| Algoritmo | Crate | Uso |
|-----------|-------|-----|
| Argon2id | `argon2` | Hash de passwords e derivação de chave de storage |

Argon2id com parâmetros mínimos: memória 64 MiB, iterações 3, paralelismo 1.
Parâmetros configuráveis via `core-config`.

### Segredos em repouso (plataforma)

| Mecanismo | Crate | Plataforma |
|-----------|-------|------------|
| DPAPI (Data Protection API) | `windows-sys` | Windows |
| Ficheiro cifrado com ChaCha20 | `chacha20poly1305` | Linux (fallback) |

A abstracção de storage de segredos está em `infra-secrets`. O comportamento
é seleccionado em compilação via `#[cfg(windows)]`.

---

## Política de zeroização

- Todo o material de chave implementa o trait `Zeroize` (crate `zeroize`).
- Structs com dados sensíveis derivam `ZeroizeOnDrop` quando aplicável.
- Arrays de bytes com segredos são declarados como `Zeroizing<Vec<u8>>` ou
  `Zeroizing<[u8; N]>`.
- Dados sensíveis nunca são clonados desnecessariamente; preferência por
  referências.

---

## Gestão de material de chave

- Chaves privadas nunca são serializadas em plaintext.
- Chaves efémeras são geradas por `OsRng` e zeroizadas após uso.
- Chaves persistentes são cifradas em repouso via `infra-secrets`.
- A duração de vida de chaves segue o princípio de mínima exposição
  (criadas o mais tarde possível, destruídas o mais cedo possível).

---

## Crates não aprovadas

| Crate | Motivo de rejeição |
|-------|-------------------|
| `openssl` | Dependência C externa complexa; substituída por alternativas pure-Rust |
| PRNGs determinísticos para chaves | Não aplicável para material criptográfico |

---

## Revisão

Este documento deve ser revisto:

- A cada adição ou substituição de crate criptográfica
- Antes de qualquer release major
- Após qualquer advisory de segurança nas crates listadas

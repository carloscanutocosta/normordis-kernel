# MAN.md

## Nome

support-backup

## Posicao arquitetural

```text
crates/kernel/support/support-backup
```

Pertence a `kernel/support` porque fornece mecanica de arquivo transversal,
sem logica de dominio, ports ou estado.

## Responsabilidade

Compressao gzip e cifragem autenticada de bytes arbitrarios num formato
de arquivo `.mbak` transportavel. Nao sabe o que cifra — e invocado por
`infra/services/backup`.

## Contrato publico

- `pack(data: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError>`
- `unpack(archive: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError>`
- `BackupError`

## Formato do arquivo .mbak

```text
pack():
  1. gzip(data)                          → bytes comprimidos
  2. XChaCha20Poly1305(compressed, kdf)  → EncryptedPayload
  3. serde_json::to_vec(payload)         → .mbak (JSON em bytes)

unpack():
  1. serde_json::from_slice              → EncryptedPayload
  2. XChaCha20Poly1305 decrypt           → bytes comprimidos
  3. gunzip                              → data original
```

O campo `kdf` usa Argon2id com salt aleatorio por pack. Dois packs do mesmo
conteudo produzem ciphertexts distintos (nonce e salt aleatorios).

## Dependencias de seguranca

- `support-crypto`: XChaCha20Poly1305 + Argon2id (derivacao de chave da passphrase)
- `flate2`: gzip/gunzip

## Erros publicos

- `MINI.BACKUP.COMPRESS_FAILED`
- `MINI.BACKUP.DECOMPRESS_FAILED`
- `MINI.BACKUP.ENCRYPT_FAILED`
- `MINI.BACKUP.DECRYPT_FAILED`

## Limitacoes atuais

- Nao cifra com chave externa (KeyProvider) — apenas passphrase.
- Sem streaming: carrega o conteudo inteiro em memoria.
- Sem versioning do formato .mbak (versao do payload e gerida por support-crypto).

## Ultima revisao

2026-05-21

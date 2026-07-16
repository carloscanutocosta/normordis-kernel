# Regras de Negócio — support/*

Os crates `support/*` definem capacidades técnicas transversais, headless e
reutilizáveis. Não devem conter regras de negócio de aplicações concretas.

---

## Regras gerais

- Identificadores técnicos persistidos devem ser não vazios e sem whitespace.
- Timestamps interoperáveis usam RFC 3339 em UTC com sufixo `Z` obrigatório.
- Hashes SHA-256 usam hexadecimal lowercase com exatamente 64 caracteres.
- Erros públicos não devem transportar paths, queries, segredos ou dados pessoais.
- Artefactos de logging técnico não substituem auditoria institucional.
- Contratos de renderização/conversão descrevem pedidos e referências; a execução
  concreta pertence a adapters/infra.

---

## support-errors — MiniError

### ERR-R01 — Formato do code

`code` deve seguir o padrão `MINI.SEGMENTO[.SEGMENTO]*`:

- Prefixo fixo `MINI.`
- Cada segmento: letras maiúsculas, dígitos, underscore ou ponto
- Exemplos válidos: `MINI.CONFIG.INVALID_APP_PROFILE`, `MINI.AUDIT.CHAIN_BROKEN`
- Exemplos inválidos: `mini.config`, `CONFIG.INVALID`, `MINI.`, `MINI..OK`

Este padrão é partilhado por `LogEvent.code` via `$ref`. Qualquer alteração
ao padrão é um breaking change MAJOR que afecta ambos os tipos.

### ERR-R02 — Sem dados pessoais em details

O campo `details` é livre mas não deve conter informação pessoalmente
identificável (RGPD art.º 5.1.c) — apenas contexto técnico para diagnóstico.

---

## support-ids — TechnicalId

### IDS-R01 — UUID v4 obrigatório

`TechnicalId` é um UUID v4 (RFC 4122). Implementações não aceitam UUID v1,
v3, v5 nem identificadores arbitrários no mesmo campo. Padrão:

```
^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$
```

---

## support-clock — UtcTimestamp

### CLK-R01 — UTC estrito

Timestamps devem terminar em `Z`. Offsets positivos ou negativos (ex: `+01:00`)
são rejeitados mesmo sendo tecnicamente equivalentes à meia-noite UTC.

### CLK-R02 — Precisão de segundos mínima

O formato mínimo aceitável é `YYYY-MM-DDTHH:MM:SSZ`. Fracções de segundo
são permitidas (`2026-01-15T10:00:00.123Z`) mas não obrigatórias.

---

## support-address — PostalCode

### ADDR-R01 — Formato postal português

`cp4` deve ter exactamente 4 dígitos; `cp3` exactamente 3 dígitos.
Zeros à esquerda são significativos e obrigatórios (ex: `"cp4": "1000"`, `"cp3": "001"`).

### ADDR-R02 — Separador opcional

O formato completo do código postal é `CP4-CP3` (ex: `1000-001`). Os campos
são armazenados separados — a concatenação com hífen é responsabilidade da UI.

---

## support-versioning — ReleaseNotes

### VER-R01 — version segue semântica

`version` deve ser uma string não vazia. O formato recomendado é semver
(`MAJOR.MINOR.PATCH`), mas a spec não impõe o padrão para compatibilidade com
versões legadas.

### VER-R02 — Sem mudanças vazias

Uma entrada de release notes deve ter pelo menos uma alteração registada
(campo que captura `changes` ou equivalente não vazio).

---

## support-normalization — NormalizationCase

### NORM-R01 — Operações canónicas

`op` deve ser um dos valores do enum definido no schema. Qualquer operação fora
do enum é rejeitada — implementações não devem inferir operações por semelhança
de nome.

### NORM-R02 — Resultado determinístico

Para o mesmo input e op, o resultado deve ser sempre igual. Implementações não
podem introduzir aleatoriedade no resultado de normalização.

---

## support-storage — StorageKey

### STOR-R01 — Sem traversal de caminho

`StorageKey` não pode conter `..` (subida de directório), `/`, `\`, `:` nem
whitespace. Esta restrição é uma invariante de segurança — não apenas estética.

### STOR-R02 — Charset restrito

Apenas `[A-Za-z0-9_.-]` são permitidos. Nenhum outro caractere é válido mesmo
que seja "inofensivo" — o charset é explicitamente restrito para garantir
portabilidade entre filesystems e stores.

### STOR-R03 — Comprimento máximo

`StorageKey` tem comprimento máximo de 256 caracteres. Chaves mais longas devem
ser recusadas antes de qualquer operação de leitura ou escrita.

---

## support-crypto — EncryptedPayload

### CRYPT-R01 — Algoritmo declarado

O campo `algorithm` identifica o algoritmo de cifra simétrica autenticada.
Actualmente suportado: `XCHACHA20-POLY1305`. Novos algoritmos requerem
incremento MINOR da spec e actualização explícita do enum.

### CRYPT-R02 — Modo external-key

Quando `kdf.algorithm == "external-key"`, a chave é fornecida externamente
(ex: via DPAPI no Windows). Neste modo:

- `kdf.memory_kib`, `kdf.iterations`, `kdf.parallelism` devem ser `0`
- `kdf.salt_b64` pode ser string vazia
- A validação nativa não verifica os parâmetros KDF (são placeholders)

### CRYPT-R03 — Modo argon2id

Quando `kdf.algorithm == "argon2id"`, uma senha é derivada para produzir a
chave. Neste modo, os parâmetros mínimos são:

| Campo | Mínimo | Razão |
|-------|--------|-------|
| `memory_kib` | 64 | Abaixo disto, o Argon2id não oferece resistência a brute-force |
| `iterations` | 1 | — |
| `parallelism` | 1 | — |
| `salt_b64` | não vazio (base64) | Salt vazio anula o propósito do KDF |

### CRYPT-R04 — Nonce e ciphertext em base64

`nonce_b64` e `ciphertext_b64` são strings base64 (standard ou URL-safe).
Implementações devem validar o encoding antes de descodificar.

---

## support-logging — LogEvent

### LOG-R01 — Nível canónico

`level` deve ser um de: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`. Maiúsculas
obrigatórias. Outros valores (ex: `WARNING`, `Fatal`) são rejeitados.

### LOG-R02 — code segue o mesmo padrão que MiniError

Quando presente, `code` segue o padrão `MINI.[A-Z0-9_.]+` (ver ERR-R01).
Um LogEvent com `code` é um registo de erro técnico. Sem `code`, é informação
de diagnóstico geral.

### LOG-R03 — Logging não substitui auditoria

Eventos de logging são efémeros e podem ser rotacionados. Acontecimentos com
relevância de auditoria institucional devem ser registados via `core-audit`,
não via `support-logging`.

---

## support-auth — WebAuthnChallenge

### AUTH-R01 — Expiração obrigatória

Todo o desafio WebAuthn deve ter `expires_at`. Desafios sem expiração não
devem ser emitidos nem aceites.

### AUTH-R02 — Validade do challenge

`value` deve ser um nonce criptográfico não vazio. Implementações devem gerar
o value com pelo menos 16 bytes de entropia (ex: 32 caracteres base64url).

### AUTH-R03 — Associação ao utilizador

`user_id` associa o desafio a um utilizador concreto. Não é permitido emitir
desafios "globais" sem user_id.

---

## support-pdf / support-typst-template / support-docx-to-typst — RenderRequest

### REND-R01 — variables é exclusivo de typst_text

O campo `variables` só é válido quando `kind == "typst_text"`. Para `kind == "pdf"`
ou `kind == "docx_to_typst"`, `variables` não deve estar presente.

### REND-R02 — source não pode ser vazio

`source` deve ser não vazio após trim. Para `pdf` e `typst_text`, é o conteúdo
ou o caminho do template. Para `docx_to_typst`, é o caminho do ficheiro Word.

### REND-R03 — Novos kinds requerem actualização da spec

Qualquer novo tipo de renderização (ex: `html`, `markdown`) requer incremento
MINOR da spec, novo fixture válido e novo fixture inválido antes de ser usado
em produção.

---

## support-backup — BackupArchiveRef

### BAK-R01 — Hash SHA-256 em hexadecimal lowercase

Quando presente, `sha256` deve ser exactamente 64 caracteres hexadecimais
em minúsculas (`[0-9a-f]{64}`). Hashes em maiúsculas ou com prefixo `0x`
são inválidos.

### BAK-R02 — Algoritmo declarado

`algorithm` identifica o método de compressão e cifra do arquivo. Valores
actuais: `tar+gzip`, `tar+gzip+chacha20poly1305`. Novos algoritmos requerem
incremento MINOR da spec.

### BAK-R03 — archive_id sem whitespace

`archive_id` não pode conter whitespace. Deve identificar univocamente o arquivo
dentro do contexto de backup.

# Manual: core-validation

## Contrato público

O crate expõe:

- `ValidationSeverity`: `Info`, `Warning`, `Error`.
- `ValidationIssue`: rule id, campo opcional, severidade e mensagem.
- `ValidationReport`: `valid` e lista de issues.
- `Normalized<T>`: valor `original` e valor `normalized`.
- `ValidationError`: erro próprio convertível para `MiniError`.
- `sha256_bytes(data: &[u8]) -> String`.
- `sha256_file(path) -> Result<String, ValidationError>`.
- `ManifestEntry { path, size, sha256 }`.
- `manifest_file(path) -> Result<ManifestEntry, ValidationError>`.
- Validadores em `validators::{string,email,uuid,nif,iban,json}`.

`ValidationReport::valid` é `true` apenas quando não existe qualquer issue com `ValidationSeverity::Error`.

## Como usar

```rust
use core_validation::validators::{json, string};
use serde_json::json;

let payload = json!({ "id": "abc" });

let mut report = string::required("name", "Alice");
report.merge(json::require_object("payload", &payload));
report.merge(json::require_field("payload", &payload, "id"));

assert!(report.is_valid());
```

```rust
use core_validation::{manifest_file, sha256_bytes};

let digest = sha256_bytes(b"abc");
assert_eq!(
    digest,
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
);
```

## Invariantes e regras

- O crate valida estrutura institucional, não regras substantivas de domínio.
- Normalização não corrige dados silenciosamente: helpers como `normalize_nif` e `normalize_iban` preservam original e normalizado.
- NIF remove whitespace, exige 9 dígitos e valida checksum português.
- IBAN remove whitespace, aplica uppercase, valida forma mínima e MOD 97.
- Email aplica trim explícito para detetar whitespace externo, rejeita espaços e valida apenas forma estrutural.
- UUID usa o crate `uuid`.
- JSON valida apenas objeto e presença de campo.
- `sha256_bytes` é determinístico, usa hexadecimal lowercase e não retorna erro.
- `sha256_file` lê ficheiros em streaming, sem carregar o ficheiro inteiro em memória.
- `manifest_file` aceita apenas ficheiros regulares e produz path normalizado de forma determinística, size de metadata e SHA-256 do conteúdo.
- A camada de integridade não assina digitalmente, não observa diretórios, não lê env vars e não persiste manifests.

## Erros canónicos

Componente: `core-validation`.

- `MINI.VALIDATION.INVALID_INPUT`
- `MINI.VALIDATION.INVALID_RULE`
- `MINI.VALIDATION.NORMALIZATION_FAILED`
- `MINI.VALIDATION.JSON_FAILED`
- `MINI.VALIDATION.OPERATION_FAILED`
- `MINI.VALIDATION.FILE_NOT_FOUND`
- `MINI.VALIDATION.NOT_REGULAR_FILE`
- `MINI.VALIDATION.FILE_READ_FAILED`
- `MINI.VALIDATION.MANIFEST_FAILED`
- `MINI.VALIDATION.HASH_FAILED`

## Limitações atuais

- Não implementa JSON Schema completo.
- Não valida existência real de email, domínio, banco ou conta.
- Não valida regras fiscais substantivas.
- Não persiste reports.
- Não persiste manifests.
- Não assina digitalmente manifests.
- Não integra automaticamente com `core-audit`.
- Ainda não fixa política forense completa para filesystem, symlinks/reparse points e paths não UTF-8.
- Ainda não define convenção canónica para manifests multi-ficheiro.

## Estado produtivo

Estado: production-ready interno/controlado.

Escopo: miniapps desktop.

Reserva: ainda não inclui forense-hardening completo.

O `core-validation` encontra-se apto para uso produtivo interno nas miniapps, no escopo atual de validação estrutural e integridade determinística. A sua utilização em cenários probatórios mais exigentes requer ainda decisões adicionais sobre política de filesystem, symlinks/reparse points, paths não UTF-8 e convenção de manifests multi-ficheiro.

## Roadmap

- `core-validation v0.1 stable`: validação estrutural e integridade determinística para miniapps desktop.
- `core-validation v0.2 filesystem hardening`: ADR curto sobre política de filesystem, decisão sobre symlinks/reparse points, paths não UTF-8 e testes adicionais de ficheiro grande.
- `core-validation v0.3 manifest list`: `ManifestList` e convenção canónica para manifests multi-ficheiro.

## ToDo

- Adicionar validadores estruturais genéricos para coleções e payloads tipados.
- Adicionar helpers de composição com contexto de objeto.
- Avaliar suporte futuro a JSON Schema sem alterar o contrato atual de `ValidationReport`.
- Avaliar convergência futura com `support-crypto` para hashing canónico.

# core-validation

Core de validação estrutural do NORMORDIS Kernel.

## Objetivo

Responder se um artefacto é estruturalmente válido para entrar no sistema, sem decidir regras substantivas de negócio.

## Responsabilidade

- Produzir `ValidationReport` com `ValidationIssue` e severidade.
- Produzir `ValidationResult` como artefacto institucional auditável.
- Validar identificadores portugueses: NIF, NISS, CC, IBAN, código postal, telefone PT.
- Validar identificadores genéricos: email, UUID, semver, MIME type.
- Validar coerência estrutural: intervalos de datas, transições de estado, intervalos numéricos.
- Validar integridade: SHA-256 de bytes e ficheiros, manifesto de ficheiro, `ManifestList`.
- Preservar valor original e normalizado através de `Normalized<T>`.
- Converter `ValidationError` para `MiniError`.

## Não responsabilidade

- Validar regimes fiscais, aprovações, workflow ou elegibilidade de negócio.
- Resolver DNS, confirmar existência bancária real ou persistir reports.
- Assinar digitalmente, observar diretórios, ler env vars, integrar UI, Tauri, SQLite ou `core-audit`.

## Exemplo mínimo

```rust
use core_validation::validators::{nif, iban, string};

let mut report = string::required("nome", "Alice");
report.merge(nif::validate_nif("nif", "501964843"));
report.merge(iban::validate_iban("iban", "PT50 0002 0123 1234 5678 9015 4"));

assert!(report.is_valid());
```

```rust
use core_validation::{ValidationResult, ValidationContext, validators::nif};

let report = nif::validate_nif("nif", "501964843");
let result = ValidationResult::from_report(
    "val_001", "Pessoa", "p_abc",
    Some(ValidationContext::new("2026-06-03T14:00:00Z").with_actor("svc_onboarding")),
    &report,
);
assert!(result.allows_progression());
```

```rust
use core_validation::{sha256_bytes, ManifestList, manifest_file};

let hash = sha256_bytes(b"payload");
let list = ManifestList::from_paths(["/path/to/file.pdf"]);
```

## Documentação completa

Ver [MAN.md](MAN.md) para referência completa de API, COSO, integração com outros cores,
limitações e roadmap.

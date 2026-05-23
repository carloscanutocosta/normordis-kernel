# core-validation

Core de validação estrutural do Mini-Kernel RS.

## Objetivo

Responder se um dado é estruturalmente válido para entrar no sistema, sem decidir regras substantivas de negócio.

## Responsabilidade

- Produzir `ValidationReport` com `ValidationIssue` e severidade.
- Validar forma estrutural mínima de strings obrigatórias, emails, UUIDs, NIF, IBAN e JSON.
- Produzir SHA-256 determinístico de bytes e ficheiros regulares.
- Gerar `ManifestEntry` determinística para ficheiros regulares.
- Preservar valor original e normalizado através de `Normalized<T>` quando existir normalização relevante.
- Converter `ValidationError` para `MiniError`.

## Não responsabilidade

- Validar regimes fiscais, aprovações, workflow ou elegibilidade de negócio.
- Resolver DNS, confirmar existência bancária real ou persistir reports.
- Assinar digitalmente, observar diretórios, ler env vars, integrar UI, Tauri, SQLite ou `core-audit`.
- Persistir manifests.

## Alinhamento com NORMORDIS core-validation

O `core-validation` das miniapps cobre agora duas famílias canónicas:

1. Validação estrutural de dados.
2. Validação probatória de integridade.

Na camada probatória, o crate calcula SHA-256 determinístico em hexadecimal lowercase, lê ficheiros em streaming e gera `ManifestEntry { path, size, sha256 }` para ficheiros regulares.

Esta camada não assina digitalmente, não persiste manifests, não valida regras de negócio e não substitui `core-audit`.

TODO: avaliar convergência futura com `support-crypto` para hashing canónico.

## Estado

O `core-validation` encontra-se apto para uso produtivo interno nas miniapps, no escopo atual de validação estrutural e integridade determinística. A sua utilização em cenários probatórios mais exigentes requer ainda decisões adicionais sobre política de filesystem, symlinks/reparse points, paths não UTF-8 e convenção de manifests multi-ficheiro.

Roadmap de estabilidade:

- `core-validation v0.1 stable`: validação estrutural e integridade determinística para miniapps desktop.
- `core-validation v0.2 filesystem hardening`: política explícita de filesystem, symlinks/reparse points e paths não UTF-8.
- `core-validation v0.3 manifest list`: convenção canónica para manifests multi-ficheiro.

## Exemplo mínimo

```rust
use core_validation::validators::{email, nif};

let mut report = email::validate_email("email", "user@example.com");
report.merge(nif::validate_nif("nif", "501964843"));

assert!(report.is_valid());
```

```rust
use core_validation::{manifest_file, sha256_bytes};

let digest = sha256_bytes(b"abc");
assert_eq!(digest.len(), 64);
```

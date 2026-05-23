# Allowlists e Blocklists — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Registar decisões explícitas de aprovação e rejeição de dependências, licenças,
build scripts e GitHub Actions usados pelo `normordis-kernel`.

## Regras mínimas

- Não existe allowlist permissiva por omissão.
- Cada entrada tem: crate/item, motivo, data, âmbito e responsável.
- Blocklists indicam impacto esperado e alternativa recomendada.
- Excepções são temporárias e rastreáveis; revistas em cada release major.
- A allowlist de licenças formal está em `deny.toml` (secção `[licenses]`).

---

## Crates com `unsafe` aprovadas

Crates que contêm código `unsafe` e são aprovadas pelo projecto:

| Crate | Versão | Motivo | Data |
|-------|--------|--------|------|
| `rusqlite` | 0.31 | Bindings FFI para SQLite (bundled); sem alternativa safe equivalente | 2026-05-23 |
| `windows-sys` | 0.59 | Bindings FFI para Win32/DPAPI; confinado a `#[cfg(windows)]` | 2026-05-23 |
| `ring` | — | Primitivos criptográficos de baixo nível; usado transitivamente | 2026-05-23 |

---

## Build scripts aprovados

Crates com `build.rs` que executam código em tempo de compilação:

| Crate | Motivo |
|-------|--------|
| `rusqlite` | Compilação do SQLite bundled (necessário para portabilidade) |
| `cc` | Compilação de código C (dependência transitiva do SQLite bundled) |

---

## GitHub Actions aprovadas

Actions usadas no CI, pinadas por SHA de commit:

| Action | Versão usada | Motivo |
|--------|-------------|--------|
| `actions/checkout` | `v4` | Checkout do repositório |
| `dtolnay/rust-toolchain` | `stable` | Instalação do toolchain Rust estável |
| `Swatinem/rust-cache` | `v2` | Cache de compilação Rust |

> **Nota:** versões `@latest` ou `@main` são proibidas. Migrar para SHA fixo
> quando a baseline evoluir para Active.

---

## Licenças aprovadas

A lista formal de licenças aprovadas está em `deny.toml`:

```toml
allow = [
  "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
  "CC0-1.0", "Unicode-DFS-2016", "Unicode-3.0",
  "OpenSSL", "BSL-1.0", "MPL-2.0", "EUPL-1.2"
]
```

Excepções de licença por crate estão na secção `[[licenses.exceptions]]` do `deny.toml`.

---

## Blocklist de licenças

| Licença | Motivo |
|---------|--------|
| `GPL-2.0` | Incompatível com distribuição proprietária |
| `GPL-3.0` | Incompatível com distribuição proprietária |
| `AGPL-3.0` | Incompatível com distribuição proprietária |
| `LGPL-2.0` / `LGPL-2.1` / `LGPL-3.0` | Requere cautela; bloqueado por precaução |

---

## Evidência esperada

- PR ou ADR para alterações a esta lista
- Referência ao relatório de auditoria quando aplicável
- Revisão em cada ciclo de release

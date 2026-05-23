# normordis-kernel

Kernel de plataforma do ecossistema NORMORDIS — Rust puro, agnóstico de plataforma (Windows/Linux) e de runtime (desktop/servidor HTTP).

[![CI](https://github.com/carloscanutocosta/normordis-kernel/actions/workflows/ci.yml/badge.svg)](https://github.com/carloscanutocosta/normordis-kernel/actions/workflows/ci.yml)
[![License: EUPL-1.2](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE)

---

## O que é

O `normordis-kernel` é a camada de plataforma partilhada pelas aplicações NORMORDIS. Fornece as capacidades centrais de domínio — gestão de recursos humanos, estrutura orgânica, auditoria, ciclo de vida documental, segurança, configuração e numeração — como uma API Rust unificada, sem depender de Tauri, Node.js ou qualquer framework de apresentação.

A analogia é a de uma SDK de sistema operativo: as aplicações consomem o kernel como biblioteca, o kernel não conhece as aplicações.

## Módulos (via crate de fachada)

| Módulo | Capacidade |
|--------|-----------|
| `normordis_kernel::rh` | Identidade, autenticação e contexto de utilizador |
| `normordis_kernel::org` | Unidades orgânicas, posições e hierarquia |
| `normordis_kernel::audit` | Auditoria append-only com cadeia de hashes verificável |
| `normordis_kernel::documental` | Ciclo de vida documental, templates NDT e eventos |
| `normordis_kernel::validation` | Validadores canónicos: NIF, IBAN, email, UUID |
| `normordis_kernel::security` | Políticas de acesso e autorização |
| `normordis_kernel::config` | Configuração de perfis de deployment |
| `normordis_kernel::metrics` | Métricas de runtime |
| `normordis_kernel::exports` | Exportação de dados e snapshots auditáveis |
| `normordis_kernel::ingest` | Entrada e registo de documentos externos |
| `normordis_kernel::numerador` | Numeração sequencial de documentos institucionais |
| `normordis_kernel::mef` | Classificação orçamental MEF |
| `normordis_kernel::runtime` | Contexto partilhado e ciclo de vida de mini-apps |
| `normordis_kernel::errors` | Tipos de erro partilhados e códigos canónicos |
| `normordis_kernel::ids` | Geração de identificadores únicos (UUID v4) |
| `normordis_kernel::clock` | Abstracção de tempo (testável e determinista) |

## Arquitectura

O workspace segue arquitectura hexagonal estrita em três camadas:

```
crates/
├── kernel/
│   ├── core/      — lógica de domínio e portos (sem I/O)
│   ├── support/   — utilitários transversais (crypto, storage, PDF, logging)
│   └── infra/     — adaptadores (SQLite, PDF, assinatura, scanner)
├── domain/        — domínios transversais (numerador, MEF)
├── runtime/       — contexto e interoperabilidade de mini-apps
└── normordis-kernel/  — fachada pública unificada
```

**Invariante:** `core/` não depende de `infra/`. As dependências fluem sempre de fora para dentro.

## Usar como dependência

O kernel não está publicado em crates.io. Adicionar via dependência Git:

```toml
[dependencies]
normordis-kernel = { git = "https://github.com/carloscanutocosta/normordis-kernel", branch = "main" }
```

Para um commit específico (recomendado em produção):

```toml
[dependencies]
normordis-kernel = { git = "https://github.com/carloscanutocosta/normordis-kernel", rev = "COMMIT_SHA" }
```

## Desenvolvimento

```powershell
# Verificar ambiente e compilar
cargo check --workspace

# Correr testes
cargo test --workspace

# Pipeline completo antes de PR
.\scripts\build-release.ps1

# Portão de segurança antes de release
.\scripts\security\release-gate.ps1
```

Ver [CONTRIBUTING.md](CONTRIBUTING.md) para o processo completo.

## Segurança

Cada release é validada por um portão de segurança automático:

- **`cargo audit`** — sem vulnerabilidades conhecidas (RustSec)
- **`cargo deny`** — conformidade de licenças com EUPL-1.2
- **`MANIFEST.sha256`** — integridade do código-fonte
- **SBOM CycloneDX** — inventário completo de dependências

Ver [SECURITY.md](SECURITY.md) para reportar vulnerabilidades e [security/](security/) para a política completa de confiança.

## Requisitos

| Componente | Versão mínima |
|-----------|--------------|
| Rust | stable (edition 2021) |
| Windows | 11 (plataforma primária) |
| Linux | Ubuntu 22.04+ (CI validado) |

## Licença

[EUPL-1.2](LICENSE) — European Union Public Licence v1.2.

© Carlos Canuto Costa

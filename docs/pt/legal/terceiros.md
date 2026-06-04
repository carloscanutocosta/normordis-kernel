---
title: "Normordis Kernel — Componentes de Terceiros e Licenças"
type: legal
framework: [CRA, ISO27001]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/legal/declaracao-conformidade.md
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/seguranca-informacao.md
---

# Normordis Kernel — Componentes de Terceiros e Licenças

## Propósito

Este documento lista os componentes de software de terceiros integrados no Normordis Kernel, as suas licenças e o seu propósito. Serve como:

- **SBOM legível por humanos** — complemento ao `Cargo.lock` (SBOM técnico completo)
- **Evidência CRA Art. 32.º** — inventário de componentes para avaliação de conformidade
- **Evidência ISO 27001 A.5.14** — gestão de segurança na cadeia de abastecimento
- **Referência de due diligence** para integradores que avaliem o kernel

> **SBOM técnico completo:** o ficheiro `Cargo.lock` na raiz do repositório contém o inventário determinístico de todas as dependências directas e transitivas com versões exactas. O relatório de auditoria `artifacts/trust/audit-report.json` (gerado em cada release) documenta o estado de segurança de cada dependência.

---

## Licença do Projecto

O Normordis Kernel é licenciado sob a **EUPL-1.2** (European Union Public Licence, versão 1.2).

A EUPL-1.2 é uma licença copyleft aprovada pela OSI, compatível com GPL-2.0, GPL-3.0, AGPL-3.0, LGPL, MPL-2.0 e outras conforme o Apêndice da licença. É a licença de referência para software de serviço público europeu.

---

## Política de Licenças de Dependências

A política de licenças está definida em `deny.toml`. São **permitidas** apenas licenças compatíveis com EUPL-1.2:

| Licença permitida | Tipo | Notas |
|------------------|------|-------|
| MIT | Permissiva | Mais comum no ecossistema Rust |
| Apache-2.0 | Permissiva | Inclui concessão de patentes |
| Apache-2.0 WITH LLVM-exception | Permissiva | Variante sem restrição de linkagem |
| BSD-2-Clause | Permissiva | Simplificada |
| BSD-3-Clause | Permissiva | Com cláusula de não-endosso |
| ISC | Permissiva | Equivalente a BSD-2 |
| MIT/Apache-2.0 (dual) | Permissiva | Escolha de uma das duas |
| MPL-2.0 | Copyleft fraco | Compatível com EUPL-1.2 |
| EUPL-1.2 | Copyleft | Licença do projecto |
| CC0-1.0 | Domínio público | Renúncia de direitos |
| Zlib, BSL-1.0, CDLA-Permissive-2.0 | Permissivas | Usos específicos |

São **proibidas** por omissão quaisquer licenças não listadas, incluindo GPL-2.0, GPL-3.0, AGPL-3.0 e LGPL (incompatíveis sem excepção explícita).

---

## Dependências Directas

### Serialização e Tipos Base

| Componente | Versão | Licença | Propósito |
|-----------|--------|---------|-----------|
| `serde` | 1.x | MIT / Apache-2.0 | Serialização e deserialização de estruturas de dados |
| `serde_json` | 1.x | MIT / Apache-2.0 | Serialização JSON — contextos de `AuditEvent`, configuração |
| `thiserror` | 2.x | MIT / Apache-2.0 | Derivação de tipos de erro — `support-errors` |
| `uuid` | 1.x | MIT / Apache-2.0 | Geração de UUIDs v4 — `ActorId`, `support-ids` |
| `chrono` | 0.4.x | MIT / Apache-2.0 | Datas e timestamps — `support-clock`, `AuditEvent` |
| `hex` | 0.4.x | MIT / Apache-2.0 | Codificação hexadecimal — hashes de auditoria |
| `base64` | 0.22.x | MIT / Apache-2.0 | Codificação Base64 — artefactos criptográficos |

### Persistência

| Componente | Versão | Licença | Propósito |
|-----------|--------|---------|-----------|
| `rusqlite` | 0.31.x | MIT | Adapter SQLite — todos os crates `*-sqlite` |
| SQLite (embutido) | 3.x | Domínio público | Motor de base de dados local — embutido via `features = ["bundled"]` |

**Nota SQLite:** o `rusqlite` com a feature `bundled` inclui o código-fonte do SQLite compilado directamente no binário. O SQLite é de domínio público — sem direitos de autor reservados pelo autor original (D. Richard Hipp).

### Runtime Assíncrono

| Componente | Versão | Licença | Propósito |
|-----------|--------|---------|-----------|
| `tokio` | 1.x | MIT | Runtime assíncrono — outbox drainer, pipeline PDF, serviços |

### Criptografia

Todas as dependências criptográficas pertencem ao projecto **RustCrypto** — mantido pela comunidade com revisões de segurança periódicas — excepto `ring`.

| Componente | Versão | Licença | Algoritmo | Propósito | Auditada |
|-----------|--------|---------|-----------|-----------|----------|
| `chacha20poly1305` | 0.10.x | MIT / Apache-2.0 | XChaCha20-Poly1305 | Cifra autenticada em repouso — `support-crypto`, `infra` | Sim |
| `argon2` | 0.5.x | MIT / Apache-2.0 | Argon2id | Derivação de chaves — `support-crypto` | Sim |
| `sha2` | 0.10.x | MIT / Apache-2.0 | SHA-256 / SHA-512 | Hashes — cadeia de auditoria, integridade | Sim |
| `ed25519-dalek` | 2.x | MIT / Apache-2.0 | Ed25519 | Assinaturas digitais — `services/signing` | Sim |
| `rsa` | 0.9.x | MIT / Apache-2.0 | RSA-OAEP / PSS | Verificação de assinaturas RSA — `support-auth` | Sim (¹) |
| `p256` | 0.13.x | MIT / Apache-2.0 | ECDSA P-256 | Assinaturas de curva elíptica | Sim |
| `p384` | 0.13.x | MIT / Apache-2.0 | ECDSA P-384 | Assinaturas de curva elíptica | Sim |
| `subtle` | 2.x | MIT / Apache-2.0 | — | Operações em tempo constante — previne timing attacks | Sim |
| `rand` | 0.8.x | MIT / Apache-2.0 | CSPRNG | Geração de números aleatórios criptograficamente seguros | Sim |
| `zeroize` | 1.x | MIT / Apache-2.0 | — | Limpeza segura de memória — segredos e chaves | Sim |

> ¹ `rsa` v0.9: existe o advisory RUSTSEC-2023-0071 (timing side-channel em operações privadas RSA). O kernel usa `rsa` exclusivamente para **verificação de assinaturas** (operações públicas), não para decifra ou assinatura privada. O risco é aceite e documentado em `deny.toml`.

### Utilitários de Sistema

| Componente | Versão | Licença | Propósito |
|-----------|--------|---------|-----------|
| `flate2` | 1.x | MIT / Apache-2.0 | Compressão gzip/zlib — backups, exportação |
| `tar` | 0.4.x | MIT / Apache-2.0 | Formato de arquivo tar — backups |
| `tempfile` | 3.x | MIT / Apache-2.0 | Ficheiros temporários seguros — operações de I/O |
| `windows-sys` | 0.59.x | MIT / Apache-2.0 | Bindings para API Windows — DPAPI em `secrets` |

---

## Dependências Transitivas com Advisories Activos

O `deny.toml` regista as seguintes excepções a advisories activos, com justificação:

| Advisory | Componente | Origem no grafo | Justificação da aceitação |
|----------|-----------|-----------------|--------------------------|
| RUSTSEC-2023-0071 | `rsa` 0.9.x | `support-auth` | Timing attack em ops. privadas; kernel usa apenas verificação (ops. públicas). Sem caminho de exploração. |
| RUSTSEC-2025-0141 | `bincode` 1.3.3 | Transitivo via Typst/PDF chain | Advisory de manutenção; sem upgrade seguro directo no grafo actual. |
| RUSTSEC-2024-0436 | `paste` | Transitivo via hayagriva/rav1e (Typst) | Advisory de manutenção; sem upgrade seguro directo. |
| RUSTSEC-2024-0320 | `yaml-rust` | Transitivo via syntect (Typst) | Advisory de manutenção; sem upgrade seguro directo. |

Todos os advisories de manutenção (sem CVE de segurança activo) são revistos em cada release. Os três transitivos do pipeline Typst serão resolvidos quando o ecossistema Typst actualizar as suas dependências.

---

## Dependência Especial — Ring

O componente `ring` utiliza uma licença composta (OpenSSL + ISC + MIT) tratada como excepção explícita em `deny.toml`. É uma biblioteca criptográfica amplamente auditada usada como dependência transitiva de alguns crates RustCrypto.

---

## Pipeline Documental — Typst

O pipeline PDF (`crates/kernel/infra/pdf/`) utiliza o **Typst** como motor de renderização. O Typst e as suas dependências introduzem um subgrafo transitivo de dependências (incluindo `rav1e`, `hayagriva`, `syntect`) com as suas próprias licenças, todas verificadas em CI via `cargo deny`.

O Typst é licenciado sob Apache-2.0 (motor) com alguns componentes MIT. A cadeia completa está em `Cargo.lock`.

---

## Verificação Independente

Qualquer integrador pode verificar o inventário de dependências de forma independente:

```powershell
# Listar todas as dependências directas com licenças
cargo license

# Verificar vulnerabilidades
cargo audit

# Verificar política de licenças e advisories
cargo deny check

# Gerar SBOM em formato CycloneDX (quando disponível)
cargo cyclonedx
```

O relatório de auditoria de cada release está em `artifacts/trust/audit-report.json`.

---

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — dependências directas, política de licenças, advisories activos documentados |

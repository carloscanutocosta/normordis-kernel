---
title: "Normordis Kernel — Conformidade CRA e ISO/IEC 27001"
type: compliance
framework: [CRA, ISO27001]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade CRA e ISO/IEC 27001

## Sumário Executivo

O Cyber Resilience Act (CRA, Regulamento (UE) 2024/2847) e a ISO/IEC 27001:2022 são os dois pilares complementares de segurança que enquadram o kernel: o CRA impõe obrigações de produto (segurança por design, SBOM, notificação de vulnerabilidades) durante o ciclo de vida do software; a ISO 27001 define o sistema de gestão que governa como essas obrigações são implementadas e mantidas.

Ambos os frameworks são parcialmente cobertos pelo `nis2.md` — o CRA partilha o foco em supply chain e o ISO 27001 partilha controlos de segurança operacional. Este documento aprofunda os requisitos específicos de cada um e o seu mapeamento ao kernel.

## Âmbito

Cobre os requisitos do CRA e da ISO 27001 aplicáveis ao Normordis Kernel versão `0.1.x`. Não cobre:

- Certificação ISO 27001 formal — decisão do integrador
- Conformidade CRA de produtos construídos sobre o kernel — responsabilidade do fabricante do produto
- Notificações CRA a autoridades — responsabilidade do fabricante

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [CRA] | Regulamento (UE) 2024/2847, de 23 de outubro de 2024 | Art. 13.º–14.º, 19.º, 32.º | Segurança por design, SBOM, notificação de vulnerabilidades |
| [ISO27001] | ISO/IEC 27001:2022 | Cláusulas 4–10, Anexo A | Sistema de gestão da segurança da informação |
| [ISO27002] | ISO/IEC 27002:2022 | Integralmente | Guia de controlos de segurança — referência para Anexo A |
| [ADR-NK-006] | ADR-NK-006 — Trust Baseline v0.1 | Integralmente | Política de confiança verificável contínua do kernel |

---

## Parte I — Cyber Resilience Act (CRA)

### Posicionamento do kernel no CRA

O CRA classifica produtos com elementos digitais em duas categorias: **default** e **críticos** (Classes I e II). O kernel é um componente de software (biblioteca/plataforma) integrado em produtos de terceiros. A responsabilidade primária pela conformidade CRA recai sobre o **fabricante do produto final** que incorpora o kernel.

Contudo, o kernel tem responsabilidades directas enquanto componente da cadeia de abastecimento:

```
Fabricante do produto final (app consumidora)
    │  responsabilidade CRA principal
    │
    ├─► avalia segurança do kernel (Art. 13.3 — due diligence)
    │
    └─► Normordis Kernel
            │  responsabilidades de componente:
            ├─► sem vulnerabilidades conhecidas (Art. 13.1)
            ├─► SBOM disponível (Art. 32)
            ├─► política de gestão de vulnerabilidades (Art. 13.6)
            └─► processo de disclosure (Art. 14)
```

### Art. 13.º — Obrigações dos fabricantes / componentes

| Requisito | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Segurança por design e por defeito | Arquitectura | Cifra sempre activa; sem modo inseguro; invariantes em compilação | Implementado |
| Sem vulnerabilidades conhecidas no momento da colocação | CI pipeline | `cargo audit` bloqueante em CI — CVEs impedem integração | Implementado |
| Política de gestão de vulnerabilidades | `security/DEPENDENCY_POLICY.md` | Critérios formais de aceitação e resposta | Implementado |
| Actualizações de segurança durante período de suporte | CI pipeline + `CHANGELOG` | `cargo audit` contínuo; patches documentados | Implementado |
| Configuração segura por defeito | `core-config` | Invariantes de startup; config inválida rejeitada | Implementado |
| Minimização de superfície de ataque | Arquitectura (ports/adapters) | `core/` sem I/O; infra injectada; sem dependências desnecessárias | Implementado |
| Due diligence sobre componentes de terceiros (Art. 13.3) | `deny.toml`, `ALLOWLISTS.md` | Licenças, advisories, allowlists explícitas | Implementado |

### Art. 14.º — Notificação de vulnerabilidades

| Requisito | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Processo de disclosure público | `SECURITY.md` | Política de reporte, triagem e resposta | Implementado |
| Notificação ENISA (24h para vulnerabilidades exploradas) | — | Processo formal de notificação a autoridades | Planeado |
| Patch e comunicação a utilizadores afectados | `CHANGELOG` + CI | Versões de segurança documentadas | Implementado (base) |

### Art. 32.º — Software Bill of Materials (SBOM)

| Requisito | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| SBOM com componentes de nível superior | `Cargo.lock` | Inventário determinístico e versionado de todas as dependências | Implementado |
| SBOM em formato legível por máquina | `Cargo.lock` (TOML) | Parseable programaticamente; exportável para CycloneDX/SPDX | Implementado (base) |
| SBOM disponível para avaliação por integradores | Repositório público | `Cargo.lock` versionado no git | Implementado |
| SBOM em formato normalizado (CycloneDX / SPDX) | — | Geração automática via `cargo cyclonedx` | Planeado |

---

## Parte II — ISO/IEC 27001:2022

### Mapeamento por Cláusula

#### Cláusula 4 — Contexto da organização

| Requisito | Implementação | Status |
|-----------|---------------|--------|
| Partes interessadas e requisitos | Documentação de conformidade (`docs/pt/`) | Implementado |
| Âmbito do ISMS | `SECURITY.md`, ADRs — âmbito do kernel definido | Implementado |

#### Cláusula 6 — Planeamento (gestão de risco)

| Requisito | Crate / Componente | Implementação | Status |
|-----------|-------------------|---------------|--------|
| Avaliação de risco de segurança | `core-audit` | `ControlOutcome::Failed` — registo de risco materializado | Implementado |
| Avaliação contínua de vulnerabilidades | CI pipeline | `cargo audit` por commit | Implementado |
| Objectivos de segurança documentados | ADRs, `SECURITY.md` | Decisões de segurança formalizadas | Implementado |

#### Cláusula 8 — Operação

| Requisito | Crate / Componente | Implementação | Status |
|-----------|-------------------|---------------|--------|
| Controlo operacional | CI pipeline | Gates obrigatórios — sem integração sem aprovação de qualidade | Implementado |
| Avaliação de risco operacional | `core-audit`, `core-config` | Evidência contínua de controlos | Implementado |
| Gestão de mudanças | `CHANGELOG`, ADRs | Todas as mudanças documentadas e rastreáveis | Implementado |

#### Cláusula 9 — Avaliação do desempenho

| Requisito | Crate / Componente | Implementação | Status |
|-----------|-------------------|---------------|--------|
| Monitorização e medição | `core-metrics`, `domain-telemetry` | KPIs contínuos | Implementado |
| Auditoria interna | `core-audit` | Audit trail imutável para revisão | Implementado |
| Revisão pela gestão | ADRs, CHANGELOG | Histórico de decisões e revisões | Implementado |

### Mapeamento de Controlos — Anexo A (selecção relevante)

**Tema A.5 — Controlos organizacionais**

| Controlo | Descrição | Implementação | Status |
|----------|-----------|---------------|--------|
| A.5.7 | Threat intelligence | `cargo audit` + RustSec advisory database | Implementado |
| A.5.14 | Segurança na cadeia de abastecimento | `DEPENDENCY_POLICY.md`, `deny.toml`, `ALLOWLISTS.md` | Implementado |
| A.5.23 | Segurança no uso de serviços cloud | Execução local; sem serviços cloud obrigatórios | Implementado |
| A.5.24 | Gestão de incidentes de segurança | `core-audit` dead-letter, `SECURITY.md` | Implementado |
| A.5.37 | Procedimentos operacionais documentados | ADRs, `docs/`, `CONTRIBUTING.md` | Implementado |

**Tema A.8 — Controlos tecnológicos**

| Controlo | Descrição | Implementação | Status |
|----------|-----------|---------------|--------|
| A.8.7 | Protecção contra malware | `cargo audit` — CVEs bloqueados em CI | Implementado |
| A.8.8 | Gestão de vulnerabilidades técnicas | `cargo audit`, `cargo deny` — avaliação contínua | Implementado |
| A.8.15 | Logging | `core-audit` — `AuditEvent` imutável e estruturado | Implementado |
| A.8.16 | Monitorização de actividades | `domain-telemetry`, `core-metrics` | Implementado |
| A.8.24 | Uso de criptografia | `support-crypto` — XChaCha20-Poly1305, Argon2id | Implementado |
| A.8.28 | Secure coding | `clippy -D warnings`, `rustfmt`, testes obrigatórios | Implementado |
| A.8.29 | Testes de segurança em desenvolvimento | CI pipeline — testes por commit; `cargo audit` | Implementado |
| A.8.31 | Separação de ambientes | `core-config` — perfis dev/staging/prod distintos | Implementado |

---

## Matriz de Conformidade Consolidada

| Requisito | Crate / Componente | Evidência | Status |
|-----------|-------------------|-----------|--------|
| Segurança por design sem modo inseguro (CRA Art. 13) | Arquitectura, `infra` | Cifra sempre activa; invariantes de compilação | Implementado |
| Sem CVEs conhecidos em CI (CRA Art. 13) | CI pipeline | `cargo audit` bloqueante | Implementado |
| SBOM — Cargo.lock (CRA Art. 32) | `Cargo.lock` | Inventário determinístico versionado | Implementado |
| Política de vulnerabilidades (CRA Art. 13.6) | `DEPENDENCY_POLICY.md` | Critérios formais documentados | Implementado |
| Processo de disclosure (CRA Art. 14) | `SECURITY.md` | Política pública de reporte | Implementado |
| Criptografia estado-da-arte (ISO A.8.24) | `support-crypto` | XChaCha20-Poly1305, Argon2id | Implementado |
| Secure coding (ISO A.8.28) | CI pipeline | `clippy -D warnings`, `rustfmt`, testes | Implementado |
| Logging de segurança (ISO A.8.15) | `core-audit` | `AuditEvent` imutável | Implementado |
| Monitorização (ISO A.8.16) | `core-metrics`, `domain-telemetry` | KPIs e telemetria contínuos | Implementado |
| Supply chain (ISO A.5.14) | `deny.toml`, `ALLOWLISTS.md` | Trust Baseline ADR-NK-006 | Implementado |
| SBOM normalizado CycloneDX/SPDX (CRA Art. 32) | — | `cargo cyclonedx` | Planeado |
| Notificação ENISA (CRA Art. 14) | — | Processo formal | Planeado |

## Limitações e Exclusões Conhecidas

- **Certificação ISO 27001**: o kernel não possui certificação ISO 27001. A decisão e o processo de certificação são responsabilidade do integrador para os seus sistemas.
- **SBOM normalizado**: o `Cargo.lock` constitui o SBOM do kernel mas não está em formato CycloneDX ou SPDX; a geração automática nestes formatos está planeada.
- **Notificação ENISA (Art. 14.º CRA)**: o processo de notificação a autoridades para vulnerabilidades activamente exploradas não está formalizado.
- **Controlos físicos (ISO A.7)**: não aplicáveis ao kernel como software; relevantes para a infraestrutura de deployment do integrador.
- **Gestão de recursos humanos (ISO A.6)**: controlos organizacionais de pessoal são responsabilidade da organização integradora.

## Glossário

| Termo | Definição |
|-------|-----------|
| **CRA** | Cyber Resilience Act — Regulamento (UE) 2024/2847; impõe requisitos de segurança a produtos com elementos digitais |
| **CycloneDX** | Formato de SBOM desenvolvido pela OWASP; suportado pela maioria das ferramentas de segurança |
| **ENISA** | Agência da União Europeia para a Cibersegurança; destinatária de notificações CRA |
| **ISMS** | Information Security Management System — sistema de gestão da segurança da informação (ISO 27001) |
| **ISO 27001** | Norma internacional certificável para sistemas de gestão da segurança da informação |
| **RustSec** | Base de dados de advisories de segurança para Rust; consultada pelo `cargo audit` |
| **SBOM** | Software Bill of Materials — inventário completo de componentes de software e dependências |
| **SPDX** | Software Package Data Exchange — formato de SBOM normalizado pela Linux Foundation |
| **Trust Baseline** | Política formal do kernel (ADR-NK-006) — nível mínimo de confiança verificável para componentes e processos |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — CRA (Arts. 13, 14, 32) e ISO 27001 (cláusulas e Anexo A) |

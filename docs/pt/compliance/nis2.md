---
title: "Normordis Kernel — Conformidade NIS2 e Cibersegurança"
type: compliance
framework: [NIS2, CRA, ISO27001]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/coso.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/seguranca-informacao.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade NIS2 e Cibersegurança

## Declaração de Conformidade NIS2

O Normordis Kernel implementa **security by design** como restrição arquitectural: as medidas de cibersegurança exigidas pelo Art. 21.º da NIS2 não são controlos operacionais adicionados ao sistema — estão embebidas na sua construção. Três invariantes técnicos materializam esta declaração:

1. **Supply chain verificável em cada commit** — `cargo audit` e `cargo deny` executam em CI obrigatório. Nenhuma versão do kernel pode ser integrada com vulnerabilidades conhecidas ou dependências fora de política.

2. **Criptografia de estado da arte por defeito** — toda a persistência usa `XChaCha20-Poly1305` com derivação de chave `Argon2id`. Não existe modo de operação sem cifra.

3. **Evidência de incidente nunca silenciada** — o dead-letter em `core-audit` garante que falhas de entrega de eventos são persistidas, não descartadas. Incidentes de controlo são registados como `ControlOutcome::Failed` e ficam rastreáveis no audit trail imutável.

---

## Sumário Executivo

A Directiva (UE) 2022/2555 (NIS2) impõe medidas de gestão de risco de cibersegurança a entidades essenciais e importantes, com especial ênfase em segurança da cadeia de abastecimento de software, criptografia, controlo de acesso e reporte de incidentes.

O Normordis Kernel posiciona-se na cadeia de abastecimento de software das aplicações da AP que operam sobre ele. O Art. 29.º da NIS2 é explícito: as entidades reguladas devem avaliar e gerir os riscos de cibersegurança introduzidos pelos seus fornecedores e prestadores de serviços de TIC — incluindo plataformas de software como o kernel.

O kernel responde a esta responsabilidade com uma postura de **trust baseline verificável**: cada componente é auditado, cada decisão de segurança está documentada em ADR, e o processo de release impõe gates obrigatórios de qualidade e segurança antes de qualquer integração.

## Âmbito

### Posicionamento do kernel na NIS2

O kernel não é directamente uma "entidade essencial" ou "entidade importante" na acepção da NIS2 — é uma plataforma de software. A sua relevância NIS2 é dupla:

- **Como componente da cadeia de abastecimento** (Art. 29.º): organizações que usam o kernel para sistemas essenciais têm obrigação de avaliar a segurança do kernel como fornecedor de TIC.
- **Como plataforma de suporte técnico**: o kernel fornece os mecanismos técnicos (auditoria, criptografia, gestão de incidentes, continuidade) que as aplicações reguladas necessitam para cumprir o Art. 21.º.

Não cobre:
- Medidas organizacionais NIS2 das entidades integradoras (governança, formação, planos de continuidade)
- Reporte de incidentes à autoridade competente (CNCS) — responsabilidade do operador
- Certificação NIS2 — decisão do integrador

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [NIS2] | Directiva (UE) 2022/2555, de 14 de dezembro de 2022 | Art. 20.º–23.º, 29.º | Governação, medidas de gestão de risco, reporte de incidentes, supply chain |
| [CISC] | Lei n.º 46/2018, de 13 de agosto (em revisão para NIS2) | Art. 3.º, 8.º, 10.º | Transposição nacional NIS1; em actualização para NIS2 |
| [CRA] | Regulamento (UE) 2024/2847 — Cyber Resilience Act | Art. 13.º–14.º, 32.º | Segurança por design, SBOM, notificação de vulnerabilidades |
| [ISO27001] | ISO/IEC 27001:2022 | Cláusulas 4–10, Anexo A | Sistema de gestão da segurança da informação — referência complementar |
| [ADR-NK-006] | ADR-NK-006 — Trust Baseline v0.1 | Integralmente | Política de confiança verificável contínua do kernel |

---

## Mapeamento por Artigo NIS2

### Art. 20.º — Governação

O Art. 20.º responsabiliza os órgãos de gestão pela aprovação e supervisão das medidas de cibersegurança.

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Decisões arquitecturais documentadas | `docs/adr/` | ADRs formalizados, versionados e revistos | Implementado |
| Política de confiança verificável | `ADR-NK-006` | Trust Baseline — política formal de confiança | Implementado |
| Política de dependências | `security/DEPENDENCY_POLICY.md` | Regras explícitas para aprovação de dependências | Implementado |
| Allowlists de componentes | `security/ALLOWLISTS.md` | `unsafe`, dependências especiais — aprovação explícita | Implementado |
| Processo de disclosure | `SECURITY.md` | Política pública de reporte de vulnerabilidades | Implementado |

### Art. 21.º — Medidas de Gestão de Risco de Cibersegurança

#### 21(a) — Análise de risco e políticas de segurança

| Mecanismo | Crate / Componente | Implementação | Status |
|-----------|-------------------|---------------|--------|
| Audit trail contínuo | `core-audit` | `AuditEvent` com `ControlOutcome` — registo de desvios | Implementado |
| Validação de configuração | `core-config` | Invariantes verificados em startup; config inválida rejeitada | Implementado |
| Métricas de risco operacional | `core-metrics`, `domain-telemetry` | Indicadores contínuos de comportamento do sistema | Implementado |
| Avaliação contínua de vulnerabilidades | CI pipeline | `cargo audit` em cada commit — CVEs detectados automaticamente | Implementado |

#### 21(b) — Tratamento de incidentes

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Registo de incidentes de controlo | `core-audit` | `ControlOutcome::Failed` — incidente registado no audit trail | Implementado |
| Dead-letter para falhas de entrega | `core-audit` | Eventos que falham são persistidos, nunca descartados | Implementado |
| Processo de disclosure | `SECURITY.md` | Fluxo formal de reporte, triagem e resposta a vulnerabilidades | Implementado |
| Notificação ao CNCS (Art. 23.º) | — | Mecanismo de reporte externo estruturado | Planeado |

#### 21(c) — Continuidade de negócio e gestão de crises

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Backup controlado | `services/backup` | Serviço de backup auditável com rastreabilidade | Implementado |
| Persistência portável (restauro) | `infra` (SQLite) | SQLite — formato portável, restauro sem dependência de vendor | Implementado |
| Dead-letter (sem perda de evidência em falha) | `core-audit` | Eventos persistidos mesmo em falha parcial do sistema | Implementado |
| Outbox transaccional (atomicidade) | `core-audit` | Estado e evidência persistidos no mesmo commit | Implementado |

#### 21(d) — Segurança da cadeia de abastecimento

Esta é a área de maior maturidade NIS2 do kernel. Ver secção dedicada abaixo.

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Auditoria de vulnerabilidades | CI pipeline | `cargo audit` — base de dados RustSec, bloqueante em CI | Implementado |
| Política de dependências | `cargo deny` + `deny.toml` | Licenças, versões duplicadas, advisories — controlados | Implementado |
| SBOM implícito | `Cargo.lock` versionado | Inventário completo e determinístico de dependências | Implementado |
| Trust Baseline | `ADR-NK-006` | Política formal de confiança verificável contínua | Implementado |
| Allowlists explícitas | `security/ALLOWLISTS.md` | `unsafe` e dependências especiais — aprovação documentada | Implementado |
| Política de dependências formal | `security/DEPENDENCY_POLICY.md` | Critérios de aceitação de novas dependências | Implementado |

#### 21(e) — Segurança na aquisição, desenvolvimento e manutenção de sistemas

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Gates de release obrigatórios | CI pipeline | `fmt + clippy(-D warnings) + test + release` — sem excepções | Implementado |
| Análise estática de segurança | `clippy -D warnings` | Todos os warnings tratados como erros | Implementado |
| Testes obrigatórios por commit | CI pipeline | Suíte completa executada em cada integração (>200 testes) | Implementado |
| Decisões documentadas | `docs/adr/` | Cada decisão arquitectural com raciocínio e alternativas | Implementado |
| Revisão de segurança | `CONTRIBUTING.md` | Processo de revisão de código com foco em segurança | Implementado |

#### 21(f) — Avaliação da eficácia das medidas

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Métricas contínuas | `core-metrics` | KPIs operacionais normalizados | Implementado |
| Telemetria de uso | `domain-telemetry` | Eventos de uso para detecção de anomalias | Implementado |
| CHANGELOG auditado | Raiz | Registo de alterações de segurança por versão | Implementado |
| Avaliação periódica de dependências | CI pipeline | `cargo audit` — execução automática contínua | Implementado |

#### 21(g) — Formação em cibersegurança

Fora do âmbito técnico do kernel. É responsabilidade organizacional do integrador.

#### 21(h) — Recursos humanos, controlo de acesso e gestão de activos

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Controlo de acesso | `core-security` | Políticas de acesso por recurso | Implementado (base) |
| Gestão de utilizadores | `core-rh` | Ciclo de vida auditado de utilizadores | Implementado |
| `ActorId` em todas as operações | `core-audit` | Cada acção tem actor identificado — não repúdio | Implementado |
| Gestão de activos (dependências) | `deny.toml`, SBOM | Inventário de componentes controlado | Implementado |
| Autorização (OrgAuthorizationPort) | — | Identificado como trabalho futuro | Planeado |

#### 21(i) — Autenticação multifactor e comunicações seguras

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Contratos de autenticação | `support-auth` | Abstracção de autenticação e sessão | Implementado |
| Integração CMD / Autenticação.Gov | — | MFA via Chave Móvel Digital (iAP/ARTE) | Planeado |
| Comunicações internas seguras | `support-crypto` | Primitivos criptográficos para comunicação segura | Implementado |

#### 21(j) — Comunicações de voz, vídeo e texto seguras

Fora do âmbito directo do kernel de domínio. Aplicável às aplicações consumidoras.

#### 21(k) — Criptografia e, se adequado, cifra

O kernel usa criptografia de estado da arte em todas as camadas de persistência e gestão de segredos:

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Cifra simétrica (dados em repouso) | `support-crypto` | `XChaCha20-Poly1305` — resistente a nonce reutilizado | Implementado |
| Derivação de chave | `support-crypto` | `Argon2id` — função de derivação resistente a GPU/ASIC | Implementado |
| Gestão de segredos (Windows) | `secrets` | DPAPI — integração com o sistema operativo | Implementado |
| Gestão de segredos (portável) | `secrets` | Fallback portável para Linux e outros | Implementado |
| Secrets nunca em logs | Arquitectura | Abstracção de secrets sem exposição em `support-logging` | Implementado |
| Hash encadeado de audit trail | `core-audit` | Integridade verificável do log — adulteração detectável | Implementado |

### Art. 22.º–23.º — Reporte de Incidentes

O Art. 23.º impõe notificação prévia ao CNCS em 24 horas para incidentes significativos, e notificação completa em 72 horas.

| Mecanismo | Crate / Componente | Implementação | Status |
|-----------|-------------------|---------------|--------|
| Detecção de incidentes de controlo | `core-audit` | `ControlOutcome::Failed` registado no audit trail | Implementado |
| Persistência de falhas | `core-audit` (dead-letter) | Eventos de falha retidos para análise e reporte | Implementado |
| Vulnerabilidades detectadas em CI | `cargo audit` | Alertas automáticos em cada commit | Implementado |
| Processo de disclosure | `SECURITY.md` | Fluxo de reporte a stakeholders internos | Implementado |
| Notificação estruturada ao CNCS | — | Mecanismo de reporte externo formal | Planeado |

### Art. 29.º — Segurança da Cadeia de Abastecimento

O Art. 29.º é o artigo mais relevante para o kernel como componente de software. Exige que as entidades reguladas avaliem e abordem os riscos de cibersegurança dos seus fornecedores e prestadores de serviços de TIC.

---

## Supply Chain Security — Detalhe Técnico

A gestão da cadeia de abastecimento é a área de maior maturidade NIS2 do kernel. A postura é de **confiança verificável contínua**, não de confiança implícita.

### Camadas de verificação

```
Dependência externa
    │
    ├─► cargo audit
    │     Verifica CVEs conhecidos (base RustSec)
    │     Bloqueante em CI — commit rejeitado se CVE presente
    │
    ├─► cargo deny
    │     deny.toml: licenças permitidas, versões duplicadas,
    │     advisories, fontes de registo
    │     Bloqueante em CI
    │
    ├─► Cargo.lock versionado
    │     SBOM implícito — inventário determinístico e auditável
    │     Cada alteração de dependência é rastreável no git
    │
    ├─► security/DEPENDENCY_POLICY.md
    │     Critérios formais de aceitação de novas dependências
    │     (maturidade, mantimento, histórico de segurança)
    │
    └─► security/ALLOWLISTS.md
          unsafe e dependências especiais aprovadas explicitamente
          com justificação documentada
```

### Documentação para avaliação por integradores (Art. 29.º)

Os integradores que precisam de avaliar o kernel como fornecedor de TIC têm à disposição:

| Artefacto | Localização | Propósito |
|-----------|-------------|-----------|
| Este documento | `docs/pt/compliance/nis2.md` | Declaração de conformidade NIS2 |
| Trust Baseline | `ADR-NK-006` | Política formal de confiança |
| Política de dependências | `security/DEPENDENCY_POLICY.md` | Critérios de avaliação de dependências |
| Allowlists | `security/ALLOWLISTS.md` | `unsafe` e excepções aprovadas |
| Política de segurança | `SECURITY.md` | Processo de disclosure e resposta |
| SBOM (Cargo.lock) | Raiz | Inventário completo de dependências |
| CHANGELOG | Raiz | Histórico de alterações de segurança |

---

## Matriz de Conformidade

| Requisito NIS2 | Crate / Componente | Evidência | Status |
|---------------|-------------------|-----------|--------|
| Política de segurança (Art. 21a) | `core-config`, CI | Invariantes, `cargo audit` contínuo | Implementado |
| Audit trail de incidentes (Art. 21b) | `core-audit` | `ControlOutcome::Failed`, dead-letter | Implementado |
| Continuidade e backup (Art. 21c) | `services/backup`, `core-audit` | Backup auditável, outbox atómico | Implementado |
| Supply chain — auditoria CVE (Art. 21d, 29) | CI pipeline | `cargo audit` bloqueante em CI | Implementado |
| Supply chain — política de dependências (Art. 21d) | `deny.toml`, `DEPENDENCY_POLICY.md` | Controlo formal de licenças e advisories | Implementado |
| Supply chain — SBOM (Art. 21d, CRA Art. 32) | `Cargo.lock` | Inventário determinístico e versionado | Implementado |
| Supply chain — Trust Baseline (Art. 21d) | `ADR-NK-006` | Política formal documentada | Implementado |
| Gates de release (Art. 21e) | CI pipeline | `fmt + clippy + test + release` obrigatórios | Implementado |
| Análise estática (Art. 21e) | `clippy -D warnings` | Todos os warnings tratados como erros | Implementado |
| Controlo de acesso (Art. 21h) | `core-security`, `core-rh` | ActorId em todos os eventos; ciclo de vida auditado | Implementado |
| Criptografia — cifra (Art. 21k) | `support-crypto`, `infra` | XChaCha20-Poly1305, Argon2id, SQLite cifrado | Implementado |
| Criptografia — gestão de segredos (Art. 21k) | `secrets` | DPAPI/portável; secrets nunca em logs | Implementado |
| Integridade do audit trail (Art. 21k) | `core-audit` | Hash encadeado verificável | Implementado |
| Detecção de incidentes (Art. 23) | `core-audit` | Dead-letter, `ControlOutcome::Failed` | Implementado |
| Notificação ao CNCS (Art. 23) | — | Mecanismo externo formal | Planeado |
| Autenticação multifactor (Art. 21i) | — | Integração CMD/Autenticação.Gov (iAP/ARTE) | Planeado |
| Autorização (Art. 21h) | — | `OrgAuthorizationPort` identificado | Planeado |

## Limitações e Exclusões Conhecidas

- **Reporte ao CNCS (Art. 23.º)**: o kernel detecta e regista incidentes mas não implementa o mecanismo de notificação estruturada à autoridade competente. É responsabilidade do operador da entidade regulada.
- **Autenticação multifactor (Art. 21.º-i)**: a integração com CMD / Autenticação.Gov para MFA está planeada mas não implementada. O kernel fornece contratos de autenticação (`support-auth`) mas a implementação MFA depende do integrador.
- **Autorização granular (Art. 21.º-h)**: `OrgAuthorizationPort` está identificado como trabalho futuro. O controlo de acesso actual é baseado em estrutura orgânica, não em permissões granulares.
- **Formação (Art. 21.º-g)** e **gestão de crises (Art. 21.º-c)** ao nível organizacional: são responsabilidades da entidade integradora, não do kernel.
- **Lei 46/2018 em revisão**: a transposição nacional da NIS2 está em curso; este documento será revisto após publicação da nova lei.
- **Certificação**: o kernel não possui certificação NIS2 formal. A decisão de certificação é do integrador para os seus sistemas essenciais.

## Glossário

| Termo | Definição |
|-------|-----------|
| **Argon2id** | Função de derivação de chave resistente a ataques de GPU e ASIC; adoptada como padrão pelo NIST e pela comunidade de cibersegurança |
| **CNCS** | Centro Nacional de Cibersegurança — autoridade nacional competente para NIS2 em Portugal |
| **CVE** | Common Vulnerabilities and Exposures — identificador normalizado de vulnerabilidades de segurança |
| **Dead-letter** | Fila de persistência para eventos de auditoria que falharam entrega — incidentes nunca silenciados |
| **DPAPI** | Data Protection API — mecanismo do Windows para cifra de dados ligada à sessão do utilizador ou da máquina |
| **NIS2** | Directiva (UE) 2022/2555 — Network and Information Systems Security; impõe medidas de cibersegurança a entidades essenciais e importantes |
| **RustSec** | Base de dados de advisories de segurança para o ecossistema Rust; consultada pelo `cargo audit` |
| **SBOM** | Software Bill of Materials — inventário de componentes de software; `Cargo.lock` é o SBOM do kernel |
| **Supply chain security** | Gestão dos riscos de cibersegurança introduzidos por fornecedores e componentes de software de terceiros |
| **Trust Baseline** | Política formal do kernel (ADR-NK-006) que define o nível mínimo de confiança verificável para componentes e processos |
| **XChaCha20-Poly1305** | Algoritmo de cifra autenticada (AEAD) de estado da arte; resistente a reutilização de nonce; usado para toda a persistência local do kernel |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — mapeamento completo Art. 21.º, supply chain security, criptografia |

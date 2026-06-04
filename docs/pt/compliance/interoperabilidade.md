---
title: "Normordis Kernel — Conformidade de Interoperabilidade"
type: compliance
framework: [EIF, INTEROPERABLE-EUROPE, iAP, RNID]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade de Interoperabilidade

## Declaração de Conformidade

O Normordis Kernel foi concebido como uma **plataforma de interoperabilidade por design**: a reutilização entre aplicações, a portabilidade de dados e a neutralidade tecnológica são restrições arquitecturais, não características opcionais. O kernel expõe uma API pública unificada e tipada, usa classificações nacionais padrão (MEF, NIF, IBAN), persiste em formatos portáveis (SQLite), e exporta em formatos abertos — alinhando-se com os 12 princípios do EIF e com os requisitos do Interoperable Europe Act.

A existência de um crate dedicado — `interoperability` em `crates/runtime/` — formaliza os contratos de interoperabilidade entre runtimes como um activo de primeira classe do kernel.

---

## Sumário Executivo

A interoperabilidade de sistemas de informação da Administração Pública é hoje uma obrigação legal a dois níveis: europeu, através do Interoperable Europe Act (Regulamento (UE) 2024/903) e do European Interoperability Framework; e nacional, através do ecossistema iAP e do Referencial Nacional de Interoperabilidade de Dados (RNID).

O Normordis Kernel contribui para esta obrigação em três dimensões: **interoperabilidade técnica** (API pública unificada, formatos abertos, contratos tipados), **interoperabilidade semântica** (classificações nacionais padrão como MEF e validadores canónicos como NIF e IBAN) e **interoperabilidade organizacional** (estrutura orgânica modelada em `core-org`, rastreável e partilhável entre sistemas).

A presente documentação mapeia como o kernel satisfaz cada uma destas dimensões e identifica as obrigações pendentes para conformidade plena.

## Âmbito

Cobre o mapeamento entre os requisitos de interoperabilidade europeus e nacionais e a implementação do Normordis Kernel versão `0.1.x`. Não cobre:

- Integração concreta com a iAP — responsabilidade da aplicação consumidora
- Interoperabilidade de sistemas legados — fora do âmbito do kernel
- Interoperabilidade de infraestrutura de rede — nível de operações

---

## Referências Normativas

**Regulamentação Europeia**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [IEA] | Regulamento (UE) 2024/903 — Interoperable Europe Act, de 11 de abril de 2024 | Art. 3.º, 6.º–8.º, 13.º | Avaliação de interoperabilidade, especificações comuns, soluções reutilizáveis, open source |
| [EIF] | European Interoperability Framework v2 (Dec. Exec. (UE) 2017/2010) | Princípios 1–12 | Framework de interoperabilidade para serviços públicos digitais europeus |
| [SDG] | Regulamento (UE) 2018/1724 — Single Digital Gateway | Art. 2.º, 6.º | Acesso digital único a informação e procedimentos administrativos |
| [DGA] | Regulamento (UE) 2022/868 — Data Governance Act | Art. 3.º–5.º, 10.º | Interoperabilidade de dados do sector público |
| [DATA-ACT] | Regulamento (UE) 2023/2854 — Data Act | Art. 23.º–31.º | Interoperabilidade de dados e portabilidade |
| [eIDAS2] | Regulamento (UE) 2024/1183 | Art. 3.º, 11.º | Interoperabilidade de identidade electrónica e serviços de confiança |

**Legislação Nacional (Portugal)**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [TIC-2020] | RCM n.º 41/2018, de 28 de março | Integralmente | Estratégia TIC 2020 — interoperabilidade como pilar da transformação digital da AP |
| [ESPAP] | DL n.º 107/2012, de 18 de maio | Art. 3.º | ESPAP — Entidade de Serviços Partilhados da AP; gestão de plataformas de interoperabilidade |
| [iAP-PORT] | Portaria n.º 195/2018, de 5 de julho | Integralmente | Plataforma de Interoperabilidade da AP (iAP) — catálogo de serviços partilhados |
| [LADA] | Lei n.º 26/2016, de 22 de agosto | Art. 15.º | Dados abertos e reutilização de informação do sector público |

**Normas e Frameworks Técnicos**

| Ref. | Norma | Relevância |
|------|-------|------------|
| [OPENAPI] | OpenAPI Specification 3.x (Linux Foundation) | Especificação de APIs REST interoperáveis |
| [JSON-SCHEMA] | JSON Schema (IETF) | Validação de estruturas de dados interoperáveis |
| [OAUTH2] | RFC 6749 — OAuth 2.0 / OpenID Connect | Interoperabilidade de identidade e autorização |
| [DCAT] | W3C DCAT v3 — Data Catalog Vocabulary | Catalogação de dados para interoperabilidade semântica |
| [SEMICEU] | SEMIC — Semantic Interoperability Community (EU) | Vocabulários e ontologias para serviços públicos europeus |

---

## O Kernel como Plataforma de Interoperabilidade

### Três camadas de interoperabilidade

O EIF define interoperabilidade em quatro camadas: legal, organizacional, semântica e técnica. O kernel endereça directamente as três últimas:

```
Interoperabilidade Legal
    → enquadramento normativo (este documento)

Interoperabilidade Organizacional
    → core-org: estrutura orgânica modelada, partilhável e auditável
    → core-rh:  pessoas e atribuições rastreáveis

Interoperabilidade Semântica
    → domain-mef:       classificação funcional MEF (padrão nacional AP)
    → domain-numerador: numeração documental (padrão AP)
    → core-validation:  NIF, IBAN, e-mail, datas — validadores canónicos nacionais
    → support-address:  moradas normalizadas segundo padrão nacional

Interoperabilidade Técnica
    → normordis-kernel (facade): API pública tipada com namespaces explícitos
    → interoperability (crate):  contratos formais entre runtimes
    → core-exports:              dados exportados em formatos abertos
    → support-versioning:        compatibilidade e migrações de esquema
```

### O crate `interoperability`

O crate `crates/runtime/interoperability` é o contrato formal de interoperabilidade entre runtimes do ecossistema Normordis. Define as interfaces que permitem que diferentes mini-apps partilhem dados, serviços e contexto sem acoplamento directo. É a materialização técnica do princípio de neutralidade tecnológica do EIF.

### Facade como API de interoperabilidade

A facade `normordis-kernel` expõe a plataforma através de namespaces explícitos e estáveis:

```rust
normordis_kernel::rh          // Recursos humanos
normordis_kernel::org         // Estrutura orgânica
normordis_kernel::audit       // Auditoria
normordis_kernel::documental  // Ciclo de vida documental
normordis_kernel::validation  // Validadores canónicos
normordis_kernel::exports     // Exportação de dados
normordis_kernel::numerador   // Numeração sequencial
normordis_kernel::mef         // Classificação funcional MEF
normordis_kernel::runtime     // Contexto de mini-apps
```

Cada namespace é um contrato estável: as apps consumidoras dependem da facade, não dos crates internos. Mudanças internas não quebram a interoperabilidade.

---

## Mapeamento EIF — 12 Princípios

### P1 — Subsidiariedade e proporcionalidade

O kernel opera localmente — não centraliza dados nem decisões desnecessariamente. A autoridade é local e operacional; a consolidação central, quando existir, é externa ao kernel.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Execução local | `miniapp-runtime` | SQLite local, sem dependência de serviços externos | Implementado |
| Autoridade local | Arquitectura | Princípio documentado em ADR-NK-001 | Implementado |

### P2 — Abertura

O kernel usa padrões abertos, publica contratos explícitos e não cria dependências de fornecedor.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| API em padrões abertos | `normordis-kernel` | Contratos Rust públicos, sem proprietary lock-in | Implementado |
| Formatos de exportação abertos | `core-exports` | Exportação estruturada em JSON/CSV/formatos abertos | Implementado |
| Dependências auditadas | CI pipeline | `cargo deny` — sem dependências proprietárias não auditadas | Implementado |
| Open source | Ecossistema | Kernel desenvolvido em Rust (open source) | Implementado |

### P3 — Transparência

Toda a operação é auditável e toda a decisão arquitectural está documentada.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Audit trail completo | `core-audit` | `AuditEvent` por cada operação significativa | Implementado |
| ADRs públicos | `docs/adr/` | Decisões arquitecturais formalizadas e versionadas | Implementado |
| Documentação de API | `docs/architecture/` | Crate-map, overview, contratos públicos | Implementado |
| CHANGELOG versionado | Raiz | Histórico público de alterações | Implementado |

### P4 — Reutilização

O kernel foi concebido explicitamente para reutilização entre múltiplas aplicações do ecossistema Normordis.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Fachada unificada | `normordis-kernel` | API única para múltiplas apps consumidoras | Implementado |
| Crates por capacidade | `core/`, `support/`, `infra/` | Cada crate é reutilizável independentemente | Implementado |
| Contratos de runtime | `interoperability` | Interfaces partilhadas entre runtimes | Implementado |
| Critério formal de criação | `ADR-NK-002` | Novos crates só nascem se reutilizáveis por >1 app | Implementado |

### P5 — Neutralidade tecnológica e portabilidade de dados

O kernel não cria dependência de tecnologia, plataforma ou fornecedor.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Persistência portável | `infra` (SQLite) | SQLite — portável entre Windows, Linux, macOS | Implementado |
| Ports/traits sem I/O | `core/` | Contratos de domínio independentes de infraestrutura | Implementado |
| Exportação de dados | `core-exports` | Dados exportáveis em formatos abertos sem lock-in | Implementado |
| Portabilidade de schema | `support-versioning` | Migrações de esquema versionadas e rastreáveis | Implementado |
| Runtime agnóstico | Arquitectura | Suporta Tauri (desktop) e servidor HTTP | Implementado |

### P6 — Centrado no utilizador

A plataforma suporta aplicações centradas no utilizador através de componentes acessíveis e fluxos documentados.

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Componentes acessíveis | `normordis-core-ui` | WCAG 2.1 AA (ecossistema) | Implementado (ecossistema) |
| Mensagens de erro claras | `support-errors` | `PublicError` estruturado e internacionalizável | Implementado |
| Validação com feedback | `core-validation` | Erros de validação descritivos por campo | Implementado |

### P7 — Inclusão e acessibilidade

Ver [docs/pt/compliance/wcag.md](wcag.md) para mapeamento completo WCAG/A11Y.

| Mecanismo | Componente | Status |
|-----------|-----------|--------|
| WCAG 2.1 AA | `normordis-core-ui` (ecossistema) | Implementado (ecossistema) |
| PDF/A acessível | `normordis-pdf` (ecossistema) | Implementado (ecossistema) |
| Campo `lang` em documentos | `core-documental` | Implementado |

### P8 — Segurança e privacidade

Ver [docs/pt/compliance/rgpd.md](rgpd.md) e [docs/pt/compliance/coso.md](coso.md).

| Mecanismo | Crate | Status |
|-----------|-------|--------|
| Privacy by design | `core-rh` + `core-audit` (ActorId) | Implementado |
| Cifra em repouso | `infra` (SQLite) | Implementado |
| Audit trail de operações | `core-audit` | Implementado |
| Supply chain trust | CI pipeline | Implementado (parcial) |

### P9 — Multilinguismo

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Campo `lang` em documentos | `core-documental` | Língua do documento como metadado obrigatório | Implementado |
| Documentação multilingue | `docs/pt/`, `docs/en/` | Estrutura preparada para PT e EN | Preparado |
| Mensagens de erro internacionalizáveis | `support-errors` | `PublicError` com estrutura internacionalizável | Implementado (base) |

### P10 — Simplificação administrativa

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Numeração automática | `domain-numerador` | Numeração sequencial sem intervenção manual | Implementado |
| Classificação MEF automática | `domain-mef` | Lookup tabular sem digitação manual de códigos | Implementado |
| Validação automática | `core-validation` | NIF, IBAN, e-mail — validação sem consulta externa | Implementado |
| Pipeline documental | `pdf/pdf-pipeline` | Geração automática de PDF/A | Implementado |

### P11 — Preservação de informação

Ver [docs/pt/compliance/arquivistica.md](arquivistica.md) para mapeamento completo.

| Mecanismo | Crate | Status |
|-----------|-------|--------|
| Ciclo de vida documental | `core-documental` | Implementado |
| PDF/A de longa duração | `normordis-pdf` (ecossistema) | Implementado (ecossistema) |
| Classificação arquivística MEF-DGLAB | `domain-mef` | Implementado |
| Audit trail imutável | `core-audit` | Implementado |

### P12 — Avaliação de eficácia e eficiência

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Métricas de runtime | `core-metrics` | Indicadores operacionais normalizados | Implementado |
| Telemetria de uso | `domain-telemetry` | Eventos de uso e estatísticas agregadas | Implementado |
| Audit de controlo | `core-audit` | `ControlOutcome` — eficácia de cada controlo | Implementado |

---

## Interoperable Europe Act — Requisitos Específicos

O Regulamento (UE) 2024/903 é o diploma mais recente e mais exigente. Aplica-se a organismos públicos quando implementam ou modificam substancialmente sistemas de informação para serviços públicos digitais transfronteiriços.

| Artigo | Requisito | Implementação no kernel | Status |
|--------|-----------|------------------------|--------|
| Art. 3.º | Avaliação de interoperabilidade antes de implementar novos sistemas | Documentação de conformidade (este documento) como base de avaliação | Implementado (base) |
| Art. 6.º | Usar especificações comuns quando disponíveis | MEF nacional, NIF/IBAN, OpenAPI, formatos abertos | Implementado |
| Art. 7.º | Usar soluções de interoperabilidade reutilizáveis do repositório europeu | `interoperability` crate; compatibilidade com SEMIC/ISA² | Planeado |
| Art. 8.º | Contribuir para soluções reutilizáveis | Arquitectura do kernel concebida para reutilização entre apps | Implementado |
| Art. 13.º | Promover soluções open source | Kernel desenvolvido em Rust (open source); dependências auditadas | Implementado |
| Art. 18.º | Governação de interoperabilidade | ADRs, CHANGELOG, documentação de conformidade | Implementado |

---

## iAP e Ecossistema Português

A Plataforma de Interoperabilidade da AP (iAP), gerida pela ARTE/ESPAP (ARTE — antiga AMA), fornece serviços partilhados que o kernel e as apps consumidoras devem integrar:

| Serviço iAP | Relevância para o kernel | Crate de integração | Status |
|-------------|--------------------------|--------------------|----|
| Autenticação.Gov (AAF) | Autenticação de utilizadores AP | `support-auth` | Planeado |
| Validação de NIF/NIPC | Verificação de identidade fiscal | `core-validation` (validação local) | Implementado (local) |
| Validação de IBAN | Verificação de contas bancárias | `core-validation` (validação local) | Implementado (local) |
| Moradas (Base de Dados Nacional de Moradas) | Normalização de moradas | `support-address` | Implementado (local) |
| Assinatura Electrónica (CMD/Chave Móvel Digital) | Assinatura qualificada de documentos | `services/signing` | Planeado (integração CMD) |
| Notificações electrónicas | Envio de notificações a cidadãos | — | Planeado |

**Nota:** O kernel implementa validação local de NIF, IBAN e moradas, o que garante operação offline e reduz dependência da iAP. A integração com a iAP para validação em tempo real é uma camada adicional que as apps consumidoras podem activar.

---

## Matriz de Conformidade

| Requisito | Crate / Módulo | Evidência | Status |
|-----------|---------------|-----------|--------|
| API pública tipada e estável (EIF P2, P4) | `normordis-kernel` (facade) | Namespaces explícitos, contratos públicos | Implementado |
| Contratos de interoperabilidade entre runtimes | `interoperability` | Interfaces formais entre mini-apps | Implementado |
| Classificação funcional MEF (semântica nacional) | `domain-mef` | Tabela MEF temporal, padrão AP | Implementado |
| Numeração documental padrão AP | `domain-numerador` | Sequências por tipo, ano, entidade | Implementado |
| Validadores canónicos nacionais (NIF, IBAN, moradas) | `core-validation`, `support-address` | Validadores alinhados com padrões nacionais | Implementado |
| Exportação em formatos abertos (EIF P5) | `core-exports` | Exportação JSON/CSV sem lock-in | Implementado |
| Portabilidade de dados (Data Act Art. 23.º) | `core-exports` | Exportação estruturada de dados do titular | Implementado (base) |
| Neutralidade de plataforma (EIF P5) | Arquitectura (ports/adapters) | `core/` sem dependência de `infra/` | Implementado |
| Preservação de informação (EIF P11) | `core-documental`, `domain-mef` | Ciclo de vida + classificação arquivística | Implementado |
| Audit trail para transparência (EIF P3) | `core-audit` | `AuditEvent` imutável por operação | Implementado |
| Multilinguismo (EIF P9) | `core-documental`, `docs/` | Campo `lang`, estrutura documental PT/EN | Implementado (base) |
| Avaliação de eficácia (EIF P12) | `core-metrics`, `domain-telemetry` | Métricas e telemetria operacionais | Implementado |
| Avaliação de interoperabilidade (IEA Art. 3.º) | Documentação de conformidade | Este documento como base de avaliação | Implementado (base) |
| Especificações comuns IEA (Art. 6.º) | `domain-mef`, `core-validation` | MEF nacional, NIF/IBAN | Implementado |
| Soluções reutilizáveis (IEA Art. 8.º) | Arquitectura do kernel | Reutilização explícita entre apps | Implementado |
| Open source (IEA Art. 13.º) | Ecossistema | Rust, dependências open source auditadas | Implementado |
| Integração iAP — Autenticação.Gov | — | — | Planeado |
| Integração iAP — CMD (assinatura) | `services/signing` | — | Planeado |
| Repositório SEMIC/ISA² (IEA Art. 7.º) | — | — | Planeado |
| RNID — Referencial Nacional de Interoperabilidade | — | — | Planeado |

## Limitações e Exclusões Conhecidas

- **Integração iAP em tempo real**: o kernel implementa validação local de NIF, IBAN e moradas. A integração com os serviços iAP para validação em tempo real (Autenticação.Gov, CMD, BDNA) é responsabilidade das apps consumidoras.
- **RNID**: o Referencial Nacional de Interoperabilidade de Dados não está formalmente mapeado; é trabalho de fase seguinte.
- **SEMIC/ISA²**: o repositório europeu de soluções reutilizáveis (IEA Art. 7.º) ainda não foi consultado para identificar soluções aplicáveis ao kernel.
- **Interoperabilidade legal**: o kernel não implementa verificação de enquadramento legal cross-border; é responsabilidade do integrador para sistemas que operem em múltiplos Estados-Membros.
- **SDG (Single Digital Gateway)**: a conformidade com o Regulamento SDG para serviços digitais transfronteiriços é responsabilidade das apps consumidoras; o kernel fornece a infra-estrutura técnica necessária.

## Glossário

| Termo | Definição |
|-------|-----------|
| **ARTE** | Agência para a Transformação e Modernização do Estado (antiga AMA — Agência para a Modernização Administrativa) — gere a iAP e os serviços partilhados da AP portuguesa |
| **CMD** | Chave Móvel Digital — serviço português de assinatura electrónica qualificada |
| **DCAT** | Data Catalog Vocabulary — vocabulário W3C para catalogação de datasets interoperáveis |
| **EIF** | European Interoperability Framework — quadro europeu de interoperabilidade para serviços públicos digitais |
| **ESPAP** | Entidade de Serviços Partilhados da AP — gestão de plataformas partilhadas, incluindo iAP |
| **iAP** | Plataforma de Interoperabilidade da AP portuguesa — catálogo de serviços partilhados (NIF, IBAN, moradas, autenticação, assinatura) |
| **IEA** | Interoperable Europe Act — Regulamento (UE) 2024/903; impõe avaliação de interoperabilidade e uso de especificações comuns |
| **ISA²** | Interoperability Solutions for Public Administrations — programa europeu de soluções de interoperabilidade |
| **Interoperabilidade organizacional** | Capacidade de organizações cooperarem, apesar de estruturas e processos diferentes |
| **Interoperabilidade semântica** | Capacidade de sistemas interpretarem dados com o mesmo significado — vocabulários, ontologias, classificações |
| **Interoperabilidade técnica** | Capacidade de sistemas comunicarem através de interfaces e protocolos padrão |
| **RNID** | Referencial Nacional de Interoperabilidade de Dados — framework português de dados interoperáveis |
| **SDG** | Single Digital Gateway — portal europeu único de acesso a informação e serviços administrativos |
| **SEMIC** | Semantic Interoperability Community — comunidade europeia de interoperabilidade semântica (vocabulários, ontologias) |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — EIF 12 princípios, Interoperable Europe Act, iAP, crate interoperability |

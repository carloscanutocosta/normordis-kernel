---
title: "Normordis Kernel — Requisitos Legais e Normativos Aplicáveis"
type: legal
framework: [RGPD, NIS2, AI-ACT, eIDAS2, DGA, CRA, DATA-ACT, IEA, SDG, SNC-AP, WCAG, COSO, INTOSAI, ISO27001, EIF, iAP, MEF-DGLAB, MoReq2010, ISO15489, ARQUIVISTICA]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/coso.md
  - docs/pt/compliance/snc-ap.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/wcag.md
  - docs/pt/compliance/arquivistica.md
  - docs/pt/compliance/interoperabilidade.md
  - docs/pt/compliance/intosai.md
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/seguranca-informacao.md
  - docs/pt/compliance/ai-act.md
  - docs/pt/compliance/eidas.md
  - docs/pt/compliance/dga.md
---

# Normordis Kernel — Requisitos Legais e Normativos Aplicáveis

## Sumário Executivo

Este documento cataloga o conjunto de instrumentos legais, regulamentares e normativos que enquadram o desenvolvimento e operação do Normordis Kernel. O kernel foi concebido para servir aplicações do sector público português, pelo que está sujeito tanto ao direito europeu como à legislação nacional, bem como a normas e frameworks internacionais de referência.

O propósito deste catálogo é triplo: serve de referência para decisores na avaliação do risco de conformidade, de guia para equipas técnicas na priorização de implementação, e de evidência para auditores de que os requisitos aplicáveis foram identificados e considerados.

A identificação de um instrumento neste documento não implica conformidade total — o estado de implementação de cada requisito está registado no documento [Visão Geral de Conformidade](../compliance/overview.md).

## Âmbito

Cobre todos os instrumentos aplicáveis ao Normordis Kernel na qualidade de plataforma de suporte a aplicações do sector público português. Exclui:

- Legislação aplicável exclusivamente às aplicações consumidoras do kernel (responsabilidade do integrador)
- Regulamentação sectorial específica de organismos concretos (ex: saúde, educação)
- Contratos e acordos de nível de serviço — estes são instrumentos contratuais, não normativos

---

## 1. Regulamentação Europeia

### 1.1 Protecção de Dados — RGPD

**Instrumento:** Regulamento (UE) 2016/679 do Parlamento Europeu e do Conselho, de 27 de abril de 2016  
**Referência:** [RGPD]  
**Estado:** Em vigor; aplicável directamente em todos os Estados-Membros

**Obrigações relevantes para o kernel:**

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 5.º | Princípios relativos ao tratamento (licitude, minimização, exactidão, limitação) | Validação e filtragem de dados pessoais em `core-validation` |
| Art. 25.º | Protecção de dados desde a concepção e por defeito (*privacy by design*) | Restrição arquitectural de primeira classe |
| Art. 30.º | Registos das actividades de tratamento (RAT) | Rastreabilidade de utilizadores em `core-rh`; RAT dedicado a implementar |
| Art. 32.º | Segurança do tratamento — medidas técnicas adequadas | SQLite com cifra em repouso; gestão de secrets |
| Art. 35.º | Avaliação de impacto sobre a protecção de dados (AIPD) | Obrigação do integrador para sistemas de alto risco |

### 1.2 Cibersegurança — NIS2

**Instrumento:** Directiva (UE) 2022/2555 do Parlamento Europeu e do Conselho, de 14 de dezembro de 2022  
**Referência:** [NIS2]  
**Estado:** Em vigor; prazo de transposição: 17 de outubro de 2024

**Obrigações relevantes para o kernel:**

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 20.º | Governação — responsabilização da gestão pela cibersegurança | `core-org`, estrutura de responsabilidades |
| Art. 21.º | Medidas de gestão de risco (análise de risco, continuidade, cadeia de abastecimento, criptografia) | Gestão de dependências (`deny.toml`, `cargo audit`); SQLite cifrado |
| Art. 22.º–23.º | Reporte de incidentes significativos | Dead-letter em `core-audit`; mecanismo de reporte a implementar |
| Art. 29.º | Segurança da cadeia de abastecimento de software | CI pipeline com SBOM, `cargo deny`, provenance |

### 1.3 Inteligência Artificial — AI Act

**Instrumento:** Regulamento (UE) 2024/1689 do Parlamento Europeu e do Conselho, de 13 de junho de 2024  
**Referência:** [AI-ACT]  
**Estado:** Em vigor; aplicação faseada (sistemas proibidos: fev. 2025; IA de alto risco: ago. 2026)

**Nota de âmbito:** O Normordis Kernel não é classificado como sistema de IA. Contudo, aplicações construídas sobre o kernel podem incorporar sistemas de IA, e o kernel fornece a infra-estrutura de auditoria e rastreabilidade exigida pelo AI Act para esses sistemas.

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 9.º | Sistema de gestão de risco para IA de alto risco | `core-audit` — suporte técnico para evidência de controlo |
| Art. 12.º | Registo de eventos (*logging*) — sistemas IA de alto risco devem manter logs automáticos | `core-audit` — `AuditEvent`, outbox transaccional |
| Art. 13.º | Transparência e fornecimento de informação | Rastreabilidade de decisões via `control_id` e `outcome` |
| Art. 17.º | Sistema de gestão da qualidade para fornecedores de IA de alto risco | Governança do kernel: CI, testes, ADRs, CHANGELOG |
| Art. 72.º–73.º | Supervisão pós-comercialização e reporte de incidentes graves | Dead-letter, audit trail persistente |

### 1.4 Identidade Electrónica — eIDAS 2

**Instrumento:** Regulamento (UE) 2024/1183 do Parlamento Europeu e do Conselho, de 11 de abril de 2024  
**Referência:** [eIDAS2]  
**Estado:** Em vigor; carteira europeia de identidade digital: 2026

**Obrigações relevantes para o kernel:**

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 3.º | Definições — identidade electrónica, assinatura qualificada | Módulo `normordis_kernel::security` |
| Art. 26.º | Requisitos de assinaturas electrónicas qualificadas | Pipeline documental PDF/Typst — assinatura de documentos |

### 1.5 Governação de Dados — Data Governance Act

**Instrumento:** Regulamento (UE) 2022/868 do Parlamento Europeu e do Conselho, de 30 de maio de 2022  
**Referência:** [DGA]  
**Estado:** Em vigor desde setembro de 2023

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 3.º–5.º | Condições de reutilização de dados protegidos do sector público | `domain-registry`; classificação e protecção de dados |
| Art. 10.º | Serviços de intermediação de dados — requisitos de neutralidade | API pública do kernel — ausência de dependência de fornecedor |

### 1.6 Resiliência Cibernética — Cyber Resilience Act

**Instrumento:** Regulamento (UE) 2024/2847 do Parlamento Europeu e do Conselho, de 23 de outubro de 2024  
**Referência:** [CRA]  
**Estado:** Em vigor; aplicação plena: outubro de 2027

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 13.º | Obrigações dos fabricantes — segurança por design, ausência de vulnerabilidades conhecidas | Arquitectura do kernel, `cargo audit` em CI |
| Art. 14.º | Notificação de vulnerabilidades activamente exploradas (ENISA, 24h) | `SECURITY.md`, processo de disclosure |
| Art. 32.º | Software Bill of Materials (SBOM) | CI pipeline; `cargo deny`, `Cargo.lock` versionado |

### 1.7 Portabilidade de Dados — Data Act

**Instrumento:** Regulamento (UE) 2023/2854 do Parlamento Europeu e do Conselho, de 13 de dezembro de 2023  
**Referência:** [DATA-ACT]  
**Estado:** Em vigor desde setembro de 2025

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 5.º | Direito de acesso a dados gerados por produtos e serviços | `normordis_kernel::exports` — exportação de dados |
| Art. 23.º | Portabilidade de dados para o sector público | Formatos abertos; `normordis_kernel::exports` |

### 1.8 Interoperabilidade — Interoperable Europe Act

**Instrumento:** Regulamento (UE) 2024/903 do Parlamento Europeu e do Conselho, de 11 de abril de 2024  
**Referência:** [IEA]  
**Estado:** Em vigor desde abril de 2024; aplicação plena desde abril de 2025

**Nota de âmbito:** Aplica-se a organismos públicos dos Estados-Membros quando implementam ou modificam substancialmente sistemas de informação para serviços públicos digitais cobertos pelo direito da União.

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 3.º | Avaliação de interoperabilidade antes de implementar novos sistemas de informação | Documentação de conformidade do kernel como base de avaliação |
| Art. 6.º | Uso de especificações técnicas e normas comuns quando disponíveis | `domain-mef` (MEF nacional), `core-validation` (NIF, IBAN), OpenAPI |
| Art. 7.º | Uso de soluções reutilizáveis do repositório Interoperable Europe | `interoperability` crate; compatibilidade com ISA²/SEMIC (planeado) |
| Art. 8.º | Contribuição para o repositório de soluções reutilizáveis | Arquitectura do kernel concebida para reutilização entre apps |
| Art. 13.º | Promoção de soluções open source | Kernel em Rust (open source); dependências auditadas |
| Art. 18.º | Governação da interoperabilidade e reporte ao Interoperable Europe Board | ADRs, documentação de conformidade, CHANGELOG |

### 1.9 Acesso Digital Único — Single Digital Gateway

**Instrumento:** Regulamento (UE) 2018/1724 do Parlamento Europeu e do Conselho, de 2 de outubro de 2018  
**Referência:** [SDG]  
**Estado:** Em vigor; totalmente aplicável desde dezembro de 2023

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 2.º | Âmbito — procedimentos administrativos cobertos | Aplicações da AP consumidoras do kernel |
| Art. 6.º | Acesso digital a procedimentos — sem discriminação de utilizadores transfronteiriços | Interoperabilidade da facade API; acessibilidade WCAG |

---

## 2. Legislação Nacional (Portugal)

### 2.1 Contabilidade Pública — SNC-AP

**Instrumento:** Decreto-Lei n.º 192/2015, de 11 de setembro  
**Referência:** [SNC-AP]  
**Estado:** Em vigor; aplicável a todas as entidades das administrações públicas

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 3.º | Âmbito de aplicação — administrações públicas | Público-alvo primário do kernel |
| Art. 4.º | Bases para a normalização contabilística — IPSAS | Módulo SNC-AP a implementar |
| Art. 9.º | Demonstrações financeiras — requisitos de apresentação | Reporte financeiro; integração com `normordis_kernel::documental` |

**Nota:** O SNC-AP baseia-se nas IPSAS (International Public Sector Accounting Standards). A conformidade com o SNC-AP implica alinhamento com os princípios IPSAS relevantes.

### 2.2 Acessibilidade Digital

**Instrumento:** Decreto-Lei n.º 83/2018, de 19 de outubro  
**Referência:** [ACC-DL]  
**Estado:** Em vigor; transpõe a Directiva (UE) 2016/2102

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 2.º | Âmbito — organismos do sector público | Aplicações consumidoras do kernel |
| Art. 3.º | Requisitos de acessibilidade — norma EN 301 549 (referencia WCAG 2.1 AA) | Camada de apresentação das apps integradoras |
| Art. 6.º | Declaração de acessibilidade | Obrigação das apps integradoras |

### 2.3 Cibersegurança Nacional

**Instrumento:** Lei n.º 46/2018, de 13 de agosto (em processo de revisão para transposição NIS2)  
**Referência:** [CISC]  
**Estado:** Em vigor; revisão legislativa em curso para alinhamento com NIS2

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 3.º | Âmbito — operadores de serviços essenciais e fornecedores de serviços digitais | Aplicável se o kernel suportar serviços essenciais |
| Art. 8.º | Medidas de segurança | Alinhamento com requisitos NIS2 (Art. 21.º) |
| Art. 10.º | Notificação de incidentes ao CNCS | Dead-letter, reporte a implementar |

### 2.4 Tribunal de Contas

**Instrumento:** Lei n.º 98/97, de 26 de agosto (Lei de Organização e Processo do Tribunal de Contas)  
**Referência:** [TC]  
**Estado:** Em vigor

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 1.º | Competência de controlo jurisdicional das despesas públicas | Rastreabilidade financeira — `core-audit`, `core-org` |
| Art. 54.º–55.º | Responsabilidade financeira — alcance e desvio | Evidência de controlo como defesa de responsabilidade |

### 2.5 Acesso à Informação

**Instrumento:** Lei n.º 26/2016, de 22 de agosto  
**Referência:** [LADA]  
**Estado:** Em vigor

| Artigo | Obrigação | Relevância |
|--------|-----------|------------|
| Art. 5.º–7.º | Direito de acesso a documentos administrativos e dados abertos | `normordis_kernel::exports`; formatos abertos |
| Art. 15.º | Reutilização de documentos administrativos — formatos abertos | `core-exports`; interoperabilidade de dados |

### 2.6 Interoperabilidade da AP — iAP e ESPAP

**Instrumento principal:** DL n.º 107/2012, de 18 de maio (ESPAP); Portaria n.º 195/2018, de 5 de julho (iAP); RCM n.º 41/2018, de 28 de março (Estratégia TIC 2020)  
**Referência:** [iAP-PORT], [ESPAP], [TIC-2020]  
**Estado:** Em vigor

A Plataforma de Interoperabilidade da AP (iAP), gerida pela ARTE/ESPAP (ARTE — antiga AMA), define os serviços partilhados obrigatórios para sistemas da AP portuguesa. O kernel alinha-se com a iAP através de validadores canónicos locais e contratos de integração.

| Instrumento | Obrigação | Relevância |
|-------------|-----------|------------|
| RCM 41/2018 | Interoperabilidade como pilar estratégico da transformação digital | Arquitectura do kernel orientada à reutilização e abertura |
| DL 107/2012 | ESPAP — plataformas partilhadas de interoperabilidade | `interoperability` crate; integração iAP (planeado) |
| Portaria 195/2018 | Catálogo iAP — NIF/NIPC, IBAN, moradas, autenticação, assinatura | `core-validation`, `support-address`; integração CMD (planeado) |

### 2.7 Arquivística — Gestão de Documentos de Arquivo

**Instrumentos:** DL n.º 447/88, de 10 de dezembro; Portaria n.º 412/2001, de 17 de abril; Portaria n.º 1253/2009, de 14 de outubro  
**Referências:** [ARQ-DL-447], [ARQ-P-412], [ARQ-P-1253]  
**Estado:** Em vigor

| Instrumento | Artigo | Obrigação | Relevância |
|-------------|--------|-----------|------------|
| DL 447/88 | Integralmente | Princípios da política arquivística nacional — conservação, acesso, eliminação controlada | `core-documental`, ciclo de vida documental |
| Portaria 412/2001 | Integralmente | Organização e conservação de arquivos da AP central — planos de classificação, prazos | `domain-mef` (MEF-DGLAB), tabela de selecção (planeado) |
| Portaria 1253/2009 | Integralmente | Gestão de documentos de arquivo para organismos da AP — avaliação, selecção, eliminação | `core-documental`; auto de eliminação (planeado) |

---

## 3. Normas e Frameworks Internacionais

### 3.1 Controlo Interno — COSO 2013

**Instrumento:** COSO Internal Control — Integrated Framework (Committee of Sponsoring Organizations, edição 2013)  
**Referência:** [COSO-2013]  
**Natureza:** Framework de referência internacional; não tem força legal directa mas é adoptado por normas de auditoria pública

O COSO 2013 define 5 componentes e 17 princípios de controlo interno. É o framework central de conformidade do kernel. Ver documento dedicado: [docs/pt/compliance/coso.md](../compliance/coso.md).

### 3.2 Controlo Interno Público — INTOSAI GOV 9100

**Instrumento:** INTOSAI GOV 9100 — Guidelines for Internal Control Standards for the Public Sector  
**Referência:** [INTOSAI]  
**Natureza:** Norma da organização internacional das entidades fiscalizadoras superiores; adoptada pelo Tribunal de Contas

Complementa o COSO para o sector público, com ênfase em responsabilização (*accountability*), transparência e orientação para o interesse público.

### 3.3 Segurança da Informação — ISO/IEC 27001

**Instrumento:** ISO/IEC 27001:2022  
**Referência:** [ISO27001]  
**Natureza:** Norma internacional certificável de sistemas de gestão da segurança da informação

| Cláusula | Requisito | Relevância |
|----------|-----------|------------|
| 4–6 | Contexto, liderança, planeamento | Governança do kernel |
| 8.1 | Planeamento e controlo operacional | `core-config`, CI pipeline |
| Anexo A.5 | Políticas de segurança da informação | `SECURITY.md`, `ADR-NK-006` |
| Anexo A.8 | Controlo de activos | `deny.toml`, SBOM |
| Anexo A.12 | Segurança nas operações — logs, monitorização | `core-audit`, `domain-telemetry` |

### 3.4 Acessibilidade Web — WCAG 2.1

**Instrumento:** W3C Web Content Accessibility Guidelines 2.1 (ISO/IEC 40500:2012)  
**Referência:** [WCAG-21]  
**Nível exigido:** AA (mínimo obrigatório pelo DL 83/2018)

Aplicável à camada de apresentação das aplicações consumidoras. O kernel fornece estruturas de dados e contratos que devem suportar a exposição de conteúdo acessível.

### 3.5 Acessibilidade TIC — EN 301 549

**Instrumento:** EN 301 549 v3.2.1 (2021) — Accessibility requirements for ICT products and services  
**Referência:** [EN301549]  
**Natureza:** Norma europeia harmonizada; referência técnica do DL 83/2018

Cobre requisitos de acessibilidade para software, hardware e serviços de comunicação. As cláusulas 9–11 são as mais relevantes para software.

### 3.6 Interoperabilidade — EIF

**Instrumento:** European Interoperability Framework v2 (Decisão de Execução (UE) 2017/2010)  
**Referência:** [EIF]  
**Natureza:** Framework de referência; adoptado pela Estratégia para a Transformação Digital da AP portuguesa

Os 12 princípios do EIF relevantes para o kernel incluem: subsidiariedade, abertura, reutilização, neutralidade tecnológica, centrado no utilizador e protecção de dados por design.

### 3.7 Contabilidade Pública Internacional — IPSAS

**Instrumento:** International Public Sector Accounting Standards (IPSASB)  
**Referência:** [IPSAS]  
**Natureza:** Normas contabilísticas internacionais; base do SNC-AP

As IPSAS definem os princípios contabilísticos que o SNC-AP adapta para o contexto português. A conformidade com o SNC-AP implica alinhamento com as IPSAS relevantes.

### 3.8 Gestão de Documentos de Arquivo — ISO 15489

**Instrumento:** ISO 15489-1:2016 e ISO 15489-2:2016 — Records Management  
**Referência:** [ISO15489-1]  
**Natureza:** Norma internacional de gestão de documentos de arquivo; adoptada em Portugal via NP 4438

| Secção | Requisito | Relevância |
|--------|-----------|------------|
| §6.3 | Classificação de documentos | `domain-mef` (MEF-DGLAB) |
| §7 | Ciclo de vida e retenção | `core-documental` — estados e log de transições |
| §8 | Metadados de arquivo | `core-documental`; MIP (planeado) |
| §9 | Avaliação e disposição | Tabela de selecção (planeado) |

### 3.9 Requisitos para Sistemas de Arquivo — MoReq2010

**Instrumento:** MoReq2010 — Model Requirements for Records Systems (CECA/DGLAB)  
**Referência:** [MoReq2010]  
**Natureza:** Modelo europeu de requisitos para sistemas de gestão de documentos; adoptado e adaptado pela DGLAB portuguesa

O MoReq2010 define requisitos funcionais e não-funcionais para SEGA (Sistemas Electrónicos de Gestão de Arquivo). O kernel implementa os módulos M2 (classificação), M3 (metadados), M4 (ciclo de vida) e M5 (integridade) como infra-estrutura base.

### 3.10 Classificação Arquivística — MEF-DGLAB

**Instrumento:** Macroestrutura Funcional (DGLAB, edição actualizada)  
**Referência:** [MEF-DGLAB]  
**Natureza:** Plano de classificação funcional de documentos de arquivo da AP portuguesa; publicado e mantido pela DGLAB

O `domain-mef` implementa a MEF-DGLAB com tabela temporal e suporte a diplomas legais, servindo simultaneamente a classificação orçamental (SNC-AP) e arquivística — dois contextos da mesma taxonomia funcional do Estado.

### 3.11 Normas Portuguesas de Arquivo — NP 4041 e NP 4438

**Instrumentos:** NP 4041:2005 (Terminologia Arquivística); NP 4438-1:2005 e NP 4438-2:2006 (Gestão de Documentos de Arquivo)  
**Referência:** [NP4041], [NP4438-1]  
**Natureza:** Normas portuguesas de arquivo; transposição nacional da ISO 15489

Estabelecem a terminologia e os princípios de gestão de documentos de arquivo para organizações portuguesas. Ver documento dedicado: [docs/pt/compliance/arquivistica.md](../compliance/arquivistica.md).

### 3.12 Requisitos para SEGA — eARQ Portugal

**Instrumento:** eARQ Portugal — Especificação de Requisitos para Sistemas de Arquivo Electrónico (DGLAB)  
**Referência:** [eARQ]  
**Natureza:** Especificação técnica portuguesa para sistemas de arquivo electrónico; publicada pela DGLAB

Define os requisitos funcionais e não-funcionais que um sistema de gestão de documentos electrónico deve satisfazer para ser considerado conforme com o quadro arquivístico nacional. A conformidade formal com o eARQ requer validação pela DGLAB.

---

## Matriz de Aplicabilidade ao Kernel

| Instrumento | Aplicabilidade | Afecta kernel directamente | Afecta apps integradoras |
|-------------|---------------|---------------------------|--------------------------|
| RGPD | Alta | Sim — rastreabilidade, cifra, privacy by design | Sim — RAT, consentimento |
| NIS2 | Alta | Sim — supply chain, gestão de risco | Sim — reporte de incidentes |
| AI Act | Média | Sim — suporte técnico para audit trail | Sim — se usarem sistemas IA |
| eIDAS 2 | Média | Sim — módulo de segurança e assinatura | Sim — autenticação |
| DGA | Média | Parcial — classificação de dados | Sim — portais de dados |
| CRA | Alta | Sim — segurança por design, SBOM, vulnerabilidades | Sim — produtos digitais |
| Data Act | Baixa | Parcial — exports | Sim — portabilidade |
| SNC-AP | Alta | Planeado — módulo dedicado | Sim — reporte financeiro |
| DL 83/2018 / WCAG | Baixa | Indirecta — contratos suportam acessibilidade | Alta — camada UI |
| Lei 46/2018 / NIS2 PT | Alta | Sim — alinhamento com NIS2 | Sim |
| Lei 98/97 (TC) | Alta | Sim — `core-audit`, rastreabilidade financeira | Sim |
| COSO 2013 | Alta | Sim — framework central | Sim |
| INTOSAI GOV 9100 | Alta | Sim — complementa COSO para sector público | Sim |
| ISO 27001 | Alta | Sim — parcial | Sim |
| EIF | Média | Sim — design da facade API | Sim |
| IEA (Interoperable Europe Act) | Alta | Sim — avaliação de interoperabilidade, especificações comuns | Sim — serviços públicos digitais |
| iAP / ESPAP | Alta | Sim — validadores locais alinhados; integração iAP planeada | Sim — serviços partilhados AP |
| RCM 41/2018 (TIC 2020) | Média | Sim — arquitectura orientada à interoperabilidade | Sim |
| DL 447/88 + Portarias arquivísticas | Alta | Sim — `core-documental`, ciclo de vida documental | Sim — gestão de arquivo |
| ISO 15489 | Alta | Sim — `core-documental`, metadados, ciclo de vida | Sim |
| MoReq2010 | Alta | Sim — módulos M2–M5 implementados como infra-estrutura | Sim |
| MEF-DGLAB | Alta | Sim — `domain-mef` (classificação arquivística + orçamental) | Sim |
| NP 4438 | Média | Sim — via ISO 15489 + `core-documental` | Sim |
| eARQ Portugal | Alta | Parcial — infra-estrutura disponível; conformidade formal requer validação DGLAB | Sim |
| IPSAS | Média | Planeado — via SNC-AP | Sim |

## Limitações e Exclusões Conhecidas

- **Legislação sectorial**: não cobre regulamentação específica de sectores como saúde (RGPD + regulamentação SNS), educação, ou segurança social.
- **Contratos e acordos**: não cobre obrigações contratuais com entidades específicas.
- **Regulamentação em evolução**: NIS2 (transposição PT em curso), AI Act (aplicação faseada até 2027), CRA (aplicação plena 2027), IEA (aplicação plena desde abril 2025) — o presente documento será revisto em conformidade.
- **Certificação**: a identificação de normas como ISO 27001 não implica certificação do kernel. A decisão de certificação é do integrador.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AIPD** | Avaliação de Impacto sobre a Protecção de Dados — obrigação RGPD Art. 35.º para tratamentos de alto risco |
| **AI Act** | Regulamento (UE) 2024/1689 — regula sistemas de IA com base em perfis de risco |
| **CNCS** | Centro Nacional de Cibersegurança — autoridade nacional competente para cibersegurança |
| **CRA** | Cyber Resilience Act — Regulamento (UE) 2024/2847 — resiliência cibernética de produtos digitais |
| **Data Act** | Regulamento (UE) 2023/2854 — portabilidade e acesso a dados |
| **DGA** | Data Governance Act — Regulamento (UE) 2022/868 — governação de dados do sector público |
| **eIDAS 2** | Regulamento (UE) 2024/1183 — identidade electrónica e serviços de confiança |
| **EIF** | European Interoperability Framework — quadro europeu de interoperabilidade |
| **ENISA** | Agência da União Europeia para a Cibersegurança |
| **INTOSAI** | International Organization of Supreme Audit Institutions — emite normas de auditoria pública |
| **IPSAS** | International Public Sector Accounting Standards — normas contabilísticas para o sector público |
| **NIS2** | Directiva (UE) 2022/2555 — cibersegurança de redes e sistemas de informação |
| **RAT** | Registo de Actividades de Tratamento — obrigação RGPD Art. 30.º |
| **RGPD** | Regulamento Geral sobre a Proteção de Dados — Regulamento (UE) 2016/679 |
| **SBOM** | Software Bill of Materials — inventário de componentes de software |
| **SNC-AP** | Sistema de Normalização Contabilística para as Administrações Públicas — DL 192/2015 |
| **WCAG** | Web Content Accessibility Guidelines — norma W3C de acessibilidade (v2.1, nível AA) |
| **ARTE** | Agência para a Transformação e Modernização do Estado (antiga AMA — Agência para a Modernização Administrativa) — gere a iAP e os serviços partilhados da AP portuguesa |
| **eARQ** | Especificação de Requisitos para Sistemas de Arquivo Electrónico — especificação técnica DGLAB para sistemas de gestão de arquivo electrónico |
| **ESPAP** | Entidade de Serviços Partilhados da Administração Pública — gestão de plataformas partilhadas, incluindo iAP |
| **iAP** | Plataforma de Interoperabilidade da AP portuguesa (ARTE/ESPAP) — catálogo de serviços partilhados: NIF, IBAN, moradas, autenticação, assinatura |
| **IEA** | Interoperable Europe Act — Regulamento (UE) 2024/903; obriga a avaliações de interoperabilidade e uso de especificações comuns em sistemas de informação de organismos públicos |
| **MEF-DGLAB** | Macroestrutura Funcional (DGLAB) — plano de classificação funcional de documentos de arquivo da AP; implementado em `domain-mef` |
| **MIP** | Metainformação para Interoperabilidade — esquema de metadados normalizado para documentos de arquivo electrónico (DGLAB) |
| **MoReq2010** | Model Requirements for Records Systems — modelo europeu de requisitos para SEGA; adoptado pela DGLAB |
| **NP 4438** | Norma Portuguesa de Gestão de Documentos de Arquivo — transposição nacional da ISO 15489 |
| **RNID** | Referencial Nacional de Interoperabilidade de Dados — framework português de dados interoperáveis |
| **SDG** | Single Digital Gateway — portal europeu único de acesso digital a informação e procedimentos administrativos |
| **SEGA** | Sistema Electrónico de Gestão de Arquivo — denominação portuguesa para sistema de gestão de documentos de arquivo |
| **SEMIC** | Semantic Interoperability Community — comunidade europeia de interoperabilidade semântica |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — catálogo completo de instrumentos aplicáveis |
| 0.2.0 | 2026-06-03 | carloscanutocosta | Interoperabilidade: IEA (UE) 2024/903, SDG, RCM 41/2018, ESPAP, iAP; arquivística: DL 447/88, Portarias 412/2001 e 1253/2009, ISO 15489, MoReq2010, MEF-DGLAB, NP 4438, eARQ; matriz e glossário expandidos |

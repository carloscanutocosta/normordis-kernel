---
title: "Normordis Kernel — Visão Geral de Conformidade"
type: overview
framework: [COSO, SNC-AP, WCAG, A11Y, RGPD, NIS2, AI-ACT, eIDAS2, DGA, CRA, EIF, IEA, iAP, INTOSAI, MEF-DGLAB, MoReq2010, ISO15489]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/coso.md
  - docs/pt/compliance/snc-ap.md
  - docs/pt/compliance/wcag.md
  - docs/pt/compliance/arquivistica.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/ai-act.md
  - docs/pt/compliance/interoperabilidade.md
  - docs/pt/compliance/eidas.md
  - docs/pt/compliance/seguranca-informacao.md
  - docs/pt/compliance/dga.md
  - docs/pt/compliance/intosai.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Visão Geral de Conformidade

## Sumário Executivo

O Normordis Kernel foi concebido segundo o princípio de **compliance by design**: a conformidade legal e normativa não é uma camada adicionada após a construção do sistema, mas uma restrição arquitectural de primeira classe. Cada módulo do kernel é construído para gerar evidência de controlo verificável, rastreável e auditável desde o momento em que uma operação ocorre.

O kernel endereça múltiplos eixos de conformidade: controlo interno (COSO), reporte financeiro público (SNC-AP), gestão e arquivo de documentos (MEF-DGLAB, MoReq2010, ISO 15489), acessibilidade digital (WCAG 2.1, EN 301 549), protecção de dados (RGPD), cibersegurança e resiliência (NIS2, Cyber Resilience Act), sistemas de inteligência artificial (AI Act), identidade electrónica (eIDAS 2), governação de dados (Data Governance Act) e interoperabilidade (Interoperable Europe Act, EIF, iAP). Todos estes eixos são tratados como restrições de primeira classe na arquitectura do kernel.

O kernel adopta uma visão **document-centric** para a Administração Pública: cada documento produzido ou recebido é um activo institucional com ciclo de vida, classe arquivística e destino final definidos. O `core-documental` e o `domain-mef` são os pilares desta visão — o segundo serve dupla função como classificação funcional orçamental (SNC-AP) e como plano de classificação arquivística (MEF-DGLAB).

A presente documentação serve de referência para decisores, auditores externos e equipas técnicas, e constitui a base do white-paper do projecto Normordis.

## Âmbito

Este documento cobre a estratégia global de conformidade do Normordis Kernel, versão `0.1.x`. Não cobre:

- Implementações específicas de cada framework — ver documentos dedicados em `docs/pt/compliance/`
- Aplicações construídas sobre o kernel — conformidade da camada de apresentação e do negócio é responsabilidade do integrador
- Conformidade de infraestrutura — rede, hosting e operações estão fora do âmbito do kernel
- Custódia institucional central — o kernel tem autoridade local; a revalidação central, quando existir, é uma responsabilidade separada

## Referências Normativas

**Regulamentação Europeia**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [RGPD] | Regulamento (UE) 2016/679 | Art. 5.º, 25.º, 30.º | Protecção de dados pessoais, privacy by design, registos de tratamento |
| [NIS2] | Directiva (UE) 2022/2555 | Art. 20.º–23.º | Cibersegurança, gestão de risco, segurança da cadeia de abastecimento, reporte de incidentes |
| [AI-ACT] | Regulamento (UE) 2024/1689 | Art. 9.º, 12.º–13.º, 17.º | Gestão de risco, rastreabilidade e logs de sistemas IA, transparência, governança |
| [eIDAS2] | Regulamento (UE) 2024/1183 | Art. 3.º, 26.º | Identidade electrónica, assinaturas qualificadas, serviços de confiança |
| [DGA] | Regulamento (UE) 2022/868 | Art. 3.º–5.º, 10.º | Governação de dados do sector público, reutilização de dados protegidos |
| [CRA] | Regulamento (UE) 2024/2847 | Art. 13.º–14.º, 32.º | Resiliência cibernética de produtos com elementos digitais, SBOM, gestão de vulnerabilidades |
| [DATA-ACT] | Regulamento (UE) 2023/2854 | Art. 5.º, 23.º | Portabilidade de dados, acesso a dados gerados por produtos |
| [IEA] | Regulamento (UE) 2024/903 — Interoperable Europe Act, de 11 de abril de 2024 | Art. 3.º, 6.º–8.º, 13.º | Avaliação de interoperabilidade, especificações comuns, soluções reutilizáveis, open source |
| [SDG] | Regulamento (UE) 2018/1724 — Single Digital Gateway | Art. 2.º, 6.º | Acesso digital único a informação e procedimentos administrativos |

**Legislação Nacional (Portugal)**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [SNC-AP] | DL n.º 192/2015, de 11 de setembro | Art. 3.º, 4.º | Sistema de Normalização Contabilística para as Administrações Públicas |
| [ACC-DL] | DL n.º 83/2018, de 19 de outubro | Art. 2.º–6.º | Acessibilidade de sítios web e aplicações móveis de organismos públicos |
| [CISC] | Lei n.º 46/2018, de 13 de agosto | Art. 3.º, 8.º–10.º | Regime jurídico da segurança do ciberespaço (transposição NIS1; em actualização para NIS2) |
| [TC] | Lei n.º 98/97, de 26 de agosto | Art. 1.º, 54.º–55.º | Controlo jurisdicional do Tribunal de Contas, responsabilidade financeira |
| [LADA] | Lei n.º 26/2016, de 22 de agosto | Art. 5.º–7.º | Acesso à informação administrativa e ambiental, transparência |
| [TIC-2020] | RCM n.º 41/2018, de 28 de março | Integralmente | Estratégia TIC 2020 — interoperabilidade como pilar da transformação digital da AP |
| [ESPAP] | DL n.º 107/2012, de 18 de maio | Art. 3.º | ESPAP — gestão de plataformas partilhadas de interoperabilidade (iAP) |
| [iAP-PORT] | Portaria n.º 195/2018, de 5 de julho | Integralmente | Plataforma de Interoperabilidade da AP — catálogo de serviços partilhados |
| [ARQ-DL-447] | DL n.º 447/88, de 10 de dezembro | Integralmente | Princípios da política arquivística nacional para a AP |
| [ARQ-P-412] | Portaria n.º 412/2001, de 17 de abril | Integralmente | Organização e conservação de arquivos da AP central |
| [ARQ-P-1253] | Portaria n.º 1253/2009, de 14 de outubro | Integralmente | Gestão de documentos de arquivo para organismos da AP |

**Normas e Frameworks Internacionais**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [COSO-2013] | COSO Internal Control — Integrated Framework (2013) | Componentes 1–5, 17 Princípios | Controlo interno, gestão de risco, evidência de auditoria |
| [INTOSAI] | INTOSAI GOV 9100 — Guidelines for Internal Control Standards | Secções I–IV | Controlo interno no sector público, complementa COSO |
| [ISO27001] | ISO/IEC 27001:2022 | Cláusulas 4–10, Anexo A | Sistema de gestão da segurança da informação |
| [WCAG-21] | W3C WCAG 2.1 (ISO/IEC 40500:2012) | Critérios de Sucesso nível AA | Acessibilidade de conteúdo web |
| [EN301549] | EN 301 549 v3.2.1 (2021) | Cláusulas 9–11 | Requisitos de acessibilidade para produtos e serviços TIC (norma europeia) |
| [EIF] | European Interoperability Framework (Dec. Exec. (UE) 2017/2010) | Princípios 1–12 | Interoperabilidade de serviços públicos digitais europeus |
| [IPSAS] | International Public Sector Accounting Standards | IPSAS 1–42 | Base das normas contabilísticas SNC-AP |
| [ISO15489-1] | ISO 15489-1:2016 — Records Management | Integralmente | Princípios e requisitos de gestão de documentos de arquivo |
| [MoReq2010] | MoReq2010 — Model Requirements for Records Systems (CECA/DGLAB) | Módulos 1–7 | Modelo europeu de requisitos para sistemas de gestão de documentos |
| [NP4438-1] | NP 4438-1:2005 — Gestão de Documentos de Arquivo | Integralmente | Princípios e requisitos de gestão arquivística |
| [MEF-DGLAB] | Macroestrutura Funcional (DGLAB, edição actualizada) | Integralmente | Classificação funcional de documentos de arquivo da AP |
| [eARQ] | eARQ Portugal — Requisitos para Sistemas de Arquivo Electrónico (DGLAB) | Integralmente | Especificação de requisitos funcionais para sistemas de arquivo electrónico |

## Implementação no Kernel

### Princípio arquitectural

O kernel adopta o modelo de **evidência de controlo embebida**: cada operação significativa gera um evento de auditoria estruturado (`AuditEvent`) com identificador de controlo (`control_id`), resultado (`outcome`) e contexto de execução (`ControlExecution`). Estes eventos são persistidos via *outbox transaccional*, garantindo que nenhuma evidência se perde por falha parcial. Eventos que falham a entrega são retidos em *dead-letter* para reprocessamento.

A persistência é feita em SQLite com cifra em repouso, em execução local — sem dependência de serviços externos.

### Crates e responsabilidades de conformidade

| Crate | Responsabilidade Principal | Frameworks |
|-------|---------------------------|------------|
| `core-audit` | Contrato central de auditoria, `AuditStore`, outbox transaccional, dead-letter | COSO |
| `core-org` | Estrutura organizacional, substituição legal de funções, evidência por posição | COSO, SNC-AP |
| `core-rh` | Gestão de pessoas e atribuições, rastreabilidade de utilizadores | COSO, RGPD |
| `core-config` | Configuração auditável, validação de invariantes de sistema | COSO |
| `core-validation` | Validação canónica de dados de domínio (NIF, IBAN, email, datas) | COSO, RGPD |
| `core-documental` | Ciclo de vida documental, estados, log de eventos, metadados MIP | Arquivística, SNC-AP, eIDAS 2 |
| `core-exports` | Exportação de dados e snapshots auditáveis | Arquivística, DGA, Data Act |
| `core-ingest` | Entrada e registo de documentos externos | Arquivística |
| `core-metrics` | Métricas operacionais de runtime | COSO (monitorização) |
| `domain-mef` | Classificação funcional orçamental (SNC-AP) e arquivística (MEF-DGLAB); tabela temporal com diplomas legais | SNC-AP, Arquivística |
| `domain-numerador` | Numeração sequencial de documentos por série, ano e entidade | SNC-AP, Arquivística |
| `domain-registry` | Registo de domínios com adapter SQLite | COSO |
| `domain-telemetry` | Eventos de uso, estatísticas agregadas | COSO (monitorização) |
| `core-security` | Políticas de acesso e autorização | NIS2, AI Act |
| `support-crypto` | Criptografia: `XChaCha20-Poly1305`, `Argon2id`, primitivos criptográficos | NIS2, CRA, RGPD |
| `secrets` | Gestão de segredos: DPAPI (Windows), fallback portável; nunca exposto em logs | NIS2, RGPD |
| `services/backup` | Backup auditável e controlado — continuidade de negócio | NIS2 |
| `interoperability` | Contratos formais de interoperabilidade entre runtimes | EIF, IEA |

### Modelo de evidência COSO

```
Operação de domínio
  → AuditEvent { control_id, outcome, ControlExecution }
  → Outbox transaccional (atomic commit)
  → AuditStore (SQLite cifrado)
  → Dead-letter (falhas persistidas para reprocessamento)
```

Os cinco componentes COSO são endereçados da seguinte forma:

| Componente COSO | Implementação no kernel |
|----------------|------------------------|
| 1. Ambiente de controlo | `core-org` — estrutura, responsabilidades, substituições legais |
| 2. Avaliação de risco | `core-audit` — registo de outcomes, evidência de desvios |
| 3. Actividades de controlo | Todos os crates com `control_id` e `ControlExecution` |
| 4. Informação e comunicação | `domain-telemetry`, `core-metrics` |
| 5. Monitorização | `domain-telemetry`, outbox drainer, dead-letter |

## Matriz de Conformidade

| Requisito | Crate / Módulo | Evidência | Status |
|-----------|---------------|-----------|--------|
| Registo de eventos de controlo | `core-audit` | `AuditEvent`, `AuditStore` | Implementado |
| Persistência transaccional de evidência | `core-audit` | Outbox + dead-letter | Implementado |
| Estrutura organizacional auditável | `core-org` | `OrgAuditAdapter`, eventos de domínio | Implementado |
| Substituição legal de funções | `core-org` | `PositionKind`, `substitutes` | Implementado |
| Controlo de concorrência optimista | `core-org` | OCC via versão de agregado | Implementado |
| Rastreabilidade de utilizadores e atribuições | `core-rh` | `UserRepository`, `PersonAssignment` | Implementado |
| Validação canónica de dados | `core-validation` | Contratos de validação de domínio | Implementado |
| Configuração auditável | `core-config` | Validação de invariantes, 65 testes | Implementado |
| Métricas de monitorização | `core-metrics` | Métricas de runtime normalizadas | Implementado |
| Reporte financeiro SNC-AP | — | — | Planeado |
| Acessibilidade WCAG 2.1 AA | — | Camada de apresentação (integrador) | Planeado |
| Registos de tratamento RGPD (RAT) | — | — | Planeado |
| Controlo de acesso e autorização | — | `OrgAuthorizationPort` (identificado) | Planeado |
| Supply chain — auditoria CVE (NIS2 Art. 21d, CRA) | CI pipeline | `cargo audit` bloqueante, `cargo deny`, `Cargo.lock` | Implementado |
| Supply chain — política de dependências (NIS2 Art. 29) | `deny.toml`, `DEPENDENCY_POLICY.md` | Trust Baseline ADR-NK-006, allowlists | Implementado |
| Criptografia em repouso (NIS2 Art. 21k, RGPD Art. 32) | `support-crypto`, `infra` | XChaCha20-Poly1305, Argon2id, SQLite cifrado | Implementado |
| Gestão de segredos (NIS2 Art. 21k) | `secrets` | DPAPI/portável; segredos nunca em logs | Implementado |
| Gestão de incidentes — detecção (NIS2 Art. 21b) | `core-audit` | `ControlOutcome::Failed`, dead-letter | Implementado |
| Continuidade — backup (NIS2 Art. 21c) | `services/backup` | Backup auditável e controlado | Implementado |
| Logging imutável para IA (AI Act Art. 12) | `core-audit` | `AuditEvent` atómico, append-only, hash chain | Implementado |
| Actor + timestamp + contexto em logs IA (AI Act Art. 12) | `core-audit` | `ControlExecution` — actor, timestamp, JSON | Implementado |
| Qualidade de dados para IA (AI Act Art. 10) | `core-validation` | Validadores canónicos NIF, IBAN, e-mail, datas | Implementado |
| Supervisão humana — estrutura (AI Act Art. 14) | `core-org` | Hierarquia, `substitutes`, `OrgAuditAdapter` | Implementado |
| Supervisão humana — atribuição (AI Act Art. 14) | `core-rh` | `PersonAssignment` vincula supervisor | Implementado |
| Sistema de qualidade (AI Act Art. 17, NIS2 Art. 21e) | CI pipeline, ADRs | Gates `fmt+clippy+test+release`, >200 testes | Implementado |
| Segurança da informação (ISO 27001) | `support-crypto`, `infra`, CI | XChaCha20, Argon2id, SQLite cifrado, supply chain | Implementado (parcial) |
| Notificação de incidentes ao CNCS (NIS2 Art. 23) | — | Mecanismo externo formal | Planeado |
| Autenticação multifactor (NIS2 Art. 21i) | — | Integração CMD/Autenticação.Gov (iAP/ARTE) | Planeado |
| Reporte de incidentes graves (AI Act Art. 73) | — | Mecanismo externo formal | Planeado |
| Interoperabilidade técnica — API (EIF P2, P4) | `normordis-kernel` (facade), `interoperability` | Namespaces explícitos, contratos entre runtimes | Implementado |
| Interoperabilidade semântica nacional (EIF P5) | `domain-mef`, `core-validation`, `support-address` | MEF, NIF, IBAN, moradas — padrões nacionais AP | Implementado |
| Avaliação de interoperabilidade (IEA Art. 3.º) | Documentação de conformidade | Este conjunto de documentos como base de avaliação | Implementado (base) |
| Especificações comuns IEA (Art. 6.º) | `domain-mef`, `core-validation` | MEF nacional, validadores canónicos | Implementado |
| Integração iAP — autenticação e assinatura | — | Autenticação.Gov, CMD | Planeado |
| Reporte de incidentes (NIS2) | — | — | Planeado |
| Identidade electrónica (eIDAS 2) | — | — | Planeado |
| Governação de dados (DGA) | — | — | Planeado |
| Portabilidade de dados (Data Act) | — | — | Planeado |
| Ciclo de vida documental (Arquivística) | `core-documental` | Estados + log de transições de documento | Implementado |
| Classificação arquivística MEF-DGLAB | `domain-mef` | Tabela MEF temporal com diplomas legais | Implementado |
| Numeração de registo de documentos | `domain-numerador` | Sequências por série, ano e entidade | Implementado |
| Ingestão e registo de documentos externos | `core-ingest`, `adapter-scanner` | Registo de entrada com metadados | Implementado |
| Geração de PDF/A institucional | `normordis-pdf` (ecossistema) | PDF/A com tags de acessibilidade via Typst | Implementado (ecossistema) |
| Acessibilidade WCAG 2.1 AA — componentes UI | `normordis-core-ui` (ecossistema) | Componentes acessíveis | Implementado (ecossistema) |
| Tabela de selecção e prazos de retenção | — | — | Planeado |
| Metadados MIP completos | — | — | Planeado |
| Auto de eliminação auditado | — | — | Planeado |

## Limitações e Exclusões Conhecidas

- **SNC-AP**: ainda não implementado ao nível do kernel; planeado para fase seguinte.
- **WCAG / A11Y**: implementado via `normordis-core-ui` (projecto do ecossistema Normordis) para componentes de interface, e via `normordis-pdf` para documentos PDF/A acessíveis. O kernel contribui com semântica de dados (língua, títulos, mensagens de erro estruturadas).
- **RGPD — Registo de Actividades de Tratamento (RAT)**: o kernel gera eventos auditáveis com rastreabilidade de utilizadores, mas não implementa um RAT dedicado.
- **Autorização**: o kernel não implementa controlo de acesso; `OrgAuthorizationPort` está identificado como trabalho futuro.
- **Custódia central**: o kernel tem autoridade local e operacional; revalidação e consolidação central, quando existir, é uma responsabilidade externa ao kernel.
- **NIS2 — notificação ao CNCS (Art. 23.º)**: o kernel detecta e regista incidentes mas não implementa notificação estruturada à autoridade competente. Responsabilidade do operador da entidade regulada. A Lei 46/2018 está em revisão para transposição da NIS2.
- **AI Act — posicionamento**: o kernel não é um sistema de IA. Fornece a infra-estrutura de conformidade (logging, supervisão humana, qualidade de dados) para sistemas de IA construídos sobre ele. Ver [docs/pt/compliance/ai-act.md](ai-act.md).
- **AI Act — reporte de incidentes graves (Art. 73.º)**: planeado; responsabilidade primária do fornecedor do sistema de IA.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AuditEvent** | Evento estruturado gerado por cada operação significativa, contendo `control_id`, `outcome` e `ControlExecution` |
| **Compliance by design** | Abordagem arquitectural em que a conformidade legal e normativa é uma restrição de primeira classe, não um requisito posterior |
| **control_id** | Identificador que liga um evento de auditoria a um controlo COSO específico |
| **ControlExecution** | Registo de contexto de execução de um controlo (quem, quando, com que resultado) |
| **COSO** | Committee of Sponsoring Organizations — framework de controlo interno e gestão de risco (edição 2013) |
| **Dead-letter** | Mecanismo de persistência de eventos de outbox que falharam, evitando perda de evidência |
| **Facade** | Módulo `normordis-kernel` que expõe a API pública unificada do kernel às aplicações consumidoras |
| **OCC** | Optimistic Concurrency Control — controlo de concorrência optimista via versão de agregado |
| **Outbox transaccional** | Padrão que garante a publicação de eventos apenas após o commit da transacção de domínio, assegurando consistência entre estado e evidência |
| **AI Act** | Regulamento (UE) 2024/1689 — regula sistemas de inteligência artificial com base em risco; impõe rastreabilidade, transparência e governança para sistemas de IA de alto risco |
| **CRA** | Cyber Resilience Act — Regulamento (UE) 2024/2847 — requisitos de cibersegurança para produtos com elementos digitais, incluindo SBOM e gestão de vulnerabilidades |
| **Data Act** | Regulamento (UE) 2023/2854 — portabilidade de dados e acesso a dados gerados por produtos e serviços digitais |
| **DGA** | Data Governance Act — Regulamento (UE) 2022/868 — governação de dados do sector público e reutilização de dados protegidos |
| **eIDAS 2** | Regulamento (UE) 2024/1183 — identidade electrónica europeia, assinaturas qualificadas e serviços de confiança |
| **EIF** | European Interoperability Framework — quadro europeu de interoperabilidade para serviços públicos digitais |
| **IEA** | Interoperable Europe Act — Regulamento (UE) 2024/903; impõe avaliação de interoperabilidade e uso de especificações comuns em sistemas de informação de organismos públicos |
| **iAP** | Plataforma de Interoperabilidade da AP portuguesa (ARTE/ESPAP) — catálogo de serviços partilhados: NIF, IBAN, moradas, autenticação, assinatura digital |
| **RNID** | Referencial Nacional de Interoperabilidade de Dados — framework português de dados interoperáveis para a AP |
| **SEMIC** | Semantic Interoperability Community — comunidade europeia de interoperabilidade semântica; vocabulários e ontologias para serviços públicos |
| **EN 301 549** | Norma europeia de acessibilidade para produtos e serviços TIC; referência técnica do DL 83/2018 |
| **INTOSAI** | International Organization of Supreme Audit Institutions — emite normas de auditoria pública; GOV 9100 define padrões de controlo interno para o sector público |
| **IPSAS** | International Public Sector Accounting Standards — normas contabilísticas internacionais do sector público; base das normas SNC-AP |
| **NIS2** | Directiva (UE) 2022/2555 — cibersegurança de redes e sistemas de informação; impõe gestão de risco, segurança da cadeia de abastecimento e reporte de incidentes |
| **RAT** | Registo de Actividades de Tratamento — obrigação RGPD (Art. 30.º) de documentar operações de tratamento de dados pessoais |
| **RGPD** | Regulamento Geral sobre a Proteção de Dados — Regulamento (UE) 2016/679 |
| **SBOM** | Software Bill of Materials — inventário de componentes de software; obrigação CRA e boa prática NIS2 |
| **SNC-AP** | Sistema de Normalização Contabilística para as Administrações Públicas — DL n.º 192/2015 |
| **WCAG** | Web Content Accessibility Guidelines — norma W3C de acessibilidade digital (versão 2.1, nível AA) |
| **eARQ** | Especificação de Requisitos para Sistemas de Arquivo Electrónico — publicada pela DGLAB; define requisitos funcionais e não-funcionais para SEGA |
| **MEF-DGLAB** | Macroestrutura Funcional (DGLAB) — plano de classificação funcional de documentos de arquivo da AP; o `domain-mef` serve este propósito em paralelo com o MEF orçamental SNC-AP |
| **MIP** | Metainformação para Interoperabilidade — esquema de metadados normalizados para documentos de arquivo electrónico (DGLAB) |
| **MoReq2010** | Model Requirements for Records Systems — modelo europeu de requisitos para sistemas de gestão de documentos; adoptado pela DGLAB |
| **normordis-core-ui** | Projecto autónomo do ecossistema Normordis que implementa componentes de interface acessíveis (WCAG 2.1 AA) |
| **normordis-pdf** | Projecto autónomo do ecossistema Normordis que gera PDF/A com tags de acessibilidade via pipeline Typst |
| **PDF/A** | Subconjunto do PDF (ISO 19005) normalizado para preservação de longa duração; obrigatório para documentos de arquivo |
| **Tabela de selecção** | Instrumento arquivístico que define, por série documental, os prazos de conservação e destinos finais (conservação permanente ou eliminação) |
| **Argon2id** | Função de derivação de chave resistente a ataques de GPU/ASIC; usada no kernel para derivação de chaves criptográficas (`support-crypto`) |
| **CNCS** | Centro Nacional de Cibersegurança — autoridade nacional competente para NIS2 em Portugal |
| **GPAI** | General Purpose AI — modelo de IA de uso geral (ex: modelos de linguagem); sujeito aos Arts. 53.º–55.º do AI Act |
| **RustSec** | Base de dados de advisories de segurança para o ecossistema Rust; consultada automaticamente pelo `cargo audit` em CI |
| **Trust Baseline** | Política formal do kernel (ADR-NK-006) que define o nível mínimo de confiança verificável para componentes e processos |
| **XChaCha20-Poly1305** | Algoritmo de cifra autenticada (AEAD) de estado da arte; resistente a reutilização de nonce; usado para toda a persistência local do kernel |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial |
| 0.2.0 | 2026-06-03 | carloscanutocosta | Referências arquivísticas (MEF-DGLAB, MoReq2010, ISO 15489, eARQ); visão document-centric AP; crates core-documental, domain-mef, domain-numerador; ecossistema normordis-core-ui e normordis-pdf |
| 0.3.0 | 2026-06-03 | carloscanutocosta | Interoperabilidade: IEA (UE) 2024/903, iAP, SDG, RCM 41/2018, ESPAP; crate interoperability; RGPD adicionado aos related; glossário expandido |
| 0.4.0 | 2026-06-03 | carloscanutocosta | NIS2 e AI Act: nis2.md e ai-act.md adicionados; crates support-crypto, secrets, core-security, services/backup; matriz expandida com supply chain, criptografia, logging IA, supervisão humana; limitações e glossário actualizados |
| 0.5.0 | 2026-06-03 | carloscanutocosta | Fase de conformidade concluída: eidas.md, seguranca-informacao.md (CRA+ISO27001), dga.md, intosai.md; todos os frameworks do frontmatter com documento dedicado |

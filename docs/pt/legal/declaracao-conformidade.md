---
title: "Normordis Kernel — Declaração de Conformidade"
type: legal
framework: [COSO, SNC-AP, RGPD, NIS2, AI-ACT, eIDAS2, DGA, CRA, ISO27001, EIF, IEA, WCAG, INTOSAI, MEF-DGLAB, MoReq2010, ISO15489]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/legal/requirements.md
  - docs/pt/legal/registo-actividades-tratamento.md
---

# Normordis Kernel — Declaração de Conformidade

## Identificação do Produto

| Campo | Valor |
|-------|-------|
| **Produto** | Normordis Kernel |
| **Versão declarada** | `0.1.x` |
| **Tipo** | Plataforma de software — biblioteca/kernel Rust para aplicações da Administração Pública |
| **Repositório** | normordis-kernel |
| **Responsável técnico** | carloscanutocosta@gmail.com |
| **Data desta declaração** | 2026-06-03 |
| **Próxima revisão** | 2026-12-03 |
| **Aprovado por** | `[a preencher]` |

---

## Âmbito da Declaração

Esta declaração cobre a conformidade do Normordis Kernel versão `0.1.x` com os instrumentos legais, regulamentares e normativos identificados na secção seguinte, **ao nível de plataforma de suporte**.

O kernel é uma plataforma headless e local — não é uma aplicação de utilizador final. A sua conformidade é avaliada enquanto:
- Componente da cadeia de abastecimento de software de sistemas AP (NIS2 Art. 29.º, CRA)
- Subcontratante de tratamento de dados pessoais (RGPD Art. 30.2)
- Infra-estrutura técnica de suporte à conformidade das aplicações integradoras

**Esta declaração não substitui** a avaliação de conformidade das aplicações construídas sobre o kernel. Os integradores são responsáveis pela conformidade dos seus sistemas.

---

## Tabela de Conformidade por Framework

### Regulamentação Europeia

| Framework | Documento de evidência | Estado | Limitações principais |
|-----------|----------------------|--------|-----------------------|
| **RGPD** — Reg. (UE) 2016/679 | [rgpd.md](../compliance/rgpd.md) | **Conforme (parcial)** | RAT disponível; consentimento e limitação de tratamento — responsabilidade do integrador |
| **NIS2** — Dir. (UE) 2022/2555 | [nis2.md](../compliance/nis2.md) | **Conforme (parcial)** | Supply chain e criptografia implementados; notificação ao CNCS (Art. 23.º) planeada |
| **AI Act** — Reg. (UE) 2024/1689 | [ai-act.md](../compliance/ai-act.md) | **Suporte técnico implementado** | O kernel não é um sistema de IA; fornece infra-estrutura de logging, supervisão e qualidade de dados para sistemas IA construídos sobre ele |
| **eIDAS 2** — Reg. (UE) 2024/1183 | [eidas.md](../compliance/eidas.md) | **Conforme (parcial)** | Infra-estrutura PAdES e contratos de autenticação implementados; integração CMD e EUDIW planeadas |
| **DGA** — Reg. (UE) 2022/868 | [dga.md](../compliance/dga.md) | **Conforme (parcial)** | Exportação auditada e neutralidade implementadas; catálogo DCAT planeado |
| **CRA** — Reg. (UE) 2024/2847 | [seguranca-informacao.md](../compliance/seguranca-informacao.md) | **Conforme (parcial)** | Supply chain, SBOM (Cargo.lock) e processo de disclosure implementados; SBOM normalizado (CycloneDX) planeado |
| **Data Act** — Reg. (UE) 2023/2854 | [dga.md](../compliance/dga.md) | **Suporte técnico implementado** | `core-exports` suporta portabilidade; requisitos específicos — responsabilidade do integrador |
| **IEA** — Reg. (UE) 2024/903 | [interoperabilidade.md](../compliance/interoperabilidade.md) | **Conforme (parcial)** | Avaliação de interoperabilidade documentada; RNID e SEMIC planeados |
| **SDG** — Reg. (UE) 2018/1724 | [interoperabilidade.md](../compliance/interoperabilidade.md) | **Suporte técnico implementado** | Conformidade SDG — responsabilidade das aplicações integradoras |

### Legislação Nacional (Portugal)

| Framework | Documento de evidência | Estado | Limitações principais |
|-----------|----------------------|--------|-----------------------|
| **SNC-AP** — DL 192/2015 | [snc-ap.md](../compliance/snc-ap.md) | **Conforme (parcial)** | MEF, numeração e infra documental implementados; módulo contabilístico (PCM, lançamentos) planeado |
| **DL 83/2018** — Acessibilidade | [wcag.md](../compliance/wcag.md) | **Conforme via ecossistema** | Implementado em `normordis-core-ui` e `normordis-pdf`; kernel contribui com semântica de dados |
| **Lei 46/2018** — Cibersegurança PT | [nis2.md](../compliance/nis2.md) | **Conforme (parcial)** | Em revisão para transposição NIS2; kernel alinhado com requisitos NIS2 |
| **Lei 98/97** — Tribunal de Contas | [intosai.md](../compliance/intosai.md), [coso.md](../compliance/coso.md) | **Conforme** | Audit trail imutável com rastreabilidade financeira; evidências disponíveis para fiscalização |
| **DL 447/88 + Portarias arquivísticas** | [arquivistica.md](../compliance/arquivistica.md) | **Conforme (parcial)** | Ciclo de vida e MEF-DGLAB implementados; tabela de selecção e auto de eliminação planeados |
| **iAP / ESPAP** — Portaria 195/2018 | [interoperabilidade.md](../compliance/interoperabilidade.md) | **Parcial** | Validadores locais (NIF, IBAN, moradas) implementados; integração iAP em tempo real planeada |

### Normas e Frameworks Internacionais

| Framework | Documento de evidência | Estado | Limitações principais |
|-----------|----------------------|--------|-----------------------|
| **COSO 2013** | [coso.md](../compliance/coso.md) | **Conforme** | 17 princípios implementados; supervisão do conselho (P2) — responsabilidade organizacional |
| **INTOSAI GOV 9100** | [intosai.md](../compliance/intosai.md) | **Conforme** | Reforços de accountability e transparência implementados sobre COSO |
| **ISO/IEC 27001:2022** | [seguranca-informacao.md](../compliance/seguranca-informacao.md) | **Conforme (parcial)** | Controlos tecnológicos implementados; certificação formal — decisão do integrador |
| **WCAG 2.1 AA** | [wcag.md](../compliance/wcag.md) | **Conforme via ecossistema** | `normordis-core-ui` implementa critérios AA; kernel fornece semântica |
| **EN 301 549 v3.2.1** | [wcag.md](../compliance/wcag.md) | **Conforme via ecossistema** | Via `normordis-core-ui`; cláusulas 9–11 |
| **EIF v2** | [interoperabilidade.md](../compliance/interoperabilidade.md) | **Conforme (parcial)** | 12 princípios mapeados; SEMIC/RNID planeados |
| **MEF-DGLAB** | [arquivistica.md](../compliance/arquivistica.md), [snc-ap.md](../compliance/snc-ap.md) | **Conforme** | `domain-mef` com tabela temporal e diplomas legais |
| **ISO 15489-1:2016** | [arquivistica.md](../compliance/arquivistica.md) | **Conforme (parcial)** | Ciclo de vida e metadados base implementados; MIP completo planeado |
| **MoReq2010** | [arquivistica.md](../compliance/arquivistica.md) | **Conforme (parcial)** | Módulos M2–M5 implementados; eARQ formal requer validação DGLAB |
| **IPSAS** | [snc-ap.md](../compliance/snc-ap.md) | **Suporte técnico** | Via SNC-AP; módulo contabilístico planeado |

---

## Legenda de Estados

| Estado | Significado |
|--------|-------------|
| **Conforme** | Todos os requisitos relevantes para o âmbito do kernel implementados e verificáveis |
| **Conforme (parcial)** | Requisitos principais implementados; componentes secundários ou opcionais planeados |
| **Conforme via ecossistema** | Implementado em projecto do ecossistema Normordis (`normordis-core-ui`, `normordis-pdf`) — não no kernel directamente |
| **Suporte técnico implementado** | Infra-estrutura técnica disponível; conformidade plena — responsabilidade da aplicação integradora |
| **Parcial** | Implementação base disponível; integração completa com serviços externos planeada |

---

## Evidências de Conformidade

A conformidade declarada neste documento é suportada pelo seguinte conjunto de evidências:

**Documentação de conformidade** (`docs/pt/compliance/`):

| Documento | Frameworks cobertos |
|-----------|-------------------|
| [overview.md](../compliance/overview.md) | Visão geral — todos os frameworks |
| [coso.md](../compliance/coso.md) | COSO 2013 — 17 princípios |
| [intosai.md](../compliance/intosai.md) | INTOSAI GOV 9100 |
| [snc-ap.md](../compliance/snc-ap.md) | SNC-AP, IPSAS |
| [arquivistica.md](../compliance/arquivistica.md) | MEF-DGLAB, MoReq2010, ISO 15489, eARQ |
| [rgpd.md](../compliance/rgpd.md) | RGPD |
| [nis2.md](../compliance/nis2.md) | NIS2, CRA (parcial), ISO 27001 (parcial) |
| [seguranca-informacao.md](../compliance/seguranca-informacao.md) | CRA, ISO 27001 |
| [ai-act.md](../compliance/ai-act.md) | AI Act |
| [eidas.md](../compliance/eidas.md) | eIDAS 2 |
| [interoperabilidade.md](../compliance/interoperabilidade.md) | EIF, IEA, iAP |
| [dga.md](../compliance/dga.md) | DGA |
| [wcag.md](../compliance/wcag.md) | WCAG, A11Y, EN 301 549 |

**Documentação legal** (`docs/pt/legal/`):

| Documento | Propósito |
|-----------|-----------|
| [requirements.md](requirements.md) | Catálogo completo de instrumentos aplicáveis |
| [registo-actividades-tratamento.md](registo-actividades-tratamento.md) | RAT RGPD Art. 30.º |

**Evidências técnicas** (repositório):

| Artefacto | Localização | Evidência |
|-----------|-------------|-----------|
| Trust Baseline | `docs/adr/ADR-NK-006` | Política formal de confiança |
| Política de dependências | `security/DEPENDENCY_POLICY.md` | Critérios de supply chain |
| Allowlists | `security/ALLOWLISTS.md` | `unsafe` e excepções aprovadas |
| Política de segurança | `SECURITY.md` | Processo de disclosure |
| SBOM | `Cargo.lock` | Inventário de dependências |
| Resultados de CI | Pipeline CI | `cargo audit`, `clippy`, testes |

---

## Limitações Gerais da Declaração

1. **Auto-declaração**: esta é uma declaração de conformidade emitida pelo próprio projecto, não por um organismo de avaliação independente. Para frameworks que exigem certificação formal (ISO 27001, eARQ), a decisão de certificação é do integrador.

2. **Âmbito de plataforma**: a conformidade é declarada ao nível do kernel como plataforma. As aplicações construídas sobre o kernel têm responsabilidades de conformidade adicionais que não são cobertas por esta declaração.

3. **Versão**: esta declaração é válida para a versão `0.1.x`. Versões futuras serão objecto de revisão e nova declaração.

4. **Regulamentação em evolução**: NIS2 (transposição PT em curso), AI Act (aplicação faseada até 2027), CRA (aplicação plena 2027) — a declaração será revista em conformidade.

5. **Itens planeados**: os itens marcados como "planeados" nesta declaração e nos documentos de suporte não estão implementados na versão actual. A sua inclusão na declaração reflecte o compromisso de implementação, não conformidade presente.

---

## Declaração Formal

O Normordis Kernel, versão `0.1.x`, foi concebido e desenvolvido segundo o princípio de **compliance by design**: a conformidade legal e normativa é uma restrição arquitectural de primeira classe, não um requisito adicionado após a construção. Os instrumentos legais identificados nesta declaração foram considerados durante o design, e a sua implementação é verificável através das evidências listadas.

Esta declaração é emitida com base no conhecimento actual do estado de implementação do kernel e nos instrumentos legais em vigor à data indicada. Não constitui garantia de conformidade para usos específicos — os integradores são responsáveis por avaliar a adequação do kernel aos requisitos do seu contexto.

| Campo | Valor |
|-------|-------|
| **Emitido por** | carloscanutocosta@gmail.com |
| **Data** | 2026-06-03 |
| **Versão do kernel** | 0.1.x |
| **Aprovado por** | `[a preencher]` |
| **Próxima revisão** | 2026-12-03 |

---

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — declaração completa com todos os frameworks; tabelas de estado e evidências |

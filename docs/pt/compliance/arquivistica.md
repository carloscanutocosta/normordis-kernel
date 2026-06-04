---
title: "Normordis Kernel — Conformidade Arquivística e Gestão de Documentos de Arquivo"
type: compliance
framework: [ARQUIVISTICA, MEF-DGLAB, MoReq2010, ISO15489]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/snc-ap.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade Arquivística e Gestão de Documentos de Arquivo

## Sumário Executivo

O Normordis Kernel foi concebido com uma visão **document-centric** para a Administração Pública: cada documento produzido ou recebido é um activo institucional com ciclo de vida próprio, classe arquivística, prazo de conservação e destino final definidos. Esta visão não é opcional para entidades públicas — é uma obrigação legal derivada do quadro arquivístico nacional e europeu.

O `core-documental` é o crate central desta visão. Implementa o ciclo de vida documental completo — da criação ao arquivo ou eliminação — com rastreabilidade total de eventos e integração com os restantes módulos do kernel. O `domain-mef` serve dupla função: além da classificação orçamental (SNC-AP), é também o suporte da Macroestrutura Funcional arquivística (DGLAB), que classifica documentos por função do Estado.

A conformidade arquivística do kernel assenta em cinco pilares: classificação funcional (MEF-DGLAB), ciclo de vida documentado, metadados de arquivo, segurança e integridade, e interoperabilidade. Os três primeiros têm infra-estrutura implementada; os dois últimos estão parcialmente implementados com trabalho planeado.

## Âmbito

Cobre o mapeamento entre o quadro arquivístico e normativo de gestão de documentos de arquivo e a implementação do Normordis Kernel versão `0.1.x`. Não cobre:

- Gestão de arquivos históricos ou permanentes — responsabilidade do arquivo institucional ou DGLAB
- Digitalização de documentos em papel de fundo histórico — fora do âmbito operacional do kernel
- Sistemas de gestão de arquivo (SGA) de terceiros — o kernel fornece os contratos e adapters, não o SGA completo
- Regulamentação específica de arquivos de saúde, notariado ou judicial

## Referências Normativas

**Legislação e regulamentação nacional**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [ARQ-DL-447] | DL n.º 447/88, de 10 de dezembro | Integralmente | Princípios da política arquivística nacional para a AP |
| [ARQ-P-412] | Portaria n.º 412/2001, de 17 de abril | Integralmente | Organização, funcionamento e conservação de arquivos da AP central |
| [ARQ-P-1253] | Portaria n.º 1253/2009, de 14 de outubro | Integralmente | Gestão de documentos de arquivo para organismos da AP central |
| [LADA] | Lei n.º 26/2016, de 22 de agosto | Art. 5.º–7.º, 15.º | Acesso a documentos administrativos, prazos de conservação e dados abertos |
| [MEF-DGLAB] | Macroestrutura Funcional (DGLAB, edição actualizada) | Integralmente | Classificação funcional de documentos de arquivo da AP |
| [eARQ] | eARQ Portugal — Requisitos para Sistemas de Arquivo Electrónico (DGLAB) | Integralmente | Especificação de requisitos funcionais e não-funcionais para SEGA |
| [MIP] | Metainformação para Interoperabilidade (DGLAB) | Integralmente | Esquema de metadados obrigatório para documentos de arquivo electrónico |

**Normas internacionais**

| Ref. | Norma | Secção | Relevância |
|------|-------|--------|------------|
| [ISO15489-1] | ISO 15489-1:2016 — Records Management | Integralmente | Princípios e requisitos de gestão de documentos de arquivo |
| [ISO15489-2] | ISO 15489-2:2016 — Records Management — Guidelines | Integralmente | Orientações de implementação do ISO 15489-1 |
| [MoReq2010] | MoReq2010 — Model Requirements for Records Systems (CECA/DGLAB) | Módulos 1–7 | Modelo europeu de requisitos para sistemas de gestão de documentos |
| [NP4041] | NP 4041:2005 — Terminologia Arquivística | Integralmente | Terminologia normalizada — vocabulário de referência |
| [NP4438-1] | NP 4438-1:2005 — Gestão de Documentos de Arquivo | Integralmente | Princípios e requisitos de gestão |
| [NP4438-2] | NP 4438-2:2006 — Gestão de Documentos de Arquivo — Directrizes | Integralmente | Directrizes de implementação |

## Visão Document-Centric para a AP

### O documento como unidade central

Na AP, o documento é a unidade fundamental de registo, prova e memória institucional. Uma plataforma verdadeiramente conforme com o quadro arquivístico trata o documento não como um ficheiro, mas como uma entidade com:

```
Documento de arquivo
    ├── Identidade:    número, data, autor, entidade produtora
    ├── Classificação: classe MEF (função + subfunção + série)
    ├── Ciclo de vida: criação → tramitação → arquivo corrente
    │                  → arquivo intermédio → destino final
    ├── Metadados:     MIP — título, datas, tipo, formato, relações
    ├── Integridade:   hash, assinatura, cadeia de custódia
    └── Destino final: conservação permanente (C) ou eliminação (E)
```

### Duplo papel do MEF no kernel

O `domain-mef` no kernel serve **duas funções complementares**:

| Contexto | MEF | Propósito |
|----------|-----|-----------|
| SNC-AP (orçamental) | Macroestrutura Funcional — classificação funcional de despesas | Classificar despesas por função do Estado para o relato orçamental |
| Arquivística (DGLAB) | Macroestrutura Funcional — plano de classificação de documentos | Organizar documentos por função produtora; determinar conservação/eliminação |

Ambos os contextos usam a mesma taxonomia funcional do Estado. O `domain-mef` é o ponto de unificação: um único crate serve os dois subsistemas, com a mesma tabela temporal e rastreabilidade de diplomas legais.

### Ciclo de vida documental no kernel

```
[Criação / Ingestão]
    core-ingest (documentos externos)
    core-documental (documentos produzidos)
    adapter-scanner (digitalização)
          │
          ▼
[Registo e Classificação]
    domain-numerador  →  número único de registo
    domain-mef        →  classe arquivística (MEF-DGLAB)
    core-documental   →  metadados MIP, tipo, formato
          │
          ▼
[Tramitação]
    core-documental   →  log de eventos de tramitação
    core-org          →  unidade orgânica responsável (despacho, aprovação)
    core-audit        →  AuditEvent por cada acção sobre o documento
          │
          ▼
[Finalização e Assinatura]
    services/signing  →  assinatura qualificada (eIDAS 2)
    core-documental   →  transição para estado "finalizado"
    pdf/pdf-pipeline  →  geração de PDF/A (formato de arquivo)
          │
          ▼
[Arquivo Corrente → Intermédio]
    core-documental   →  transição de estados por prazo
    core-exports      →  exportação para sistemas de arquivo
          │
          ▼
[Destino Final]
    Conservação (C)   →  transferência para arquivo permanente
    Eliminação (E)    →  auto de eliminação + registo de destruição
```

---

## Mapeamento por Requisito Arquivístico

### Classificação e Organização (ISO 15489 §6.3, MoReq2010 M2)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Plano de classificação funcional | `domain-mef` | MEF-DGLAB com tabela temporal | Implementado |
| Atribuição de classe a cada documento | `core-documental` + `domain-mef` | Campo de classe arquivística no documento | Implementado (base) |
| Séries documentais | `core-documental` | Tipologias documentais por série | Planeado |
| Numeração de registo | `domain-numerador` | Sequência por série, ano e entidade | Implementado |

### Metadados (MIP, MoReq2010 M3, ISO 15489 §8)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Metadados de identificação (título, datas, autor) | `core-documental` | Campos obrigatórios no modelo de documento | Implementado (base) |
| Metadados de contexto (entidade produtora, unidade) | `core-org` + `core-documental` | Associação a unidade orgânica produtora | Implementado |
| Metadados de relação (respostas, anexos, referências) | `core-documental` | Relações entre documentos | Planeado |
| Metadados de preservação (formato, hash, versão) | `core-documental`, `support-versioning` | Hash de integridade, formato de arquivo | Planeado |
| MIP completo | — | — | Planeado |

### Ciclo de Vida e Retenção (ISO 15489 §7, MoReq2010 M4)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Estados do ciclo de vida | `core-documental` | Estados: rascunho, em tramitação, finalizado, arquivado | Implementado |
| Log de eventos de documento | `core-documental` | Registo de cada transição de estado com actor e timestamp | Implementado |
| Audit trail de operações | `core-audit` | `AuditEvent` por cada operação sobre documento | Implementado |
| Prazo de conservação por série/classe | — | — | Planeado |
| Tabela de selecção e avaliação | — | — | Planeado |
| Alertas de prazo vencido | — | — | Planeado |

### Integridade e Autenticidade (eARQ §4.3, MoReq2010 M5)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Imutabilidade após finalização | `core-documental` | Transição de estado unidireccional após fecho | Implementado (base) |
| Assinatura qualificada | `services/signing` | Assinatura com suporte eIDAS 2 | Implementado |
| Hash de integridade | `core-audit` | Cadeia de hashes verificável | Implementado |
| Formato de arquivo (PDF/A) | `pdf/pdf-pipeline` | Geração de PDF/A via Typst | Planeado |
| Cifra em repouso | `infra` (SQLite cifrado) | SQLite com XChaCha20-Poly1305 | Implementado |
| Cadeia de custódia | `core-audit` + `core-documental` | Log completo de operações + actors | Implementado |

### Acesso e Pesquisa (MoReq2010 M6, LADA)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Pesquisa por metadados | `core-documental` | Pesquisa por tipo, estado, data, entidade | Implementado (base) |
| Pesquisa por classe MEF | `domain-mef` + `core-documental` | Filtro por código de classificação | Implementado (base) |
| Controlo de acesso a documentos | `core-security` | Políticas de acesso por documento/série | Planeado |
| Registo de acessos | `core-audit` | `AuditEvent` para operações de leitura | Planeado |
| Exportação para entidades externas | `core-exports` | Snapshots e exportação estruturada | Implementado |

### Eliminação e Preservação (ISO 15489 §9, ARQ-P-412)

| Requisito | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Auto de eliminação | `core-documental` + `core-audit` | Documento de eliminação com evidência | Planeado |
| Destruição controlada e auditada | `core-audit` | AuditEvent de eliminação (não-supressão silenciosa) | Planeado |
| Transferência para arquivo permanente | `core-exports` | Exportação para arquivo histórico | Planeado |
| Preservação de metadados pós-eliminação | `core-audit` | Registo de eliminação retido no audit log | Planeado |

---

## Matriz de Conformidade

| Requisito | Crate / Módulo | Evidência | Status |
|-----------|---------------|-----------|--------|
| Classificação funcional MEF-DGLAB | `domain-mef` | Tabela temporal com diplomas legais | Implementado |
| Numeração de registo por série | `domain-numerador` | Sequências por tipo, ano e entidade | Implementado |
| Ciclo de vida documentado (estados) | `core-documental` | Estados + log de transições | Implementado |
| Audit trail de operações documentais | `core-audit` | `AuditEvent` com actor, timestamp, control_id | Implementado |
| Assinatura qualificada de documentos | `services/signing` | Assinatura eIDAS 2 | Implementado |
| Geração de documentos (PDF/A institucional) | `pdf/normordis-pdf`¹, `pdf/pdf-pipeline` | Pipeline Typst → PDF/A (ecossistema Normordis) | Implementado |
| Ingestão de documentos externos | `core-ingest`, `adapter-scanner` | Registo de entrada com metadados | Implementado |
| Integridade criptográfica | `core-audit`, `support-crypto` | Hash encadeado, SQLite cifrado | Implementado |
| Estrutura organizacional produtora | `core-org` | Unidades orgânicas e posições | Implementado |
| Metadados MIP completos | — | — | Planeado |
| Séries documentais formalizadas | — | — | Planeado |
| Tabela de selecção e prazos de retenção | — | — | Planeado |
| Formato PDF/A para arquivo | `pdf/normordis-pdf`¹ | Pipeline Typst → PDF/A (ecossistema Normordis) | Implementado |
| Controlo de acesso por série/classe | `core-security` | — | Planeado |
| Auto de eliminação auditado | `core-documental` + `core-audit` | — | Planeado |
| Transferência para arquivo permanente | `core-exports` | — | Planeado |
| Conformidade eARQ Portugal | — | — | Planeado |

## Limitações e Exclusões Conhecidas

- **eARQ Portugal**: o kernel ainda não foi avaliado formalmente contra a especificação eARQ. A conformidade total com eARQ requer validação por entidade competente (DGLAB).
- **MIP completo**: o esquema de metadados MIP não está totalmente implementado; o `core-documental` cobre os campos essenciais mas faltam campos de preservação e relação.
- **Tabela de selecção**: os prazos de conservação e destinos finais por série não estão implementados — é o módulo de maior impacto para a conformidade arquivística operacional.
- **PDF/A**: implementado via `normordis-pdf`, projecto do ecossistema Normordis que gera PDF/A conforme através do pipeline Typst. O kernel integra `normordis-pdf` como dependência de ecossistema — a conformidade PDF/A é garantida por esse projecto, não reimplementada no kernel.
- **Arquivo histórico**: o kernel não inclui funcionalidades de arquivo histórico ou permanente (gestão de fundos, descrição arquivística EAD). Estas são responsabilidade do sistema de arquivo institucional ou da DGLAB.
- **Autoridade local**: o kernel tem autoridade local e operacional. A custódia definitiva e a validação do arquivo permanente são responsabilidades externas.

## Glossário

| Termo | Definição |
|-------|-----------|
| **Auto de eliminação** | Documento que regista formalmente a destruição de documentos de arquivo após avaliação |
| **Classe arquivística** | Unidade de classificação no plano de classificação — corresponde a uma função ou actividade |
| **Custódia** | Responsabilidade pela guarda e protecção de documentos de arquivo; não implica propriedade |
| **Destino final** | Decisão de avaliação arquivística: C (Conservação permanente) ou E (Eliminação) |
| **DGLAB** | Direcção-Geral do Livro, dos Arquivos e das Bibliotecas — autoridade arquivística nacional |
| **eARQ Portugal** | Especificação de Requisitos para Sistemas de Arquivo Electrónico — publicada pela DGLAB |
| **Fundo** | Conjunto de documentos produzidos e recebidos por uma entidade no exercício das suas funções |
| **MEF-DGLAB** | Macroestrutura Funcional (DGLAB) — plano de classificação funcional de documentos da AP |
| **MIP** | Metainformação para Interoperabilidade — esquema de metadados normalizados para documentos de arquivo electrónico (DGLAB) |
| **MoReq2010** | Model Requirements for Records Systems — modelo europeu de requisitos para sistemas de gestão de documentos; adoptado pela DGLAB |
| **PDF/A** | Subconjunto do PDF normalizado pela ISO 19005 para preservação de longa duração |
| **Prazo de conservação** | Período durante o qual um documento deve ser conservado antes de avaliação para destino final |
| **Série documental** | Conjunto de documentos do mesmo tipo, produzidos no exercício da mesma função |
| **SEGA** | Sistema Electrónico de Gestão de Arquivo — denominação portuguesa para RMS |
| **Tabela de selecção** | Instrumento que define, por série, os prazos de conservação e destinos finais |
| **Tramitação** | Circuito de um documento dentro de uma entidade até à sua resolução ou arquivo |

---

> ¹ **normordis-pdf** é um projecto autónomo do ecossistema Normordis que implementa a geração de PDF/A institucional via pipeline Typst. O kernel integra-o como dependência de ecossistema; a conformidade com os perfis PDF/A-1b / PDF/A-2b é garantida por esse projecto.

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — visão document-centric AP, mapeamento core-documental e domain-mef |

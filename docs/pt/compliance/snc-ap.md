---
title: "Normordis Kernel — Conformidade SNC-AP"
type: compliance
framework: SNC-AP
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/coso.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade SNC-AP

## Sumário Executivo

O Sistema de Normalização Contabilística para as Administrações Públicas (SNC-AP), estabelecido pelo DL n.º 192/2015, define o quadro contabilístico obrigatório para todas as entidades das administrações públicas portuguesas. Baseia-se nas normas IPSAS e organiza-se em três subsistemas: contabilidade orçamental, financeira e analítica.

O Normordis Kernel não é um sistema contabilístico em si — é a plataforma de suporte sobre a qual sistemas SNC-AP conformes são construídos. Esta distinção é fundamental: o kernel fornece a infra-estrutura de classificação orçamental, rastreabilidade de documentos, numeração sequencial, auditoria de operações e geração de relatórios que os subsistemas SNC-AP requerem.

O alinhamento mais directo já implementado é o `domain-mef`, que implementa a Macroestrutura Funcional (classificação orçamental funcional obrigatória no SNC-AP) com tabela temporal e suporte a diplomas legais. Os restantes módulos SNC-AP estão planeados para fase seguinte.

## Âmbito

Cobre o mapeamento entre os requisitos do SNC-AP e a implementação do Normordis Kernel versão `0.1.x`. Não cobre:

- Implementação contabilística nas aplicações consumidoras — responsabilidade do integrador
- Elaboração de demonstrações financeiras — suportada pelo kernel mas executada pelas apps
- SNC-AP para administração regional (DL 85/2016) — mesmo quadro, âmbito distinto
- POCAL (predecessor ao SNC-AP) — fora de âmbito

## Referências Normativas

**Legislação principal**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [SNC-AP] | DL n.º 192/2015, de 11 de setembro | Art. 3.º–5.º | Âmbito, subsistemas e entrada em vigor |
| [SNC-AP-PCM] | Portaria n.º 189/2016, de 14 de julho | Integralmente | Plano de Contas Multidimensional (PCM) |
| [SNC-AP-BADF] | Portaria n.º 218/2016, de 9 de agosto | Integralmente | Bases para Apresentação das Demonstrações Financeiras |
| [SNC-AP-NCP] | Aviso n.º 8254/2015, DR 2.ª série, n.º 242 | NCPs 1–27 | Normas de Contabilidade Pública |
| [LEO] | Lei n.º 151/2015, de 11 de setembro (Lei de Enquadramento Orçamental) | Art. 9.º–12.º | Princípios orçamentais e classificações |
| [SNC-AP-REG] | DL n.º 85/2016, de 21 de dezembro | Integralmente | SNC-AP para administração regional |

**Normas internacionais de base**

| Ref. | Norma | Secção | Relevância |
|------|-------|--------|------------|
| [IPSAS-1] | IPSAS 1 — Presentation of Financial Statements | Integralmente | Base da NCP 1 — estrutura das demonstrações financeiras |
| [IPSAS-2] | IPSAS 2 — Cash Flow Statements | Integralmente | Base da NCP 2 — demonstrações de fluxos de caixa |
| [IPSAS-24] | IPSAS 24 — Presentation of Budget Information | Integralmente | Base da NCP 26 — contabilidade e relato orçamental |
| [IPSAS-35] | IPSAS 35 — Consolidated Financial Statements | Integralmente | Base da NCP 27 — demonstrações financeiras consolidadas |

## O SNC-AP no Contexto do Kernel

### Estrutura do SNC-AP

O SNC-AP organiza-se em três subsistemas interdependentes:

```
SNC-AP
├── Contabilidade Orçamental
│     Regista execução do orçamento (despesa e receita)
│     Classificações: económica, orgânica, funcional (MEF), programa
│
├── Contabilidade Financeira
│     Base de acréscimo (accrual) — activos, passivos, rendimentos, gastos
│     Demonstrações: Balanço, DR, DFC, DACP, Anexo
│
└── Contabilidade Analítica (opcional para algumas entidades)
      Apuramento de custos por actividade, programa ou serviço
```

### Papel do Kernel numa Stack SNC-AP

O kernel posiciona-se como a **camada de plataforma** — não executa contabilidade, mas fornece os blocos fundamentais:

```
App consumidora (SNC-AP)
    │
    ├── Classificação orçamental   → domain-mef (MEF), domain-numerador
    ├── Ciclo de vida documental   → core-documental, pdf/pipeline
    ├── Rastreabilidade financeira → core-audit (AuditEvent + control_id)
    ├── Estrutura institucional    → core-org (unidades orgânicas)
    ├── Validação de dados         → core-validation (NIF, IBAN, datas)
    ├── Identificadores únicos     → support-ids, domain-numerador
    ├── Exportação / reporte       → core-exports, pdf/normordis-pdf
    └── Segurança e cifra          → support-crypto, secrets
```

---

## Mapeamento por Subsistema

### Subsistema 1 — Contabilidade Orçamental

A contabilidade orçamental regista a execução do orçamento com quatro classificações obrigatórias: económica, orgânica, funcional (MEF) e por programa.

#### MEF — Macroestrutura Funcional

O kernel implementa a classificação funcional obrigatória através de `domain-mef`:

| Funcionalidade | Crate | Implementação | Status |
|---------------|-------|---------------|--------|
| Tabela MEF com vigência temporal | `domain-mef` | Entidades com datas de início/fim — suporta alterações de diplomas | Implementado |
| Associação a diploma legal | `domain-mef` | Referência ao diploma que criou ou alterou cada classificação | Implementado |
| Adapter SQLite | `mef-sqlite` | Persistência local da tabela MEF | Implementado |
| Classificação funcional de despesa | `domain-mef` | Atribuição de código MEF a rubricas orçamentais | Implementado |
| Actualização da tabela por nova lei | `domain-mef` | Novo registo temporal sem destruição do histórico | Implementado |

**Alinhamento normativo:** NCP 26 (IPSAS 24) — Contabilidade e Relato Orçamental exige que a execução orçamental seja apresentada por classificação funcional. O `domain-mef` implementa essa taxonomia de forma temporalmente correcta.

#### Numeração de Documentos Orçamentais

| Funcionalidade | Crate | Implementação | Status |
|---------------|-------|---------------|--------|
| Numeração sequencial por série | `domain-numerador` | Sequências por tipo documental, ano e entidade | Implementado |
| Alocação apenas na finalização | `domain-numerador` | ADR-NK-005 — número atribuído apenas ao finalizar | Implementado |
| Adapter SQLite | `numerador-sqlite` | Persistência local com garantia de unicidade | Implementado |

#### Classificação Económica e Orgânica

| Funcionalidade | Crate | Status |
|---------------|-------|--------|
| Plano de Contas Multidimensional (PCM) | — | Planeado |
| Classificação económica de despesa/receita | — | Planeado |
| Unidades orgânicas como centros orçamentais | `core-org` (base) | Parcial |

### Subsistema 2 — Contabilidade Financeira

A contabilidade financeira opera em base de acréscimo (*accrual*) e produz as demonstrações financeiras exigidas pelo SNC-AP: Balanço, Demonstração de Resultados, Demonstração de Fluxos de Caixa, Demonstração das Alterações no Capital Próprio, e Anexo.

| Funcionalidade | Crate | Status |
|---------------|-------|--------|
| Lançamentos contabilísticos (diário) | — | Planeado |
| Plano de contas (PCM) | — | Planeado |
| Balanço | — | Planeado |
| Demonstração de Resultados | — | Planeado |
| Demonstração de Fluxos de Caixa | `core-documental` + `pdf/` (infra) | Planeado |
| Demonstração das Alterações no Capital Próprio | — | Planeado |
| Anexo (notas às demonstrações) | `core-documental` (infra) | Planeado |
| Reconciliação orçamental-financeira | — | Planeado |

**Infra-estrutura já disponível para suporte:**

| Funcionalidade | Crate | Implementação |
|---------------|-------|---------------|
| Ciclo de vida documental | `core-documental` | Estados de documento, log de eventos |
| Geração de PDF | `pdf/normordis-pdf`, `pdf/pdf-pipeline` | Pipeline Typst → PDF para demonstrações |
| Assinatura de documentos | `services/signing` | Assinatura qualificada de demonstrações financeiras |
| Validação de NIF / IBAN | `core-validation` | Entidades e contas financeiras |
| Audit trail de operações | `core-audit` | Rastreabilidade de cada lançamento |

### Subsistema 3 — Contabilidade Analítica

A contabilidade analítica é opcional para algumas entidades (obrigatória para organismos com orçamento acima de determinado limiar) e apura custos por actividade, programa ou serviço.

| Funcionalidade | Crate | Status |
|---------------|-------|--------|
| Centros de custo | `core-org` (base estrutural) | Planeado (módulo dedicado) |
| Imputação de custos | — | Planeado |
| Relato analítico | — | Planeado |

---

## Mapeamento por NCP

As 27 Normas de Contabilidade Pública definem o tratamento contabilístico por tema. As NCPs com maior relevância para o kernel:

| NCP | Título | Relevância para o kernel | Status |
|-----|--------|--------------------------|--------|
| NCP 1 | Estrutura e conteúdo das demonstrações financeiras | `core-documental`, `pdf/` | Planeado |
| NCP 2 | Políticas contabilísticas, estimativas e erros | `core-config`, `support-versioning` | Parcial |
| NCP 5 | Activos fixos tangíveis | — | Planeado |
| NCP 7 | Activos intangíveis | — | Planeado |
| NCP 11 | Inventários | — | Planeado |
| NCP 14 | Rédito de transacções com contraprestação | — | Planeado |
| NCP 15 | Rédito de transacções sem contraprestação (impostos) | — | Planeado |
| NCP 18 | Benefícios dos empregados | `core-rh` (base) | Planeado |
| NCP 26 | Contabilidade e relato orçamental | `domain-mef`, `domain-numerador` | Parcial |
| NCP 27 | Demonstrações financeiras consolidadas | — | Planeado |

---

## Matriz de Conformidade

| Requisito SNC-AP | Crate / Módulo | Evidência | Status |
|-----------------|---------------|-----------|--------|
| Classificação funcional (MEF) — NCP 26 | `domain-mef` | Tabela temporal com diplomas legais | Implementado |
| Numeração sequencial de documentos | `domain-numerador` | Sequências por tipo, ano e entidade | Implementado |
| Estrutura orgânica (unidades) | `core-org` | Hierarquia de unidades orgânicas | Implementado |
| Validação de identificadores financeiros (NIF, IBAN) | `core-validation` | Validadores canónicos | Implementado |
| Ciclo de vida documental | `core-documental` | Estados e log de eventos de documento | Implementado |
| Geração PDF de documentos financeiros | `pdf/normordis-pdf`, `pdf/pdf-pipeline` | Pipeline Typst → PDF | Implementado |
| Assinatura de documentos financeiros | `services/signing` | Assinatura qualificada | Implementado |
| Audit trail de operações financeiras | `core-audit` | `AuditEvent` com `control_id` | Implementado |
| Exportação de dados | `core-exports` | Snapshots auditáveis | Implementado |
| Plano de Contas Multidimensional (PCM) | — | — | Planeado |
| Lançamentos contabilísticos | — | — | Planeado |
| Demonstrações financeiras (NCP 1) | — | — | Planeado |
| Relato orçamental completo (NCP 26) | `domain-mef` + módulo orçamental | — | Parcial |
| Contabilidade analítica | — | — | Planeado |
| Reconciliação orçamental-financeira | — | — | Planeado |
| Consolidação (NCP 27) | — | — | Planeado |

---

## Limitações e Exclusões Conhecidas

- **PCM não implementado**: o Plano de Contas Multidimensional (Portaria 189/2016) é o núcleo da contabilidade financeira SNC-AP e ainda não existe como módulo do kernel.
- **Lançamentos contabilísticos**: o kernel não implementa motor contabilístico (diário, razão, balancete). Esta é a lacuna mais significativa para conformidade SNC-AP completa.
- **NCP 26 parcial**: a classificação MEF está implementada, mas o relato orçamental completo (execução de despesa e receita) requer módulo dedicado.
- **Contabilidade analítica**: não implementada; relevante apenas para entidades com obrigação legal.
- **Consolidação (NCP 27)**: fora de âmbito na versão actual; exige agregação multi-entidade.
- **Âmbito institucional**: o kernel tem autoridade local. A consolidação central e o reporte ao DGO/ESPAP são responsabilidades externas ao kernel.
- **POCAL**: o kernel não suporta POCAL (predecessor SNC-AP); apenas SNC-AP.

## Glossário

| Termo | Definição |
|-------|-----------|
| **BADF** | Bases para Apresentação das Demonstrações Financeiras — Portaria n.º 218/2016 |
| **Classificação económica** | Classificação de despesas e receitas por natureza económica (ex: pessoal, bens, transferências) |
| **Classificação funcional** | Classificação de despesas pela função do Estado que servem — implementada pelo MEF |
| **Classificação orgânica** | Classificação de despesas pela entidade ou unidade orgânica responsável |
| **DGO** | Direcção-Geral do Orçamento — entidade supervisora da execução orçamental |
| **ESPAP** | Entidade de Serviços Partilhados da Administração Pública |
| **IPSAS** | International Public Sector Accounting Standards — normas internacionais que fundamentam o SNC-AP |
| **LEO** | Lei de Enquadramento Orçamental — Lei n.º 151/2015 |
| **MEF** | Macroestrutura Funcional — classificação funcional das despesas públicas; obrigatória no SNC-AP |
| **NCP** | Norma de Contabilidade Pública — uma das 27 normas que constituem o SNC-AP |
| **PCM** | Plano de Contas Multidimensional — Portaria n.º 189/2016; estrutura de contas do SNC-AP |
| **POCAL** | Plano Oficial de Contabilidade das Autarquias Locais — predecessor do SNC-AP, em extinção |
| **POCP** | Plano Oficial de Contabilidade Pública — predecessor do SNC-AP a nível central |
| **SNC-AP** | Sistema de Normalização Contabilística para as Administrações Públicas — DL n.º 192/2015 |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — mapeamento SNC-AP com destaque para domain-mef e infra-estrutura existente |

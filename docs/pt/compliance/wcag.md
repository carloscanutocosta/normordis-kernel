---
title: "Normordis Kernel — Conformidade WCAG e Acessibilidade Digital"
type: compliance
framework: [WCAG, A11Y, EN301549]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade WCAG e Acessibilidade Digital

## Sumário Executivo

A acessibilidade digital para a Administração Pública é uma obrigação legal em Portugal desde o DL n.º 83/2018, que transpõe a Directiva (UE) 2016/2102 e adopta a norma EN 301 549 como referência técnica — a qual incorpora os critérios WCAG 2.1 nível AA.

O Normordis Kernel é uma plataforma headless sem camada de apresentação própria. Por isso, a conformidade WCAG é implementada principalmente em `normordis-core-ui`, projecto do ecossistema Normordis que fornece os componentes de interface utilizados pelas aplicações consumidoras. O kernel contribui com a camada de dados e semântica — estruturas de documento, metadados de língua, templates acessíveis e PDF/A — que suportam a conformidade na camada de apresentação.

Esta separação de responsabilidades é intencional: o kernel garante que os dados e documentos que chegam à UI têm a estrutura necessária para serem apresentados de forma acessível; o `normordis-core-ui` garante que essa apresentação cumpre os critérios WCAG 2.1 AA.

## Âmbito

Cobre o mapeamento entre os requisitos WCAG 2.1 / EN 301 549 e as responsabilidades do Normordis Kernel e do ecossistema Normordis versão `0.1.x`. Não cobre:

- Implementação de componentes UI — responsabilidade de `normordis-core-ui`
- Testes de acessibilidade de aplicações concretas — responsabilidade do integrador
- WCAG 2.2 (outubro 2023) — extensão de WCAG 2.1; retrocompatível; a adoptar em versão futura
- Acessibilidade de conteúdo em papel ou presencial

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [ACC-DL] | DL n.º 83/2018, de 19 de outubro | Art. 2.º–6.º | Acessibilidade de sítios web e apps móveis de organismos públicos; referência à EN 301 549 |
| [DIR-2102] | Directiva (UE) 2016/2102 | Art. 4.º–8.º | Requisitos de acessibilidade para o sector público; base do DL 83/2018 |
| [WCAG-21] | W3C WCAG 2.1 (ISO/IEC 40500:2012) | Critérios de Sucesso nível AA | Padrão técnico de acessibilidade web; incorporado pela EN 301 549 |
| [WCAG-22] | W3C WCAG 2.2 (outubro 2023) | Novos critérios AA | Extensão de WCAG 2.1; retrocompatível — a adoptar |
| [EN301549] | EN 301 549 v3.2.1 (2021) | Cláusulas 9–11 | Norma europeia harmonizada de acessibilidade TIC; referência técnica do DL 83/2018 |
| [WCAG-ARIA] | W3C ARIA 1.2 (Accessible Rich Internet Applications) | Integralmente | Atributos semânticos para componentes dinâmicos |

## Modelo de Responsabilidades no Ecossistema

A conformidade WCAG distribui-se por dois projectos complementares:

```
normordis-kernel  (este projecto)
    │
    ├── Dados semânticos e estruturados
    │     core-documental  → estrutura de documento com língua e metadados
    │     core-validation  → validação de atributos acessíveis (ex: alt text)
    │     support-typst-template → templates com estrutura semântica
    │
    └── Documentos acessíveis
          pdf/normordis-pdf¹  → PDF/A com tags de acessibilidade
          domain-numerador    → identificadores únicos para elementos UI

normordis-core-ui  (projecto de ecossistema)
    │
    ├── Componentes UI acessíveis
    │     WCAG 2.1 AA — Perceivable, Operable, Understandable, Robust
    │     ARIA 1.2 — semântica para componentes dinâmicos
    │     EN 301 549 cláusulas 9–11 — requisitos técnicos
    │
    └── Declaração de acessibilidade (DL 83/2018 Art. 8.º)
```

---

## Os Quatro Princípios WCAG (POUR)

### Princípio 1 — Perceptível

Informação e componentes de interface devem ser apresentáveis de formas que os utilizadores possam percepcionar.

| Critério | Nível | Responsabilidade | Implementação |
|----------|-------|-----------------|---------------|
| 1.1.1 Conteúdo não-textual (alt text) | A | `normordis-core-ui` + `core-documental` (metadados de imagem) | `normordis-core-ui` |
| 1.3.1 Informação e relações semânticas | A | `normordis-core-ui` + `support-typst-template` | `normordis-core-ui` |
| 1.3.2 Sequência com significado | A | `normordis-core-ui` | `normordis-core-ui` |
| 1.3.3 Características sensoriais | A | `normordis-core-ui` | `normordis-core-ui` |
| 1.3.4 Orientação | AA | `normordis-core-ui` | `normordis-core-ui` |
| 1.3.5 Identificação do objectivo de entrada | AA | `normordis-core-ui` | `normordis-core-ui` |
| 1.4.1 Uso de cor | A | `normordis-core-ui` | `normordis-core-ui` |
| 1.4.3 Contraste (mínimo) | AA | `normordis-core-ui` | `normordis-core-ui` |
| 1.4.4 Redimensionamento de texto | AA | `normordis-core-ui` | `normordis-core-ui` |
| 1.4.10 Refluxo | AA | `normordis-core-ui` | `normordis-core-ui` |
| 1.4.11 Contraste de componentes não-textuais | AA | `normordis-core-ui` | `normordis-core-ui` |

**Contribuição do kernel:** `support-typst-template` define a estrutura semântica dos templates documentais (títulos, secções, tabelas), que são renderizados em PDF/A com tags de acessibilidade via `normordis-pdf`.

### Princípio 2 — Operável

Componentes de interface e navegação devem ser operáveis.

| Critério | Nível | Responsabilidade | Implementação |
|----------|-------|-----------------|---------------|
| 2.1.1 Teclado | A | `normordis-core-ui` | `normordis-core-ui` |
| 2.1.2 Sem bloqueio de teclado | A | `normordis-core-ui` | `normordis-core-ui` |
| 2.4.1 Ignorar blocos | A | `normordis-core-ui` | `normordis-core-ui` |
| 2.4.2 Título de página | A | `normordis-core-ui` + `core-documental` (título do documento) | Partilhado |
| 2.4.3 Ordem de foco | A | `normordis-core-ui` | `normordis-core-ui` |
| 2.4.4 Objectivo da ligação | A | `normordis-core-ui` | `normordis-core-ui` |
| 2.4.6 Cabeçalhos e etiquetas | AA | `normordis-core-ui` + `support-typst-template` | Partilhado |
| 2.4.7 Foco visível | AA | `normordis-core-ui` | `normordis-core-ui` |
| 2.5.3 Etiqueta no nome | AA | `normordis-core-ui` | `normordis-core-ui` |

### Princípio 3 — Compreensível

Informação e operação da interface devem ser compreensíveis.

| Critério | Nível | Responsabilidade | Implementação |
|----------|-------|-----------------|---------------|
| 3.1.1 Língua da página | A | `core-documental` (campo `lang`) + `normordis-core-ui` | **Kernel** + `normordis-core-ui` |
| 3.1.2 Língua de partes | AA | `normordis-core-ui` | `normordis-core-ui` |
| 3.2.1 Em foco | A | `normordis-core-ui` | `normordis-core-ui` |
| 3.2.2 Em entrada | A | `normordis-core-ui` | `normordis-core-ui` |
| 3.2.3 Navegação consistente | AA | `normordis-core-ui` | `normordis-core-ui` |
| 3.3.1 Identificação de erros | A | `normordis-core-ui` + `core-validation` (mensagens de erro) | **Partilhado** |
| 3.3.2 Etiquetas ou instruções | A | `normordis-core-ui` | `normordis-core-ui` |
| 3.3.3 Sugestão de erro | AA | `normordis-core-ui` + `core-validation` | **Partilhado** |
| 3.3.4 Prevenção de erros | AA | `core-validation` + `normordis-core-ui` | **Partilhado** |

**Contribuição do kernel:** `core-validation` produz mensagens de erro canónicas, estruturadas e internacionalizáveis que `normordis-core-ui` apresenta de forma acessível (3.3.1, 3.3.3, 3.3.4). O campo `lang` em `core-documental` alimenta o atributo `lang` da página (3.1.1).

### Princípio 4 — Robusto

Conteúdo deve ser suficientemente robusto para ser interpretado por tecnologias de apoio.

| Critério | Nível | Responsabilidade | Implementação |
|----------|-------|-----------------|---------------|
| 4.1.1 Análise sintáctica | A | `normordis-core-ui` | `normordis-core-ui` |
| 4.1.2 Nome, função, valor | A | `normordis-core-ui` (ARIA) | `normordis-core-ui` |
| 4.1.3 Mensagens de estado | AA | `normordis-core-ui` + `support-errors` (mensagens canónicas) | **Partilhado** |

**Contribuição do kernel:** `support-errors` define mensagens de erro e estado canónicas (`PublicError`) com estrutura consistente, que `normordis-core-ui` expõe via ARIA live regions (4.1.3).

---

## Documentos Acessíveis — PDF/A com Tags

Os documentos gerados pelo kernel via `normordis-pdf` (ecossistema Normordis) produzem **PDF/A com tags de acessibilidade**, conformes com:

| Requisito | Norma | Implementação |
|-----------|-------|---------------|
| Estrutura lógica com tags | PDF/UA (ISO 14289) | `normordis-pdf` via Typst |
| Texto extraível por leitores de ecrã | WCAG 1.3.1 | `normordis-pdf` via Typst |
| Língua do documento definida | WCAG 3.1.1 | `core-documental` campo `lang` → `normordis-pdf` |
| Título do documento nos metadados | WCAG 2.4.2 | `core-documental` título → `normordis-pdf` |
| Contraste em templates | WCAG 1.4.3 | `support-typst-template` (paleta acessível) |
| Ordem de leitura correcta | WCAG 1.3.2 | `support-typst-template` (estrutura Typst) |

> ¹ **normordis-pdf** é um projecto autónomo do ecossistema Normordis. A conformidade PDF/A e PDF/UA é garantida por esse projecto.

---

## Declaração de Acessibilidade (DL 83/2018 Art. 8.º)

O DL 83/2018 obriga cada organismo público a publicar uma Declaração de Acessibilidade. O kernel suporta a geração desta declaração como documento institucional via `core-documental` + `normordis-pdf`. O conteúdo da declaração (estado de conformidade, conteúdos não acessíveis, mecanismo de contacto) é da responsabilidade do integrador.

---

## Matriz de Conformidade

| Requisito | Projecto | Evidência | Status |
|-----------|---------|-----------|--------|
| WCAG 2.1 AA — critérios de UI | `normordis-core-ui` | Componentes acessíveis | Implementado (ecossistema) |
| PDF/A com tags de acessibilidade | `normordis-pdf` | PDF/UA via Typst | Implementado (ecossistema) |
| Língua do documento (3.1.1) | `core-documental` (campo `lang`) | Metadado `lang` em todos os documentos | Implementado |
| Mensagens de erro acessíveis (3.3.1) | `core-validation` + `normordis-core-ui` | `PublicError` estruturado | Implementado |
| Mensagens de estado canónicas (4.1.3) | `support-errors` + `normordis-core-ui` | `PublicError`, ARIA live regions | Implementado |
| Templates com estrutura semântica | `support-typst-template` | Títulos, secções, tabelas em Typst | Implementado |
| Título do documento (2.4.2) | `core-documental` | Campo título obrigatório | Implementado |
| Declaração de Acessibilidade | `core-documental` + `normordis-pdf` | Template de declaração | Planeado |
| WCAG 2.2 — novos critérios AA | `normordis-core-ui` | — | Planeado |
| Testes automáticos de acessibilidade PDF (Tagged) | CI `test-linux` | `poppler-utils pdfinfo` | Implementado (observacional) |

## Limitações e Exclusões Conhecidas

- **Kernel é headless**: a esmagadora maioria dos critérios WCAG é responsabilidade de `normordis-core-ui`, não do kernel. O kernel contribui com semântica e dados, não com apresentação.
- **WCAG 2.2**: ainda não adoptado; os novos critérios AA (Focus Appearance, Dragging Movements, Target Size) são retrocompatíveis com WCAG 2.1 e serão incorporados em versão futura.
- **Testes automáticos PDF**: o job `test-linux` valida estrutura Tagged dos PDFs produzidos (requisito PDF/UA) via `poppler-utils`. Validação completa PDF/A + PDF/UA com veraPDF é um passo manual de release até o pipeline Typst produzir PDF/UA certificado. O gate reporta mas não bloqueia enquanto a maturação estiver em curso.
- **Testes automáticos UI**: a validação WCAG automática (ex: axe-core) aplica-se à camada UI e é responsabilidade do integrador e de `normordis-core-ui`.
- **Conteúdo de terceiros**: documentos importados via `core-ingest` podem não ser acessíveis — responsabilidade do produtor original.
- **Língua múltipla**: `core-documental` suporta o campo `lang`, mas a gestão de conteúdo multilingue dentro de um documento é responsabilidade da app integradora.

## Glossário

| Termo | Definição |
|-------|-----------|
| **A11Y** | Abreviatura de *accessibility* (11 letras entre o 'a' e o 'y') |
| **ARIA** | Accessible Rich Internet Applications — especificação W3C de atributos semânticos para conteúdo dinâmico |
| **AT** | Assistive Technology — tecnologias de apoio (leitores de ecrã, ampliadores, etc.) |
| **DL 83/2018** | Decreto-Lei que transpõe a Directiva (UE) 2016/2102; obriga organismos públicos à conformidade WCAG 2.1 AA |
| **EN 301 549** | Norma europeia harmonizada de acessibilidade para produtos e serviços TIC; incorpora WCAG 2.1 |
| **normordis-core-ui** | Projecto do ecossistema Normordis que implementa componentes de interface acessíveis (WCAG 2.1 AA) |
| **normordis-pdf** | Projecto do ecossistema Normordis que gera PDF/A com tags de acessibilidade via pipeline Typst |
| **PDF/A** | Subconjunto do PDF (ISO 19005) para preservação de longa duração |
| **PDF/UA** | PDF Universal Accessibility (ISO 14289) — PDF com tags de acessibilidade para leitores de ecrã |
| **POUR** | Perceivable, Operable, Understandable, Robust — os quatro princípios WCAG |
| **WCAG** | Web Content Accessibility Guidelines — norma W3C de acessibilidade (versão 2.1, nível AA obrigatório) |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — modelo de responsabilidades kernel / normordis-core-ui, POUR mapping |

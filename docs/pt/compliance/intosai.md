---
title: "Normordis Kernel — Conformidade INTOSAI GOV 9100"
type: compliance
framework: INTOSAI
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

# Normordis Kernel — Conformidade INTOSAI GOV 9100

## Sumário Executivo

O INTOSAI GOV 9100 — Guidelines for Internal Control Standards for the Public Sector (INTOSAI, 2004, revisto) é o quadro de referência de controlo interno adoptado pelas entidades fiscalizadoras superiores, incluindo o Tribunal de Contas português. Baseia-se no COSO 1992/2013 e adapta-o ao contexto do sector público, com ênfase reforçada em responsabilização (*accountability*), transparência, legalidade e orientação para o interesse público.

O Normordis Kernel já implementa o COSO 2013 como framework central (ver [docs/pt/compliance/coso.md](coso.md)). O INTOSAI GOV 9100 é o "COSO do sector público" — partilha os 5 componentes e reforça dimensões específicas relevantes para a AP: rastreabilidade financeira exigida pelo Tribunal de Contas, ética e integridade na função pública, e transparência para cidadãos e órgãos de controlo.

Este documento mapeia os reforços INTOSAI sobre o COSO e como o kernel os implementa.

## Âmbito

Cobre o mapeamento entre o INTOSAI GOV 9100 e o Normordis Kernel versão `0.1.x`, com foco nos elementos que vão além do COSO 2013. Não duplica o mapeamento COSO — este documento deve ser lido em conjunto com [docs/pt/compliance/coso.md](coso.md).

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [INTOSAI] | INTOSAI GOV 9100 — Guidelines for Internal Control Standards for the Public Sector | Secções I–IV | Framework de controlo interno para o sector público |
| [COSO-2013] | COSO Internal Control — Integrated Framework (2013) | Componentes 1–5 | Base do INTOSAI GOV 9100 |
| [TC] | Lei n.º 98/97, de 26 de agosto | Art. 1.º, 54.º–55.º | Competência do Tribunal de Contas; responsabilidade financeira |
| [ISSAI] | ISSAI 100 — Fundamental Principles of Public Sector Auditing | Integralmente | Princípios fundamentais de auditoria do sector público |

---

## INTOSAI GOV 9100 vs. COSO 2013 — Diferenças Chave

O INTOSAI GOV 9100 partilha os 5 componentes COSO mas reforça quatro dimensões específicas do sector público:

| Dimensão INTOSAI | Ênfase | Relevância para o kernel |
|-----------------|--------|--------------------------|
| **Accountability** | Responsabilização pública — os gestores públicos respondem perante cidadãos, parlamento e órgãos de controlo | `ActorId` em todos os eventos; audit trail disponível para Tribunal de Contas |
| **Transparência** | Informação disponível, verificável e compreensível para todos os stakeholders | `core-audit` imutável; documentação pública; `core-exports` |
| **Legalidade** | Conformidade estrita com a lei — controlo de legalidade precede controlo de eficiência | `core-org` (estrutura legal); `core-validation` (dados legais); ADRs |
| **Interesse público** | O controlo interno serve o cidadão, não apenas a organização | Toda a arquitectura orientada à AP — `domain-mef`, SNC-AP, arquivística |

---

## Mapeamento INTOSAI por Componente

### C1 — Ambiente de Controlo (reforços INTOSAI)

O INTOSAI reforça o ambiente de controlo com ênfase na ética da função pública e na independência dos controlos.

| Mecanismo INTOSAI | Crate | Implementação | Status |
|------------------|-------|---------------|--------|
| Ética e integridade — verificável tecnicamente | `core-audit` | Audit trail append-only: impossível apagar evidência de desvio | Implementado |
| Estrutura de autoridade e delegação legal | `core-org` | `PositionKind`, `substitutes` — delegação formal e rastreável | Implementado |
| Independência do controlo interno | `core-audit` (separado de `core-org`) | Auditoria é um crate separado e independente da lógica de negócio | Implementado |
| Responsabilização perante órgãos externos | `core-audit` + `core-exports` | Log disponível para exportação para auditores externos | Implementado |

### C2 — Avaliação de Risco (reforços INTOSAI)

O INTOSAI enfatiza o risco de não-conformidade legal como categoria de risco primária no sector público.

| Mecanismo INTOSAI | Crate | Implementação | Status |
|------------------|-------|---------------|--------|
| Risco de incumprimento legal | `core-validation` | Validação de requisitos legais (NIF, IBAN, datas) antes de persistência | Implementado |
| Risco de desvio financeiro (Tribunal de Contas) | `core-audit` | `ControlOutcome` regista desvios; cadeia de evidência imutável | Implementado |
| Risco de fraude e corrupção | `core-audit` (append-only + hash) | Adulteração retroactiva detectável; dead-letter para falhas | Implementado |
| Risco de mudança normativa | `domain-mef` (tabela temporal) | MEF com vigência temporal — adaptação a diplomas legais novos | Implementado |

### C3 — Actividades de Controlo (reforços INTOSAI)

O INTOSAI reforça os controlos de legalidade e as aprovações formais.

| Mecanismo INTOSAI | Crate | Implementação | Status |
|------------------|-------|---------------|--------|
| Controlo de legalidade de operações | `core-validation` + `control_id` | Cada operação tem contexto de controlo explícito com referência legal | Implementado |
| Aprovação formal documentada | `core-org` + `core-audit` | Aprovações por posição hierárquica registadas como `AuditEvent` | Implementado |
| Segregação de funções | `core-org` (`PositionKind`) | Funções distintas com responsabilidades explícitas | Implementado (base) |
| Controlo de numeração de documentos | `domain-numerador` | Sequências únicas e auditáveis — impossível reutilizar números | Implementado |

### C4 — Informação e Comunicação (reforços INTOSAI)

O INTOSAI enfatiza a comunicação com órgãos de controlo externos (Tribunal de Contas, órgão de controlo interno).

| Mecanismo INTOSAI | Crate | Implementação | Status |
|------------------|-------|---------------|--------|
| Informação para controlo externo | `core-audit` + `core-exports` | Log exportável para auditores do Tribunal de Contas | Implementado |
| Rastreabilidade financeira | `core-audit` (referência `control_id` financeiro) | Cada operação financeira liga a evidência de controlo | Implementado |
| Comunicação de irregularidades | `core-audit` (dead-letter + `ControlOutcome::Failed`) | Irregularidades registadas, nunca silenciadas | Implementado |
| Relatórios para cidadãos (transparência) | `core-exports`, `core-documental` | Exportação em formatos abertos; documentos acessíveis | Implementado (base) |

### C5 — Monitorização (reforços INTOSAI)

O INTOSAI distingue monitorização contínua de avaliações independentes periódicas — estas últimas fundamentais para o sector público.

| Mecanismo INTOSAI | Crate | Implementação | Status |
|------------------|-------|---------------|--------|
| Monitorização contínua | `domain-telemetry`, `core-metrics` | Telemetria e métricas de runtime | Implementado |
| Avaliação independente periódica | `core-audit` (exportável) | Log disponível para auditores externos em qualquer momento | Implementado |
| Auto-avaliação de eficácia de controlos | `core-audit` (`ControlOutcome` agregado) | Histórico de outcomes por `control_id` | Implementado (base) |
| Comunicação de deficiências ao nível adequado | `core-audit` (dead-letter) + `SECURITY.md` | Deficiências persistidas e processo de escalada documentado | Implementado |

---

## Alinhamento com o Tribunal de Contas (Lei 98/97)

O Tribunal de Contas exerce controlo jurisdicional das despesas públicas e responsabilização financeira. O kernel fornece o substrato técnico de evidência que as entidades controladas precisam de apresentar:

| Obrigação (Lei 98/97) | Mecanismo kernel | Crate | Status |
|----------------------|-----------------|-------|--------|
| Rastreabilidade de despesas (Art. 1.º) | `AuditEvent` com `control_id` financeiro | `core-audit` | Implementado |
| Evidência de controlo como defesa de responsabilidade (Art. 54.º–55.º) | Audit trail imutável com hash chain | `core-audit` | Implementado |
| Documentação financeira verificável | Pipeline documental + numeração | `core-documental`, `domain-numerador` | Implementado |
| Exportação de dados para fiscalização | Snapshots auditáveis em formatos abertos | `core-exports` | Implementado |

---

## Matriz de Conformidade

| Requisito INTOSAI | Crate / Módulo | Evidência | Status |
|------------------|---------------|-----------|--------|
| Accountability — actor em cada evento | `core-audit` | `ControlExecution.actor: ActorId` | Implementado |
| Accountability — log disponível para controlo externo | `core-audit` + `core-exports` | Exportação auditável | Implementado |
| Transparência — audit trail imutável | `core-audit` | Append-only + hash chain | Implementado |
| Legalidade — validação de requisitos legais | `core-validation` | NIF, IBAN, datas — conformidade antes de persistência | Implementado |
| Legalidade — estrutura de autoridade formal | `core-org` | `PositionKind`, `substitutes`, `OrgAuditAdapter` | Implementado |
| Risco de fraude — imutabilidade | `core-audit` | Sem UPDATE/DELETE; hash encadeado | Implementado |
| Risco de mudança normativa | `domain-mef` | Tabela temporal com diplomas legais | Implementado |
| Controlo de numeração (segregação) | `domain-numerador` | Sequências únicas por série | Implementado |
| Comunicação de irregularidades | `core-audit` (dead-letter) | Nunca silenciadas | Implementado |
| Monitorização independente | `core-audit` (exportável) | Log disponível para auditores externos | Implementado |
| Tribunal de Contas — rastreabilidade financeira | `core-audit`, `core-org` | `control_id`, `AuditEvent` financeiro | Implementado |
| Segregação de funções granular | — | `OrgAuthorizationPort` (planeado) | Planeado |
| Auto-avaliação formal de eficácia | — | Agregação de `ControlOutcome` por período | Planeado |

## Limitações e Exclusões Conhecidas

- **Segregação de funções granular**: a `core-org` modela a estrutura de funções mas a aplicação de regras de segregação (ex: quem aprova não pode executar) depende de `OrgAuthorizationPort`, identificado como trabalho futuro.
- **Auto-avaliação formal**: a agregação de `ControlOutcome` por período para relatórios de auto-avaliação de eficácia está identificada como trabalho futuro.
- **INTOSAI ISSAI 100**: os princípios de auditoria do sector público (ISSAI 100) são aplicáveis às entidades fiscalizadoras externas (Tribunal de Contas), não ao kernel como plataforma.
- **Controlo interno vs. auditoria externa**: o kernel implementa controlo interno; a auditoria externa (Tribunal de Contas) usa o audit trail do kernel mas é um processo independente.

## Glossário

| Termo | Definição |
|-------|-----------|
| **Accountability** | Responsabilização — obrigação de prestar contas dos recursos públicos e das decisões tomadas |
| **INTOSAI** | International Organization of Supreme Audit Institutions — organização internacional das entidades fiscalizadoras superiores |
| **INTOSAI GOV 9100** | Guidelines for Internal Control Standards for the Public Sector — adaptação do COSO ao sector público |
| **ISSAI** | International Standards of Supreme Audit Institutions — normas internacionais de auditoria do sector público |
| **Segregação de funções** | Princípio de controlo interno que separa responsabilidades incompatíveis (ex: quem autoriza não executa; quem executa não regista) |
| **Tribunal de Contas** | Órgão supremo de fiscalização financeira do Estado português; exerce controlo jurisdicional e de legalidade |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — reforços INTOSAI sobre COSO, Tribunal de Contas, accountability, transparência |

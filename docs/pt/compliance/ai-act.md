---
title: "Normordis Kernel — Conformidade AI Act"
type: compliance
framework: AI-ACT
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
  - docs/pt/compliance/nis2.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade AI Act

## Declaração de Posicionamento

**O Normordis Kernel não é um sistema de inteligência artificial.** É uma plataforma de suporte headless para aplicações da Administração Pública. O kernel não toma decisões automatizadas, não treina modelos, não classifica indivíduos e não executa inferência.

O que o kernel faz é fornecer a **infra-estrutura de conformidade AI Act** que os sistemas de IA construídos sobre ele necessitam: logging imutável e rastreável (Art. 12.º), governação de dados auditável (Art. 10.º), estrutura de supervisão humana (Art. 14.º), gestão de risco baseada em evidência (Art. 9.º) e sistema de qualidade verificável (Art. 17.º).

Esta distinção é fundamental: o kernel não é o objecto de regulação do AI Act — é a plataforma que permite que as aplicações reguladas cumpram os seus requisitos.

---

## Sumário Executivo

O Regulamento (UE) 2024/1689 (AI Act) estabelece um quadro de regulação de sistemas de inteligência artificial baseado em risco. Para a Administração Pública portuguesa, os sistemas de maior relevância são os de **alto risco** listados no Anexo III — que abrangem decisões em emprego, acesso a serviços essenciais, infraestrutura crítica e justiça.

Aplicações construídas sobre o Normordis Kernel que incorporem IA de alto risco (por exemplo, sistemas de apoio a decisões de RH, classificação de processos administrativos, ou priorização de serviços) ficam sujeitas às obrigações dos Arts. 9.º a 17.º. O kernel fornece os alicerces técnicos para satisfazer estas obrigações.

A presente documentação serve dois propósitos: clarificar o posicionamento do kernel no ecossistema AI Act, e demonstrar que a infra-estrutura de conformidade AI Act está disponível para as aplicações integradoras.

## Âmbito

Cobre o mapeamento entre os requisitos do AI Act e a infra-estrutura de conformidade fornecida pelo Normordis Kernel versão `0.1.x`. Não cobre:

- Conformidade AI Act das aplicações consumidoras — responsabilidade do fornecedor/operador do sistema de IA
- Sistemas de IA de risco mínimo ou limitado — requisitos simplificados, sem impacto significativo no kernel
- Modelos de IA de uso geral (GPAI, Arts. 53.º–55.º) — aplicável a fornecedores de modelos, não a plataformas de suporte
- Avaliação de conformidade e certificação — responsabilidade do fornecedor do sistema de IA de alto risco

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [AI-ACT-9] | Regulamento (UE) 2024/1689 | Art. 9.º | Sistema de gestão de risco para IA de alto risco |
| [AI-ACT-10] | Regulamento (UE) 2024/1689 | Art. 10.º | Dados e governação de dados |
| [AI-ACT-11] | Regulamento (UE) 2024/1689 | Art. 11.º | Documentação técnica |
| [AI-ACT-12] | Regulamento (UE) 2024/1689 | Art. 12.º | Manutenção de registos — logging automático |
| [AI-ACT-13] | Regulamento (UE) 2024/1689 | Art. 13.º | Transparência e informação aos operadores |
| [AI-ACT-14] | Regulamento (UE) 2024/1689 | Art. 14.º | Supervisão humana |
| [AI-ACT-15] | Regulamento (UE) 2024/1689 | Art. 15.º | Exactidão, robustez e cibersegurança |
| [AI-ACT-17] | Regulamento (UE) 2024/1689 | Art. 17.º | Sistema de gestão da qualidade |
| [AI-ACT-72] | Regulamento (UE) 2024/1689 | Art. 72.º–73.º | Monitorização pós-comercialização, reporte de incidentes graves |

**Calendário de aplicação:**

| Data | Aplicação |
|------|-----------|
| Agosto 2024 | Regulamento em vigor |
| Fevereiro 2025 | Capítulo I (definições) e II (IA proibida) — já aplicáveis |
| Agosto 2025 | Modelos GPAI (Arts. 53.º–55.º) — já aplicáveis |
| Agosto 2026 | IA de alto risco — Anexo III (emprego, serviços essenciais, infraestrutura, etc.) |
| Agosto 2027 | IA de alto risco — Anexo I (produtos de segurança) |

---

## Posicionamento do Kernel no Ecossistema AI Act

### Categorias de actores

O AI Act distingue fornecedores (*providers*), operadores (*deployers*), importadores e distribuidores. O kernel enquadra-se como:

- **Componente de infraestrutura** — não é um sistema de IA, não é fornecedor nem operador de IA
- **Plataforma de suporte** — fornece a camada técnica sobre a qual sistemas de IA conformes podem ser construídos

```
Fornecedor de sistema IA de alto risco
    │  usa
    ▼
Aplicação consumidora do kernel
    │  usa
    ▼
Normordis Kernel  ←  infra-estrutura de conformidade AI Act
    │
    ├── core-audit     → Art. 12.º (logging imutável)
    ├── core-validation → Art. 10.º (qualidade de dados)
    ├── core-org        → Art. 14.º (supervisão humana)
    ├── core-config     → Art. 9.º  (gestão de risco)
    ├── core-documental → Art. 11.º (documentação técnica)
    ├── core-metrics    → Art. 72.º (monitorização)
    └── domain-telemetry → Art. 72.º (monitorização)
```

### Sistemas de alto risco relevantes para a AP (Anexo III)

As seguintes categorias do Anexo III são directamente relevantes para aplicações da AP que usem o kernel:

| Categoria Anexo III | Exemplos AP | Crates kernel mais relevantes |
|--------------------|-------------|-------------------------------|
| §4 — Emprego e gestão de trabalhadores | Apoio a decisões de recrutamento, avaliação de desempenho | `core-rh`, `core-org`, `core-audit` |
| §5 — Acesso a serviços essenciais | Classificação de pedidos, priorização de atendimento | `core-audit`, `core-validation` |
| §2 — Infraestrutura crítica | Sistemas de gestão de infra-estrutura pública | `core-audit`, `core-config` |
| §6 — Aplicação da lei | Apoio a decisões de fiscalização | `core-audit`, `core-org` |
| §8 — Administração da justiça | Apoio a decisões judiciais ou administrativas | `core-audit`, `core-documental` |

---

## Mapeamento por Artigo AI Act

### Art. 9.º — Sistema de Gestão de Risco

O Art. 9.º exige que os fornecedores de IA de alto risco implementem um sistema contínuo de gestão de risco ao longo do ciclo de vida do sistema.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Registo de outcomes de controlo | `core-audit` | `ControlOutcome { Passed, Failed, Skipped }` — risco materializado | Implementado |
| Contexto de falha estruturado | `core-audit` | `ControlExecution.context` — dados do desvio para análise | Implementado |
| Gestão de risco de configuração | `core-config` | Invariantes verificados; desvios registados | Implementado |
| Evidência persistente (dead-letter) | `core-audit` | Falhas de controlo nunca perdidas | Implementado |
| Métricas de risco operacional | `core-metrics` | Indicadores contínuos de comportamento | Implementado |

### Art. 10.º — Dados e Governação de Dados

O Art. 10.º impõe práticas de governação de dados para sistemas de IA de alto risco, incluindo exames de dados quanto a possíveis enviesamentos e gestão da qualidade.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Validação canónica de dados | `core-validation` | NIF, IBAN, e-mail, datas, UUID — validação antes de persistência | Implementado |
| Rastreabilidade de dados pessoais | `core-rh` | Dados de pessoas geridos com ciclo de vida auditado | Implementado |
| Separação dados pessoais / eventos | Arquitectura | `ActorId` opaco; dados pessoais isolados em `core-rh` | Implementado |
| Minimização de dados (RGPD/AI Act) | `core-validation` | Validação sem armazenamento desnecessário | Implementado |
| Registo de proveniência de dados | `core-audit` | Actor e timestamp em cada operação sobre dados | Implementado |

### Art. 11.º — Documentação Técnica

O Art. 11.º exige documentação técnica completa antes da colocação no mercado de um sistema de IA de alto risco, mantida actualizada durante o ciclo de vida.

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Documentação arquitectural | `docs/architecture/` | `overview.md`, `crate-map.md` — actualizados por versão | Implementado |
| Decisões técnicas documentadas | `docs/adr/` | ADRs formalizados com raciocínio e alternativas | Implementado |
| Documentação de conformidade | `docs/pt/compliance/` | Este conjunto de documentos | Implementado |
| CHANGELOG versionado | Raiz | Histórico de alterações por versão | Implementado |
| Numeração de documentos | `domain-numerador` | Numeração sequencial de documentação institucional | Implementado |
| Ciclo de vida documental | `core-documental` | Versões, estados e log de eventos de documentos técnicos | Implementado |

### Art. 12.º — Manutenção de Registos (Logging)

**Esta é a área de maior contribuição directa do kernel.** O Art. 12.º exige que sistemas de IA de alto risco registem automaticamente eventos ao longo do seu funcionamento, com nível de detalhe suficiente para verificação posterior de conformidade.

| Requisito Art. 12.º | Mecanismo kernel | Crate | Status |
|--------------------|-----------------|-------|--------|
| Logs automáticos durante funcionamento | `AuditEvent` gerado atomicamente com operação | `core-audit` | Implementado |
| Identificação do sistema e versão | `control_id` + versão do kernel | `core-audit` | Implementado |
| Data e hora de cada operação | `ControlExecution.timestamp` | `core-audit` | Implementado |
| Dados de entrada relevantes | `ControlExecution.context` — dados estruturados | `core-audit` | Implementado |
| Identificação das pessoas envolvidas | `ControlExecution.actor: ActorId` | `core-audit` | Implementado |
| Imutabilidade dos logs | `AuditStore` append-only + hash encadeado | `core-audit` | Implementado |
| Retenção de logs pelo período exigido | `AuditStore` (SQLite persistente) | `core-audit` | Implementado |
| Evidência de incidentes | Dead-letter — falhas nunca descartadas | `core-audit` | Implementado |

O modelo de logging do kernel para sistemas de IA:

```
Decisão do sistema de IA
    │
    └─► AuditEvent {
            control_id:  "ai.decision.employment.screening.v1",
            outcome:     ControlOutcome::Passed,
            execution:   ControlExecution {
                actor:     ActorId("uuid-do-operador"),
                timestamp: Timestamp::now(),
                context:   json!({
                    "model_version": "1.2.3",
                    "input_hash":    "sha256:...",
                    "confidence":    0.92,
                    "decision":      "approved",
                    "human_review":  false
                })
            }
        }
        → Outbox transaccional (mesmo commit da decisão)
        → AuditStore (SQLite cifrado, append-only)
        → Hash encadeado (adulteração detectável)
```

### Art. 13.º — Transparência e Informação aos Operadores

O Art. 13.º exige que sistemas de IA de alto risco sejam suficientemente transparentes para que os operadores interpretem os outputs e os usem adequadamente.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| `outcome` de cada decisão | `core-audit` | `ControlOutcome` — resultado explícito e estruturado | Implementado |
| `control_id` como referência de controlo | `core-audit` | Liga cada decisão ao controlo que a governa | Implementado |
| Contexto estruturado da decisão | `core-audit` | `ControlExecution.context` — dados de entrada e saída | Implementado |
| Exportação de dados para revisão | `core-exports` | Dados de operação exportáveis para auditor | Implementado |
| Documentação da API | `docs/` | Contratos públicos, ADRs, compliance docs | Implementado |

### Art. 14.º — Supervisão Humana

O Art. 14.º exige que sistemas de IA de alto risco sejam concebidos para permitir supervisão humana efectiva, incluindo a capacidade de interromper ou inverter o sistema.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Estrutura de supervisão orgânica | `core-org` | `PositionKind`, hierarquia de autoridade | Implementado |
| Substituição legal de supervisores | `core-org` | `substitutes` — delegação formal e rastreável | Implementado |
| Audit trail de decisões de supervisão | `core-org` + `core-audit` | `OrgAuditAdapter` — cada acção de supervisão auditada | Implementado |
| Atribuição de pessoas a funções de supervisão | `core-rh` | `PersonAssignment` — vincula supervisor à posição | Implementado |
| Registo de intervenção humana | `core-audit` | `AuditEvent` para override ou interrupção de sistema IA | Implementado (suporte) |
| Autorização de acções de supervisão | — | `OrgAuthorizationPort` (planeado) | Planeado |

### Art. 15.º — Exactidão, Robustez e Cibersegurança

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Validação de dados de entrada | `core-validation` | Validação canónica antes de qualquer operação | Implementado |
| OCC — detecção de conflitos | `core-org` | Modificações concorrentes detectadas explicitamente | Implementado |
| Cibersegurança (ver NIS2) | `support-crypto`, `infra` | XChaCha20-Poly1305, Argon2id, supply chain | Implementado |
| Integridade de logs (anti-adulteração) | `core-audit` | Hash encadeado verificável | Implementado |
| Configuração validada | `core-config` | Invariantes verificados; sistema não arranca com config inválida | Implementado |

### Art. 17.º — Sistema de Gestão da Qualidade

O Art. 17.º exige que os fornecedores de IA de alto risco implementem um sistema de gestão da qualidade que cubra o ciclo de vida completo.

| Mecanismo | Componente | Implementação | Status |
|-----------|-----------|---------------|--------|
| Gates de release obrigatórios | CI pipeline | `fmt + clippy(-D warnings) + test + release` | Implementado |
| Testes automatizados | CI pipeline | >200 testes por commit | Implementado |
| Decisões de design documentadas | `docs/adr/` | ADRs com raciocínio, alternativas e contexto | Implementado |
| Versionamento semântico | `CHANGELOG` | Histórico público de alterações por versão | Implementado |
| Auditoria de dependências | `cargo audit` + `cargo deny` | Vulnerabilidades e política de licenças | Implementado |
| Processo de contribuição | `CONTRIBUTING.md` | Guidelines de qualidade de código | Implementado |
| Rastreabilidade de documentos | `core-documental` + `domain-numerador` | Numeração e ciclo de vida de documentação | Implementado |

### Arts. 72.º–73.º — Monitorização Pós-Comercialização e Incidentes Graves

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Telemetria de uso em produção | `domain-telemetry` | Eventos de uso para detecção de desvios | Implementado |
| Métricas operacionais contínuas | `core-metrics` | KPIs de runtime para monitorização | Implementado |
| Registo de incidentes graves | `core-audit` | `ControlOutcome::Failed` + dead-letter | Implementado |
| Rastreabilidade para análise post-hoc | `core-audit` | Audit trail imutável disponível para investigação | Implementado |
| Reporte de incidentes graves ao regulador | — | Mecanismo externo estruturado | Planeado |

---

## Matriz de Conformidade

| Requisito AI Act | Crate / Componente | Evidência | Status |
|-----------------|-------------------|-----------|--------|
| Logging automático (Art. 12.º) | `core-audit` | `AuditEvent` atómico, append-only, hash chain | Implementado |
| Actor identificado em cada log (Art. 12.º) | `core-audit` | `ControlExecution.actor: ActorId` | Implementado |
| Timestamp em cada log (Art. 12.º) | `core-audit` | `ControlExecution.timestamp` | Implementado |
| Imutabilidade dos logs (Art. 12.º) | `core-audit` | `AuditStore` append-only + hash encadeado | Implementado |
| Dados de entrada em logs (Art. 12.º) | `core-audit` | `ControlExecution.context` | Implementado |
| Qualidade e validação de dados (Art. 10.º) | `core-validation` | Validadores canónicos NIF, IBAN, e-mail, datas | Implementado |
| Governação de dados pessoais (Art. 10.º) | `core-rh` | Ciclo de vida auditado; separação ActorId/dados | Implementado |
| Gestão de risco contínua (Art. 9.º) | `core-audit`, `core-config` | Outcomes, dead-letter, invariantes | Implementado |
| Supervisão humana — estrutura (Art. 14.º) | `core-org` | Hierarquia, substituições, audit trail | Implementado |
| Supervisão humana — atribuição (Art. 14.º) | `core-rh` | `PersonAssignment` vincula supervisor | Implementado |
| Transparência de outputs (Art. 13.º) | `core-audit` | `control_id`, `outcome`, contexto estruturado | Implementado |
| Documentação técnica (Art. 11.º) | `docs/`, `core-documental` | ADRs, compliance docs, CHANGELOG | Implementado |
| Sistema de qualidade (Art. 17.º) | CI pipeline, ADRs | Gates obrigatórios, testes, versionamento | Implementado |
| Monitorização pós-comercialização (Art. 72.º) | `domain-telemetry`, `core-metrics` | Telemetria e métricas contínuas | Implementado |
| Cibersegurança (Art. 15.º) | `support-crypto`, NIS2 | XChaCha20, Argon2id, supply chain | Implementado |
| Exportação para auditoria (Art. 13.º) | `core-exports` | Exportação estruturada de dados de operação | Implementado |
| Reporte de incidentes graves (Art. 73.º) | — | Mecanismo externo formal | Planeado |
| Autorização granular (Art. 14.º) | — | `OrgAuthorizationPort` (planeado) | Planeado |

## Limitações e Exclusões Conhecidas

- **O kernel não é um sistema de IA**: não toma decisões de IA, não treina modelos, não executa inferência. Qualquer avaliação de conformidade AI Act aplica-se ao sistema de IA construído sobre o kernel, não ao kernel em si.
- **Reporte de incidentes graves (Art. 73.º)**: o kernel detecta e regista incidentes mas não implementa o mecanismo de notificação formal ao regulador. É responsabilidade do fornecedor do sistema de IA.
- **Avaliação de conformidade e certificação**: os procedimentos de avaliação de conformidade para IA de alto risco (notificação a organismos notificados, declaração EU de conformidade) são responsabilidade do fornecedor do sistema de IA.
- **Modelos GPAI (Arts. 53.º–55.º)**: não aplicável ao kernel; aplicável aos fornecedores de modelos de linguagem ou outros modelos de uso geral.
- **Dados de treino (Art. 10.º)**: o kernel não gere dados de treino de modelos de IA. Esta é uma responsabilidade do fornecedor do sistema de IA.
- **Calendário de aplicação**: os requisitos para IA de alto risco (Anexo III) são aplicáveis a partir de agosto de 2026. Este documento será revisto em conformidade.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AI Act** | Regulamento (UE) 2024/1689 — primeiro quadro regulatório global para sistemas de inteligência artificial, baseado em níveis de risco |
| **Deployer** | Operador — entidade que usa um sistema de IA sob a sua responsabilidade |
| **GPAI** | General Purpose AI — modelo de IA de uso geral (ex: modelos de linguagem); sujeitos aos Arts. 53.º–55.º a partir de agosto de 2025 |
| **IA de alto risco** | Sistema de IA listado no Anexo I ou Anexo III do AI Act; sujeito a requisitos rigorosos de conformidade |
| **IA proibida** | Sistemas de IA proibidos pelo Art. 5.º (ex: pontuação social generalizada, manipulação subliminar) |
| **Organismo notificado** | Entidade de avaliação de conformidade acreditada para certificar sistemas de IA de alto risco |
| **Provider** | Fornecedor — entidade que desenvolve e coloca no mercado um sistema de IA |
| **Supervisão humana** | Capacidade de pessoas autorizadas monitorizar, intervir, interromper ou inverter um sistema de IA de alto risco |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — posicionamento kernel/AI Act, mapeamento Arts. 9.º–17.º, 72.º–73.º, modelo de logging |

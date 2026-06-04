---
title: "Normordis Kernel — Conformidade COSO: Mapeamento Detalhado"
type: compliance
framework: COSO
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/intosai.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade COSO: Mapeamento Detalhado

## Sumário Executivo

O Normordis Kernel adopta o COSO Internal Control — Integrated Framework (edição 2013) como framework central de controlo interno. Cada crate do kernel foi concebido para gerar evidência verificável alinhada com os 5 componentes e 17 princípios COSO.

A abordagem distingue-se de implementações tradicionais: o COSO é tratado como **restrição arquitectural**, não como requisito de auditoria posterior. Isto significa que evidência de controlo é produzida atomicamente com cada operação de domínio — via outbox transaccional — e não reconstituída após o facto.

Este documento detalha o mapeamento entre os 17 princípios COSO e a implementação concreta no kernel, crate a crate.

## Declaração de Conformidade COSO

O Normordis Kernel implementa os **5 componentes e 17 princípios** do COSO Internal Control — Integrated Framework (2013) como restrições arquitecturais de primeira classe. A conformidade não é declarada por auto-avaliação posterior — é uma propriedade emergente do design: os invariantes técnicos do kernel tornam fisicamente impossível executar uma operação auditável sem produzir evidência de controlo.

### Invariantes arquitecturais que garantem conformidade

Os seguintes invariantes são garantidos pelo compilador Rust ou pela arquitectura de persistência, não por convenção ou disciplina operacional:

| Invariante | Garantia | Princípio COSO |
|------------|----------|----------------|
| `AuditEvent` escrito no mesmo commit da operação de domínio | Evidência nunca separável da operação — não existe janela de inconsistência | P10, P14 |
| `AuditStore` é append-only — sem `UPDATE` nem `DELETE` | Evidência imutável após escrita — adulteração retroactiva impossível | P1, P8 |
| `control_id` obrigatório em todas as operações auditáveis | Nenhuma operação de negócio pode ocorrer sem contexto de controlo | P10 |
| `actor: ActorId` obrigatório em `ControlExecution` | Responsabilização não é opcional — toda a acção tem um actor identificado | P5 |
| Dead-letter persiste eventos que falham entrega | Deficiências de comunicação nunca são silenciadas | P17 |
| Hash encadeado de eventos | Adulteração do log detectável em qualquer ponto da cadeia | P1, P8 |
| OCC via versão de agregado em `core-org` | Modificações concorrentes geram conflito explícito — nunca silencioso | P9 |
| `clippy -D warnings` + testes obrigatórios no CI | O código que quebra invariantes não pode ser integrado | P11, P12 |

### Cobertura de testes por crate de controlo

A suíte de testes verifica automaticamente que os invariantes se mantêm após cada alteração:

| Crate | Testes | Cobertura principal |
|-------|--------|---------------------|
| `core-audit` | 70 | `AuditStore` append-only, outbox, dead-letter, hash chain |
| `core-rh` | 119 | `PersonAssignment`, `UserRepository`, integração SQLite com atomicidade |
| `core-config` | 65 | Invariantes de configuração, rejeição de config inválida |
| `core-validation` | Enterprise-grade | Validadores canónicos (NIF, IBAN, email, UUID, datas) |
| `core-org` | — | `OrgAuditAdapter`, OCC, substituições legais, eventos de domínio |

### Escopo da conformidade

O kernel é **COSO-compliant ao nível de plataforma**: garante que as aplicações construídas sobre ele têm os alicerces técnicos necessários para controlo interno efectivo. A conformidade COSO a nível organizacional — processos, supervisão do conselho, cultura de controlo — é responsabilidade do integrador e da organização, não do kernel.

---

## Âmbito

Cobre o mapeamento completo do COSO 2013 ao Normordis Kernel versão `0.1.x`. Não cobre:

- Avaliação de conformidade COSO das aplicações consumidoras (responsabilidade do integrador)
- Implementação de INTOSAI GOV 9100 (complementar ao COSO para o sector público — documento separado)
- Processos organizacionais externos ao kernel (ex: comités de risco, supervisão do conselho)

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [COSO-2013] | COSO Internal Control — Integrated Framework (2013) | Componentes 1–5, Princípios 1–17 | Framework central de controlo interno |
| [INTOSAI] | INTOSAI GOV 9100 — Guidelines for Internal Control Standards | Secções I–IV | Complemento COSO para sector público |
| [TC] | Lei n.º 98/97, de 26 de agosto | Art. 54.º–55.º | Responsabilidade financeira — evidência de controlo como defesa |

## O Framework COSO no Contexto do Kernel

### Estrutura do COSO 2013

O COSO 2013 organiza o controlo interno em **5 componentes** e **17 princípios**:

```
┌─────────────────────────────────────────────────────┐
│  OBJECTIVOS: Operações · Reporte · Conformidade      │
├─────────────────────────────────────────────────────┤
│  C1: Ambiente de Controlo      (Princípios 1–5)      │
│  C2: Avaliação de Risco        (Princípios 6–9)      │
│  C3: Actividades de Controlo   (Princípios 10–12)    │
│  C4: Informação e Comunicação  (Princípios 13–15)    │
│  C5: Monitorização             (Princípios 16–17)    │
└─────────────────────────────────────────────────────┘
```

### Modelo de Evidência Embebida

O kernel não apenas **suporta** o COSO — **produz** evidência COSO atomicamente:

```
Operação de domínio
    │
    ├─► Estado persistido (SQLite cifrado)
    │
    └─► AuditEvent {
            control_id: ControlId,     // liga ao princípio COSO
            outcome: ControlOutcome,   // Passed / Failed / Skipped
            execution: ControlExecution {
                actor: ActorId,
                timestamp: Timestamp,
                context: serde_json::Value,
            }
        }
        │
        ├─► Outbox transaccional (mesmo commit)
        └─► Dead-letter (se entrega falhar)
```

A garantia fundamental é: **se a operação foi persistida, a evidência foi persistida**. Não existe janela de inconsistência.

---

## Componente 1 — Ambiente de Controlo

> *A organização demonstra compromisso com integridade e valores éticos, e a gestão estabelece estrutura, autoridade e responsabilidade.*

### P1 — Compromisso com integridade e valores éticos

**Relevância para o kernel:** O kernel não pode substituir a cultura organizacional, mas fornece os instrumentos técnicos que tornam a integridade verificável e auditável.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Audit trail append-only | `core-audit` | `AuditStore` — escrita apenas, sem actualização ou deleção |
| Cadeia de hashes verificável | `core-audit` | Hash encadeado de eventos (integridade do log) |
| Cifra em repouso | `infra` (SQLite) | SQLite cifrado — impede acesso directo não autorizado |
| Configuração validada | `core-config` | Invariantes verificados em startup; desvios registados |

**Status:** Implementado

### P2 — Independência e supervisão do conselho

**Relevância para o kernel:** Fora do âmbito técnico directo. O kernel fornece o substrato de informação que suporta a supervisão independente.

**Status:** Parcial (suporte técnico disponível; estrutura de supervisão é responsabilidade organizacional)

### P3 — Estrutura, autoridade e responsabilidade

**Relevância para o kernel:** `core-org` implementa a estrutura organizacional formal com rastreabilidade de posições, responsabilidades e substituições legais.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Estrutura organizacional | `core-org` | Hierarquia de posições (`PositionKind`) e unidades orgânicas |
| Substituição legal | `core-org` | `substitutes` — delegação formal de autoridade rastreável |
| Auditoria de mudanças org. | `core-org` | `OrgAuditAdapter` — cada alteração gera `AuditEvent` |
| Controlo de concorrência | `core-org` | OCC via versão de agregado — previne conflitos silenciosos |

**Status:** Implementado

### P4 — Compromisso com competência

**Relevância para o kernel:** `core-rh` rastreia atribuições de pessoas a funções, permitindo verificar que funções estão preenchidas por pessoas com perfil adequado.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Atribuição de pessoas | `core-rh` | `PersonAssignment` — vincula pessoa a posição/função |
| Repositório de utilizadores | `core-rh` | `UserRepository` — gestão do ciclo de vida de utilizadores |

**Status:** Implementado (base); roles com evidência COSO planeados

### P5 — Responsabilização

**Relevância para o kernel:** Toda a operação auditável é associada a um `ActorId` — a responsabilização é um atributo de primeira classe de cada `AuditEvent`.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Actor em cada evento | `core-audit` | `ControlExecution.actor: ActorId` |
| Imutabilidade do log | `core-audit` | `AuditStore` append-only |
| Rastreabilidade de utilizadores | `core-rh` | `UserService` — ciclo de vida auditado |

**Status:** Implementado

---

## Componente 2 — Avaliação de Risco

> *A organização especifica objectivos adequados, identifica e analisa riscos, e avalia riscos de fraude e mudanças significativas.*

### P6 — Objectivos adequados

**Relevância para o kernel:** Os objectivos de controlo estão embebidos nos `control_id` — cada identificador de controlo mapeia para um objectivo de negócio ou conformidade.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Identificadores de controlo | `core-audit` | `ControlId` — taxonomia de objectivos de controlo |
| Validação de dados | `core-validation` | Contratos de validação canónica de domínio |

**Status:** Implementado (base); taxonomia de `control_id` a formalizar

### P7 — Identificação e análise de risco

**Relevância para o kernel:** O campo `outcome` em cada `AuditEvent` regista o resultado de cada controlo — `Passed`, `Failed`, ou `Skipped`. Desvios são evidência de risco materializado.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Resultado de controlo | `core-audit` | `ControlOutcome { Passed, Failed, Skipped }` |
| Contexto de falha | `core-audit` | `ControlExecution.context` — dados estruturados do desvio |
| Métricas operacionais | `core-metrics` | Agregações de runtime para detecção de padrões |

**Status:** Implementado

### P8 — Avaliação de risco de fraude

**Relevância para o kernel:** A arquitectura append-only e o dead-letter são as principais salvaguardas contra fraude técnica (adulteração de registos, supressão de evidência).

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Immutabilidade do log | `core-audit` | Sem operações de UPDATE/DELETE em `AuditStore` |
| Dead-letter | `core-audit` | Eventos que falham entrega são persistidos, não descartados |
| Hash encadeado | `core-audit` | Detecção de adulteração retroactiva do log |
| Cifra em repouso | `infra` | Impede acesso directo não autorizado à base de dados |

**Status:** Implementado

### P9 — Identificação e análise de mudanças significativas

**Relevância para o kernel:** Mudanças de configuração e mudanças organizacionais são auditadas como eventos de controlo.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Auditoria de configuração | `core-config` | Mudanças de perfil geram `AuditEvent` |
| Auditoria organizacional | `core-org` | Mudanças de estrutura geram eventos de domínio auditados |
| Eventos de domínio | `core-org` | Publicados via outbox transaccional |

**Status:** Implementado

---

## Componente 3 — Actividades de Controlo

> *A organização selecciona e desenvolve actividades de controlo que mitigam riscos, incluindo controlos sobre tecnologia.*

### P10 — Selecção e desenvolvimento de actividades de controlo

**Relevância para o kernel:** Cada operação significativa é envolvida num controlo explícito com `control_id`. O kernel não executa operações de negócio sem contexto de controlo.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Controlo por operação | Todos | `control_id` obrigatório em operações auditáveis |
| Execução de controlo | `core-audit` | `ControlExecution` — contexto completo de cada controlo |
| Validação de domínio | `core-validation` | Pré-condições verificadas antes de persistência |

**Status:** Implementado

### P11 — Controlos gerais sobre tecnologia

**Relevância para o kernel:** O kernel implementa múltiplas camadas de controlo sobre a tecnologia subjacente.

| Mecanismo | Crate / Componente | Implementação |
|-----------|-------------------|---------------|
| Cifra em repouso | `infra` (SQLite) | SQLite com cifra — dados ilegíveis sem chave |
| Gestão de secrets | `normordis_kernel::security` | Abstracção de secrets sem exposição em logs |
| Validação de configuração | `core-config` | Invariantes verificados em startup; rejeição de config inválida |
| Supply chain | CI pipeline | `cargo audit`, `cargo deny`, SBOM, provenance |
| Qualidade de código | CI pipeline | `clippy -D warnings`, `rustfmt`, testes obrigatórios |

**Status:** Implementado (parcial — segurança a aprofundar)

### P12 — Implementação de controlos via políticas e procedimentos

**Relevância para o kernel:** As políticas de controlo estão codificadas como contratos Rust — não são documentos externos, são invariantes do compilador.

| Mecanismo | Componente | Implementação |
|-----------|-----------|---------------|
| Contratos de domínio | `core/` | Tipos Rust — invariantes verificados em tempo de compilação |
| ADRs | `docs/adr/` | Decisões arquitecturais formalizadas e versionadas |
| Trust Baseline | `ADR-NK-006` | Política de confiança verificável contínua |
| Processo de release | CI pipeline | `fmt + clippy + test + release` obrigatórios |

**Status:** Implementado

---

## Componente 4 — Informação e Comunicação

> *A organização obtém e gera informação relevante e de qualidade, e comunica internamente e com partes externas.*

### P13 — Informação relevante e de qualidade

**Relevância para o kernel:** `domain-telemetry` e `core-metrics` fornecem observabilidade operacional; `core-audit` fornece o registo de controlo de alta fidelidade.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Eventos de uso | `domain-telemetry` | Registo de eventos de utilização com timestamps |
| Estatísticas agregadas | `domain-telemetry` | Agregações sobre janelas temporais |
| Métricas de runtime | `core-metrics` | Contadores e medições operacionais normalizados |
| Audit log estruturado | `core-audit` | `AuditEvent` em formato estruturado (JSON) |

**Status:** Implementado

### P14 — Comunicação interna

**Relevância para o kernel:** O outbox transaccional é o mecanismo de comunicação interna entre componentes — garante que eventos de domínio são entregues mesmo em caso de falha parcial.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Outbox transaccional | `core-audit` | Eventos publicados atomicamente com operações de domínio |
| Eventos de domínio | `core-org` | Publicação de mudanças organizacionais para consumidores |
| Drainer | `core-audit` | Processamento assíncrono de eventos pendentes no outbox |

**Status:** Implementado

### P15 — Comunicação com partes externas

**Relevância para o kernel:** A facade `normordis-kernel` é a interface de comunicação com as aplicações consumidoras. O módulo de exports suporta comunicação de dados para o exterior.

| Mecanismo | Componente | Implementação |
|-----------|-----------|---------------|
| API pública unificada | `normordis-kernel` (facade) | Namespace explícito por domínio |
| Exportação de dados | `normordis_kernel::exports` | Formatos abertos para interoperabilidade |
| Documentação de interface | `docs/` | ADRs, architecture overview, crate-map |

**Status:** Implementado (parcial — reporte externo formal a implementar)

---

## Componente 5 — Monitorização

> *A organização realiza avaliações contínuas ou separadas para verificar se os componentes de controlo interno estão presentes e a funcionar.*

### P16 — Avaliações contínuas e separadas

**Relevância para o kernel:** A monitorização contínua está integrada na arquitectura — não é um processo separado.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Telemetria contínua | `domain-telemetry` | Eventos de uso em tempo real |
| Métricas de runtime | `core-metrics` | Indicadores operacionais contínuos |
| Dead-letter queue | `core-audit` | Sinalização automática de falhas de entrega |
| Suíte de testes | CI | Avaliação automática em cada commit (>200 testes) |
| Cargo audit | CI | Avaliação contínua de vulnerabilidades de dependências |

**Status:** Implementado

### P17 — Avaliação e comunicação de deficiências

**Relevância para o kernel:** Deficiências de controlo são registadas como eventos `Failed` ou retidas em dead-letter — nunca silenciadas.

| Mecanismo | Crate | Implementação |
|-----------|-------|---------------|
| Outcomes de falha | `core-audit` | `ControlOutcome::Failed` — deficiência registada como evento |
| Dead-letter | `core-audit` | Eventos não entregues retidos para análise |
| `SECURITY.md` | Raiz | Processo formal de reporte de vulnerabilidades |
| CHANGELOG | Raiz | Registo de deficiências corrigidas por versão |

**Status:** Implementado (base); reporte formal de deficiências a formalizar

---

## Matriz Consolidada de Conformidade COSO

| Componente | Princípio | Crates Principais | Status |
|------------|-----------|------------------|--------|
| C1 — Ambiente | P1 — Integridade | `core-audit`, `infra` | Implementado |
| C1 — Ambiente | P2 — Supervisão | — | Parcial |
| C1 — Ambiente | P3 — Estrutura e autoridade | `core-org` | Implementado |
| C1 — Ambiente | P4 — Competência | `core-rh` | Implementado (base) |
| C1 — Ambiente | P5 — Responsabilização | `core-audit`, `core-rh` | Implementado |
| C2 — Risco | P6 — Objectivos adequados | `core-audit`, `core-validation` | Implementado (base) |
| C2 — Risco | P7 — Identificação de risco | `core-audit`, `core-metrics` | Implementado |
| C2 — Risco | P8 — Risco de fraude | `core-audit`, `infra` | Implementado |
| C2 — Risco | P9 — Mudanças significativas | `core-config`, `core-org` | Implementado |
| C3 — Actividades | P10 — Actividades de controlo | Todos os crates auditáveis | Implementado |
| C3 — Actividades | P11 — Controlos tecnológicos | `infra`, CI pipeline | Implementado (parcial) |
| C3 — Actividades | P12 — Políticas e procedimentos | ADRs, contratos Rust | Implementado |
| C4 — Informação | P13 — Informação de qualidade | `domain-telemetry`, `core-metrics` | Implementado |
| C4 — Informação | P14 — Comunicação interna | `core-audit` (outbox) | Implementado |
| C4 — Informação | P15 — Comunicação externa | `normordis-kernel` (facade) | Implementado (parcial) |
| C5 — Monitorização | P16 — Avaliações contínuas | `domain-telemetry`, CI | Implementado |
| C5 — Monitorização | P17 — Comunicação de deficiências | `core-audit` (dead-letter) | Implementado (base) |

## Limitações e Exclusões Conhecidas

- **P2 (Supervisão)**: o kernel não pode implementar a supervisão independente do conselho — é uma responsabilidade organizacional. O kernel fornece os dados necessários para essa supervisão.
- **P6 (Objectivos)**: a taxonomia formal de `control_id` (mapeamento explícito para objectivos COSO) está identificada como trabalho futuro.
- **P4 (Competência)**: `RoleService` com evidência COSO e `UserRoleAssignment` temporal estão planeados mas não implementados.
- **P15 (Comunicação externa)**: reporte formal estruturado para entidades externas (ex: CNCS, Tribunal de Contas) não está implementado.
- **INTOSAI GOV 9100**: complemento COSO para o sector público — mapeamento dedicado a documentar separadamente.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AuditEvent** | Evento estruturado gerado atomicamente com cada operação de domínio auditável |
| **AuditStore** | Contrato de persistência append-only de `AuditEvent` — sem UPDATE ou DELETE |
| **control_id** | Identificador que liga um `AuditEvent` a um controlo e princípio COSO específico |
| **ControlExecution** | Contexto de execução de um controlo: actor, timestamp, dados estruturados |
| **ControlOutcome** | Resultado de um controlo: `Passed`, `Failed`, ou `Skipped` |
| **COSO** | Committee of Sponsoring Organizations — framework de controlo interno (edição 2013) |
| **Dead-letter** | Fila de persistência para eventos de outbox que falharam entrega — nunca descartados |
| **INTOSAI** | International Organization of Supreme Audit Institutions — normas de auditoria pública |
| **OCC** | Optimistic Concurrency Control — controlo de concorrência via versão de agregado |
| **Outbox transaccional** | Padrão que garante publicação atómica de eventos com o commit da transacção de domínio |
| **RAT** | Registo de Actividades de Tratamento — obrigação RGPD Art. 30.º |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — mapeamento completo dos 17 princípios COSO |

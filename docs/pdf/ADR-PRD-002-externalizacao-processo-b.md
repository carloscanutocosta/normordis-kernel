# ADR-PRD-002 — Externalização do Pipeline de Geração de PDF para Processo B

Estado: Aprovado  
Âmbito: Evolução do ADR-PRD-001 — arquitectura edge por unidade orgânica (UO)  
Autor: Carlos Costa  
Data: 2026-04-12  
Origem: ADR-PRD-002 (mini-apps-rusty)  
Depende de: [ADR-PRD-001](ADR-PRD-001-pipeline-assincrono.md)

---

## Contexto

O ADR-PRD-001 define um pipeline assíncrono de geração de PDF embutido na aplicação, com fila local, worker único e cache por hash. Este modelo resolve latência e cold start, mas apresenta limitações estruturais em cenários multi-utilizador:

- **Multiplicação de workers** — cada utilizador executa o seu próprio worker, cache e spool, com duplicação de trabalho.
- **Concorrência não controlada** — vários utilizadores podem gerar PDFs simultaneamente, sobrecarregando recursos partilhados.
- **Falta de centralização** — sem controlo global, observabilidade agregada ou política uniforme de execução.
- **Limitações de auditabilidade** — a geração ocorre fora de um contexto institucional controlado.

---

## Decisão

Externalizar o pipeline de geração de PDF para um **Processo B local por unidade orgânica (UO)**.

### Definição de Processo B

O Processo B é um serviço local, residente na infra da UO (ex.: mini-PC), responsável por:
- receber pedidos de geração
- gerir fila central
- executar compilação Typst
- gerir cache
- produzir artefactos PDF

### Arquitectura lógica

```text
[ Apps consumidoras (clientes) ]
            │
            ▼
   (HTTP / IPC local)
            │
            ▼
     Processo B (UO)
   ├─ Queue (FIFO)
   ├─ Worker(s)
   ├─ Cache (hash → PDF)
   ├─ Metrics
            │
            ▼
     Storage local (NAS / SSD)
```

### API de entrada

Interface HTTP (localhost) ou IPC (socket):
- `POST /jobs`
- `GET /jobs/{id}`
- `GET /health`

### Cache partilhada

Baseada em hash, comum a todos os utilizadores da UO. Elimina recomputação duplicada.

---

## Consequências

### Positivas

- Redução adicional de latência média (cache partilhada elimina recomputações duplicadas)
- Maior previsibilidade e controlo de concorrência
- Simplificação das apps consumidoras (o worker sai do processo da app)
- Base para integração com `core-audit` e `core-documental`

### Negativas

- Necessidade de serviço local na infra da UO
- Dependência de disponibilidade do Processo B
- Complexidade adicional de deployment

---

## Evolução futura

1. Integração com `core-documental` — armazenamento imutável e hash institucional
2. Integração com `core-audit` — registo do acto de geração
3. Execução distribuída — múltiplos nós por UO com balanceamento
4. Continuidade offline — fila persistente, store-and-forward

---

## Referências

- [ADR-PRD-001](ADR-PRD-001-pipeline-assincrono.md)
- [ADR-PRD-003](ADR-PRD-003-integracao-crate-typst.md)

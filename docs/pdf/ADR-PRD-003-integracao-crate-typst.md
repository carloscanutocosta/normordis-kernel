# ADR-PRD-003 — Integração Direta do Crate `typst` no Worker do Processo B

Estado: Aprovado  
Âmbito: Processo B (ADR-PRD-002) — worker de geração de PDF, contexto de 8–12 utilizadores simultâneos  
Autor: Carlos Costa  
Data: 2026-04-12  
Origem: ADR-PRD-003 (mini-apps-rusty)  
Depende de: [ADR-PRD-002](ADR-PRD-002-externalizacao-processo-b.md)

---

## Contexto

No modelo herdado do ADR-PRD-001, o worker executa a compilação através do CLI do Typst:

```bash
typst compile input.typ output.pdf --root templates
```

Por cada job sem cache hit, este modelo implica spawn de novo processo, carregamento de fontes do zero e parsing do template do zero. Em contexto de **8–12 utilizadores simultâneos**, o efeito é agravado: múltiplos processos Typst concorrentes, pressão de CPU e IO multiplicada, sem partilha de estado entre compilações.

---

## Decisão

Substituir a invocação do CLI `typst` por **integração directa do crate `typst`** no binário do Processo B.

### Arquitectura

```text
[ Processo B ]
   ├─ API (Axum / IPC)
   ├─ Fila central (FIFO)
   ├─ Worker Pool (N workers)
   │    ├─ Worker 1 ──┐
   │    ├─ Worker 2 ──┤── Arc<SharedCompilerState>
   │    └─ Worker N ──┘
   ├─ Cache (hash → PDF)
   └─ Métricas
```

### Estado partilhado entre workers

```text
SharedCompilerState
  ├─ FontBook         (fontes carregadas uma vez no arranque)
  ├─ TemplateCache    (templates parsed e em memória)
  └─ AssetsIndex      (índice de assets disponíveis)
```

Cada job cria um `JobWorld` leve que referencia este estado via `Arc`, sem recarregar fontes ou templates.

### Fases internas do job

1. **Preparing** — construção do `JobWorld` em memória; sem escrita em disco
2. **Compiling** — `typst::compile(&world)` no thread do worker
3. **Exporting** — `typst_pdf::pdf(&document, &options)` em memória
4. **Storing** — gravação em `cache/<hash>.pdf`; estado → `Done`

### Worker pool

- Default: 2 workers
- Configurável: até N (recomendado ≤ núcleos físicos disponíveis)
- Compilação Typst é CPU-bound — mais workers que núcleos não aumenta throughput

---

## Comparação com modelo CLI

| Dimensão | CLI subprocess | Crate directo |
|---|---|---|
| Spawn de processo | Por job | Nenhum |
| Carregamento de fontes | Por job | Uma vez (arranque) |
| Parsing de template | Por job | Partilhado (cache) |
| Latência estimada (cold) | 10–15 s | 1–3 s |
| Latência estimada (warm) | 10–15 s | < 1 s |
| Throughput (8–12 users) | Degradação linear | Controlado por worker pool |

---

## Consequências

### Positivas

- Redução drástica de latência (eliminação de spawn + carregamento de fontes)
- Throughput controlado e previsível
- Menor pressão de IO (sem escrita de ficheiros intermédios)
- Base para compilação incremental futura

### Negativas

- O Processo B passa a depender da versão do crate `typst` (pin obrigatório)
- `typst::World` requer implementação própria (não trivial)
- Arranque mais lento (carregamento de fontes)

---

## Referências

- [ADR-PRD-002](ADR-PRD-002-externalizacao-processo-b.md)
- [ME-PRD-003 — Modelo de execução](ME-PRD-003-modelo-execucao-processo-b.md)
- `crates/kernel/infra/pdf/render-typst/`

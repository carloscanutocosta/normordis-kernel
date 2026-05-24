# ME-PRD-003 — Modelo de Execução: Integração Direta do Crate `typst` no Processo B

Estado: Activo  
Âmbito: Processo B — `crates/kernel/infra/pdf/render-typst`  
Conforme: [ADR-PRD-003](ADR-PRD-003-integracao-crate-typst.md)

---

## Visão geral

```text
[ Processo B ]
   │
   ├─ Arranque
   │    ├─ Carregamento de fontes → FontBook
   │    ├─ Pré-carregamento de templates → TemplateCache
   │    └─ Inicialização do worker pool
   │
   ├─ Operação (por job)
   │    ├─ Verificação de cache
   │    ├─ Construção de JobWorld
   │    ├─ typst::compile()
   │    ├─ typst_pdf::pdf()
   │    └─ Gravação em cache
   │
   └─ API (HTTP / IPC)
        ├─ POST /jobs
        ├─ GET /jobs/{id}
        └─ GET /health
```

---

## Fase 1 — Arranque do Processo B

### 1.1 Carregamento de fontes

- Varrer directórios de fontes configurados
- Construir `FontBook` com todas as fontes disponíveis
- Manter em `Arc<FontBook>` — sem releitura de disco por job

### 1.2 Pré-carregamento de templates

- Carregar ficheiros `.typ` dos templates registados
- Indexar por `(template_id, template_version)`
- Manter em `Arc<TemplateCache>`

### 1.3 Inicialização do worker pool

```text
workers: 2          (default)
max_workers: N      (≤ núcleos físicos)
queue: FIFO
```

Cada worker recebe `Arc<SharedCompilerState>` e receptor da fila de jobs.

### 1.4 API disponível

Processo B anuncia estado `ready` e começa a aceitar pedidos.

---

## Fase 2 — Recepção de pedido

```json
{
  "template_id": "certidao",
  "template_version": "1.0.0",
  "payload": {},
  "assets_version": "1.0.0"
}
```

Processamento:
1. Canonicalização do payload (JSON determinístico)
2. Cálculo do hash SHA-256
3. Verificação de cache

**Cache hit:** devolução imediata; `queue_wait_ms: 0`, `typst_compile_ms: 0`

**Cache miss:** criação de `PdfJob`; inserção na fila FIFO; devolução de `job_id`

---

## Fase 3 — Processamento pelo worker

### 3.1 Preparing — construção do JobWorld

```text
JobWorld
  ├─ Arc<FontBook>         ← partilhado, sem cópia
  ├─ Arc<TemplateCache>    ← partilhado, sem cópia
  ├─ payload: Value        ← específico do job
  └─ template_id/version   ← específico do job
```

Sem escrita em disco nesta fase. Métrica: `prepare_world_ms`

### 3.2 Compiling

```rust
let document = typst::compile(&world);
```

Execução em memória, no thread do worker, com fontes e templates já carregados. Métrica: `typst_compile_ms`

### 3.3 Exporting

```rust
let pdf_bytes = typst_pdf::pdf(&document, &PdfOptions::default());
```

Resultado: `Vec<u8>` em memória. Métrica: `typst_export_ms`

### 3.4 Storing

- Gravação: `cache/<hash>.pdf`
- Estado → `Done`

Métrica: `store_output_ms`

---

## Estados do job

```
Queued → Preparing → Compiling → Exporting → Storing → Done
                                                      → Failed
```

---

## Modelo de dados

```text
SharedCompilerState
  ├─ font_book: Arc<FontBook>
  ├─ template_cache: Arc<TemplateCache>
  └─ typst_version: String

JobWorld  (impl typst::World)
  ├─ state: Arc<SharedCompilerState>
  ├─ template_source: Arc<str>
  ├─ payload: serde_json::Value
  └─ now: DateTime<Utc>

JobMetrics
  ├─ queue_wait_ms: u64
  ├─ prepare_world_ms: u64
  ├─ typst_compile_ms: u64
  ├─ typst_export_ms: u64
  ├─ store_output_ms: u64
  └─ total_ms: u64
```

---

## Regras operacionais

1. `SharedCompilerState` é imutável após arranque — alterações de templates/fontes requerem reinicialização.
2. `JobWorld` é local ao worker — não partilhado entre workers.
3. Worker pool fixo — sem scaling dinâmico em runtime.
4. Fila FIFO com backpressure — rejeita novos pedidos com `503` se a fila ultrapassar o limite.
5. Idempotência garantida por hash.

---

## Falhas

| Tipo | Fase | Tratamento |
|------|------|-----------|
| Template não encontrado | Preparing | `Failed` com mensagem |
| Erro de compilação Typst | Compiling | `Failed` com diagnóstico Typst |
| Erro de exportação | Exporting | `Failed` com mensagem |
| Erro de IO | Storing | `Failed`, PDF não persistido |
| Worker panic | Qualquer | Job marcado `Failed`, worker reiniciado |

---

## Comportamento sob carga (8–12 utilizadores)

Com 2 workers: 2 jobs em compilação simultânea, restantes em `Queued` (FIFO). O indicador de pressão é `queue_wait_ms` — se consistentemente > 5s, considerar aumentar para 3–4 workers.

Com cache quente: maioria dos pedidos resolvida antes de entrar na fila; workers raramente saturados.

---

## Extensões previstas

1. Compilação incremental via APIs `typst` para reutilização entre compilações do mesmo template
2. Recarga de templates sem reinício (`POST /admin/reload-templates`)
3. Integração com `core-documental` — `pdf_bytes` directo ao adapter de persistência imutável
4. Fila persistente em SQLite para continuidade offline

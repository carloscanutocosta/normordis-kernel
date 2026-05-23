# ME-PRD-001 — Modelo de Execução do Pipeline de Geração de PDF

Estado: Activo  
Âmbito: `crates/kernel/infra/pdf/pdf-pipeline`  
Conforme: [ADR-PRD-001](ADR-PRD-001-pipeline-assincrono.md)

---

## Visão geral

O sistema implementa um pipeline determinístico, assíncrono e observável, composto por:

- **Chamador** (app consumidora — Tauri, HTTP, CLI)
- **Backend (Rust)** — `pdf-pipeline`
- **Pipeline local** (fila + worker)
- **Engine Typst**
- **Cache de artefactos**

---

## Fluxo completo

### 1. Inicialização

Trigger: arranque da app ou primeiro pedido

Acção: `warm_pipeline()`

Efeitos:
- criação de directórios: `runtime/spool`, `runtime/cache`
- validação do engine Typst
- marcação do pipeline como "warm"

### 2. Preparação do pedido

Construção de `PdfJobRequest`:
- `template_id`
- `template_version`
- `payload`
- `assets_version`

### 3. Submissão do job

Acção: `enqueue_pdf_job(request)`

Processamento:
1. Canonicalização do payload
2. Cálculo de hash
3. Verificação de cache

**Caso A — Cache hit:** devolução imediata; estado `Done { from_cache: true }`

**Caso B — Cache miss:** criação de job; inserção em fila; estado inicial `Queued`

### 4. Processamento pelo worker

Loop contínuo: receber job → processar.

---

## Fases internas do job

### 4.1 Preparing

- Criação de directório: `runtime/spool/<job_id>/`
- Preparação de `payload.json` e `document.typ`

### 4.2 Compiling

```bash
typst compile document.typ output.pdf --root templates
```

### 4.3 Storing

- Gravação em: `runtime/cache/<hash>.pdf`
- Actualização do estado: `Done`

---

## Estados do job

```
Queued → Preparing → Compiling → Storing → Done
                                         → Failed
```

---

## Modelo de dados

```text
PdfJob
  ├─ job_id
  ├─ hash
  ├─ request
  └─ created_at

PdfJobState
  ├─ job
  ├─ status
  └─ metrics
```

---

## Cache

**Chave:**

```text
hash = SHA256(
  template_id +
  template_version +
  payload_canonical +
  assets_version +
  typst_version
)
```

**Garantias:** determinismo, idempotência, reprodutibilidade.

---

## Directórios

```text
runtime/
  spool/
    <job_id>/
  cache/
    <hash>.pdf
```

---

## Métricas por job

- `prepare_payload_ms`
- `write_inputs_ms`
- `typst_compile_ms`
- `store_output_ms`
- `total_ms`

---

## Regras operacionais

1. **Exclusividade de execução** — 1 worker activo
2. **Ordem** — FIFO
3. **Idempotência** — garantida por hash
4. **Não bloqueio** — o chamador nunca espera pela compilação

---

## Falhas

| Tipo | Tratamento |
|------|-----------|
| Erro de compilação Typst | Estado `Failed` com mensagem |
| Erro de IO | Estado `Failed` com mensagem |
| Payload inválido | Estado `Failed` antes de entrar na fila |

---

## Extensões previstas

- Multi-worker controlado (Processo B — ADR-PRD-002)
- Integração com `core-documental` e `core-audit`
- Persistência institucional

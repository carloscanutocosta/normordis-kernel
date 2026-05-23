# ADR-PRD-001 — Pipeline Assíncrono de Geração de PDF com Typst

Estado: Aprovado  
Âmbito: Pipeline PDF do normordis-kernel — `crates/kernel/infra/pdf/pdf-pipeline`  
Autor: Carlos Costa  
Data: 2026-04-12  
Origem: ADR-PRD-001 (mini-apps-rusty)

---

## Contexto

A geração de documentos PDF constitui uma operação crítica em aplicações institucionais. No modelo anterior, a geração através de Typst era realizada de forma síncrona, por invocação directa do comando `typst compile` por cada pedido. Este modelo apresenta os seguintes problemas:

- **Latência elevada (10–15 segundos)** por geração
- **Cold start recorrente** (inicialização completa por cada execução)
- **Bloqueio do chamador** em aplicações síncronas
- **Ausência de reutilização de trabalho incremental**
- **Contenção de recursos** em cenários multi-utilizador

---

## Decisão

Adoptar um **pipeline assíncrono de geração de PDF**, baseado nos seguintes princípios:

### 1. Execução assíncrona

Todos os pedidos de geração são processados de forma assíncrona. O chamador recebe um `job_id` imediatamente e consulta o estado via polling.

### 2. Warm-up do pipeline

Ao inicializar, o pipeline executa `warm_pipeline()`:
- cria directórios de runtime (spool/cache)
- valida o engine Typst
- marca o pipeline como "warm"

### 3. Fila local de jobs

Cada pedido cria um `PdfJob`. Os jobs são enfileirados (FIFO). Separação entre pedido e execução.

### 4. Worker único residente

Um único worker processa jobs sequencialmente, evitando contenção de recursos e maximizando previsibilidade.

### 5. Cache por hash determinístico

Cada job é identificado por hash SHA-256 baseado em:
- template
- versão do template
- payload (canonizado)
- versão dos assets
- versão do Typst

Se existir PDF previamente gerado, devolução imediata sem recompilação.

### 6. Separação de fases

Cada job segue explicitamente: `Preparing → Compiling → Storing`

### 7. Instrumentação obrigatória

Métricas por job: `prepare_payload_ms`, `write_inputs_ms`, `typst_compile_ms`, `store_output_ms`, `total_ms`

---

## Modelo conceptual

```
Command → Queue → Worker → Artifact
```

---

## Consequências

### Positivas

- Redução significativa da latência (10–15s → 1–3s)
- Chamador não bloqueado
- Reutilização de resultados (cache)
- Base para escalabilidade futura
- Observabilidade detalhada

### Negativas

- Maior complexidade de implementação
- Necessidade de gestão de estado local
- Introdução de concorrência controlada

---

## Evolução futura

1. Externalização para Processo B (ADR-PRD-002)
2. Integração com `core-documental` — persistência imutável e custódia
3. Distribuição por UO (edge computing)
4. Política de retenção e auditoria com `core-audit`

---

## Referências

- [ME-PRD-001 — Modelo de execução](ME-PRD-001-modelo-execucao-pipeline.md)
- [ADR-PRD-002](ADR-PRD-002-externalizacao-processo-b.md)
- `crates/kernel/infra/pdf/pdf-pipeline/`

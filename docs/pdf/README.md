# Pipeline PDF — normordis-kernel

Estado: Activo  
Âmbito: `crates/kernel/infra/pdf/`  
Actualizado: 2026-05-23

---

## Decisões de arquitectura

| Doc | Título | Estado | Origem |
|-----|--------|--------|--------|
| [ADR-PRD-001](ADR-PRD-001-pipeline-assincrono.md) | Pipeline assíncrono de geração de PDF com Typst | Aprovado | ADR-PRD-001 (mini-apps-rusty) |
| [ADR-PRD-002](ADR-PRD-002-externalizacao-processo-b.md) | Externalização do pipeline para Processo B (por UO) | Aprovado | ADR-PRD-002 (mini-apps-rusty) |
| [ADR-PRD-003](ADR-PRD-003-integracao-crate-typst.md) | Integração direta do crate `typst` no worker do Processo B | Aprovado | ADR-PRD-003 (mini-apps-rusty) |

## Modelos de execução

| Doc | Título |
|-----|--------|
| [ME-PRD-001](ME-PRD-001-modelo-execucao-pipeline.md) | Modelo de execução do pipeline (ADR-PRD-001) |
| [ME-PRD-003](ME-PRD-003-modelo-execucao-processo-b.md) | Modelo de execução do worker com crate `typst` (ADR-PRD-003) |

## Formatos documentais

| Doc | Título | Versão |
|-----|--------|--------|
| [NCRTF.md](NCRTF.md) | NORMAXIS Canonical Rich Text Format | 1.3.0 |
| [NDF.md](NDF.md) | NORMAXIS Document Format | 1.1.0 |
| [NDT.md](NDT.md) | NORMAXIS Document Template | 2.0.0 |

---

## Crates do pipeline

```
crates/kernel/infra/pdf/
  render-typst/        — renderização Typst → PDF (crate typst integrado)
  normordis-pdf/       — geração PDF institucional (NDT/NDF/NCRTF)
  documentos-pdf/      — pipeline PDF de tipos documentais específicos
  pdf-pipeline/        — orquestrador assíncrono (fila + worker + cache)
```

---

## Objectivo

O pipeline implementa geração de PDFs de forma:

- **assíncrona** — não bloqueia o chamador
- **eficiente** — cache por hash SHA-256 elimina recompilação de documentos idênticos
- **determinística** — o mesmo payload produz sempre o mesmo PDF
- **não bloqueante** — o worker processa jobs sequencialmente, a UI (quando existir) nunca espera

O padrão operacional é:

```
Command → Queue → Worker → Artifact
```

---

## Formatos: NCRTF, NDT, NDF

```
NDT (template)  +  NdtData (dados)
        ↓  compile_ndt()
       NDF  ──────────────────────────► arquivo documental
        ↓  render_ndf()
       PDF  ─────────────────────────── entrega / publicação
```

O NCRTF é o formato de rich text embebível em campos de um NDT — conteúdo editável pelo utilizador, serializado em JSON, consumido pelo backend Rust.

---

## Contexto de uso

Os ADRs neste directório descrevem o design do pipeline tal como foi concebido e implementado originalmente em `mini-apps-rusty`. As partes relativas a comandos Tauri e frontend são contexto das apps consumidoras — o kernel expõe `pdf-pipeline` como crate de infra agnóstico de runtime.

Apps consumidoras (Tauri, HTTP) integram o pipeline via `miniapp-runtime` ou directamente via `pdf-pipeline`.

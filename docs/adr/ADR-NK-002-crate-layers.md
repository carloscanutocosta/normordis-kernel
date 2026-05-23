# ADR-NK-002 — Estrutura de crates `core/`, `support/` e `infra/`

Estado: Aceite  
Âmbito: normordis-kernel · Organização de crates  
Autor: Carlos Costa  
Data: 2026-05-11  
Versão: v1.0.0  
Origem: ADR-MINIAPPS-003-crates-core-support-infra (mini-apps-rusty)

---

## Contexto

O workspace do `normordis-kernel` contém dezenas de crates Rust. Sem uma
estrutura lógica explícita, a diferença entre:

- semântica de domínio reutilizável;
- contratos e primitivos técnicos transversais;
- adapters concretos de infraestrutura;

torna-se progressivamente opaca, aumentando o risco de dependências invertidas
e dificultando a auditoria arquitectural.

---

## Decisão

Adoptar a seguinte estrutura lógica de crates, implementada no workspace:

```
crates/
  kernel/
    core/     — semântica de domínio e portos (sem I/O)
    support/  — primitivos e contratos técnicos transversais headless
    infra/    — adapters concretos e materialização técnica
  domain/     — domínios transversais (numerador, MEF)
  runtime/    — contexto e ciclo de vida de mini-apps
normordis-kernel/  — fachada pública unificada
```

### Regra de dependências

```
apps → normordis-kernel (fachada) → core → support → infra
```

Restrições:

- `core/` não depende de adapters concretos (`infra/`);
- `core/` declara ports/traits específicos do seu domínio;
- `support/` fornece primitivos transversais e contratos partilhados;
- `infra/` implementa adapters concretos;
- o wiring final ocorre no runtime/bootstrap da app.

Formula:

```
core    define ports e semântica
support fornece contratos e primitivos
infra   implementa adapters
runtime faz wiring
```

### Estrutura implementada

```
crates/kernel/
  core/
    core-audit, core-config, core-validation, core-rh, core-org,
    core-security, core-documental, core-exports, core-ingest, core-metrics

  support/
    support-auth, support-errors, support-backup, support-crypto,
    support-storage, support-logging, support-normalization,
    support-typst-template, support-pdf, support-clock, support-ids,
    support-address, support-versioning, support-docx-to-typst

  infra/
    adapter-sqlite, secrets, runtime-bootstrap, rh-sqlite, org-sqlite,
    security-sqlite, rh-security-bridge, documental-sqlite, files,
    metrics-sqlite, exports-sqlite, adapter-scanner, app-bootstrap,
    address-sqlite, versioning-sqlite, storage-sqlite, numerador-sqlite,
    mef-sqlite, services/signing, services/export, services/backup,
    services/ingest-scanner, pdf/render-typst, pdf/normordis-pdf,
    pdf/documentos-pdf, pdf/pdf-pipeline
```

---

## Consequências

### Positivas

- maior fidelidade ao modelo NORMORDIS;
- leitura arquitectural clara;
- separação explícita entre contrato e adapter;
- menor risco de dependências invertidas;
- testes de conformidade por camada;
- possibilidade de adicionar outros adapters sem alterar semântica.

### Negativas

- disciplina contínua na criação de novos crates;
- necessidade de documentar cada novo crate na camada correcta.

---

## Política para novos crates

1. Nascer sempre na camada correcta (`core/`, `support/` ou `infra/`);
2. Não mover crates existentes sem necessidade concreta;
3. Cada crate deve ser testável isoladamente;
4. Cada nova dependência directa segue `security/DEPENDENCY_POLICY.md`.

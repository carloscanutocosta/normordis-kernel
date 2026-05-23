# ADR-NK-001 — Formalização do normordis-kernel como camada autónoma

Estado: Aceite  
Âmbito: Arquitectura · Plataforma Rust  
Autor: Carlos Costa  
Data: 2026-05-02  
Versão: v1.0.0  
Origem: ADR-MINIAPPS-002 (mini-apps-rusty)

---

## Contexto

O repositório `mini-apps-rusty` evoluiu para uma monorepo Rust com múltiplos
crates de suporte reutilizáveis. A ausência de um enquadramento explícito para
estas capacidades levaria a:

- crescimento ad hoc de crates;
- duplicação de lógica entre aplicações;
- acesso directo a SQLite e filesystem sem contrato;
- incoerência entre mini-apps;
- perda de alinhamento com princípios NORMORDIS.

---

## Decisão

Extrair o conjunto de crates reutilizáveis do `mini-apps-rusty` para um
repositório autónomo — o `normordis-kernel`.

O `normordis-kernel` é definido como:

- conjunto de crates Rust reutilizáveis e headless;
- contratos Rust estáveis expostos via crate de fachada `normordis-kernel`;
- adapters locais (SQLite, filesystem, PDF, assinatura);
- camada transversal consumida por aplicações do ecossistema NORMORDIS.

A fachada pública (`crates/normordis-kernel`) expõe módulos namespaced que
permitem às apps consumir o kernel como uma SDK de plataforma:

```rust
use normordis_kernel::documental::DocumentCustody;
use normordis_kernel::audit::AuditService;
use normordis_kernel::rh::UserContext;
```

---

## Consequências

### Positivas

- reutilização sistemática de capacidades por múltiplas apps;
- redução de duplicação de código;
- coerência arquitectural garantida por CI (clippy, testes, audit);
- agnóstico de plataforma (Windows/Linux) e runtime (desktop/HTTP);
- evolução independente do kernel sem afectar apps consumidoras.

### Negativas

- as apps consumidoras passam a ter dependência Git do kernel;
- necessidade de disciplina na criação de crates;
- maior carga de documentação e governança.

---

## Alternativas consideradas

### 1. Manter os crates no repositório mini-apps-rusty

Rejeitada como modelo permanente. Não permite reutilização por outros
repositórios NORMORDIS sem duplicação.

### 2. Publicar em crates.io

Considerada para o futuro. Na fase actual (v0.3.x), o kernel é privado e
evolui rapidamente — publicação em crates.io seria prematura.

---

## Estado futuro

O `normordis-kernel` torna-se a referência de plataforma para:

- organização dos crates em `crates/kernel/`, `crates/domain/`, `crates/runtime/`;
- definição de contratos Rust estáveis;
- consumo por apps via dependência Git;
- evolução arquitectural do ecossistema NORMORDIS.

# normordis-kernel — Visão Geral Arquitectural

Estado: Activo  
Âmbito: normordis-kernel · Arquitectura de plataforma  
Autor: Carlos Costa  
Data: 2026-05-02  
Versão: v1.0.0

---

## 1. Definição

O `normordis-kernel` é a camada de plataforma partilhada do ecossistema
NORMORDIS — base local, headless e reutilizável que permite às aplicações
partilharem capacidades transversais em Rust sem duplicação de lógica.

O kernel não é uma cópia do backend NORMORDIS central. É uma plataforma
própria, inspirada nos princípios NORMORDIS, ajustada ao contexto de
aplicações desktop e servidores HTTP locais.

Fórmula de síntese:

```
normordis-kernel = capacidades locais reutilizáveis + contratos Rust + adapters locais
```

---

## 2. Objectivos

- Execução local em desktop (Windows primário, Linux validado em CI);
- Agnóstico de runtime: suporta Tauri (desktop) e servidor HTTP;
- Persistência SQLite controlada com cifra em repouso;
- Validação local determinística;
- Registo auditável com cadeia de hashes verificável;
- Composição documental com pipeline PDF/Typst;
- Reutilização de crates entre múltiplas apps NORMORDIS.

---

## 3. Relação com NORMORDIS

```
NORMORDIS
  define princípios, linguagem arquitectural e critérios de coerência

normordis-kernel
  aplica esses princípios numa plataforma Rust reutilizável
  exposta como API via crate de fachada

Apps consumidoras (mini-apps-rusty, etc.)
  consomem o kernel como dependência Git
  não conhecem os crates internos — apenas a fachada
```

---

## 4. Camadas do workspace

```
crates/
  kernel/
    core/      — semântica de domínio e portos (sem I/O, testável isoladamente)
    support/   — primitivos e contratos técnicos transversais headless
    infra/     — adapters concretos (SQLite, PDF, assinatura, secrets)
  domain/      — domínios transversais (numerador, MEF)
  runtime/     — contexto e ciclo de vida de mini-apps

normordis-kernel/  — fachada pública unificada (SDK de plataforma)
```

Ver [crate-map.md](crate-map.md) para o inventário completo.

---

## 5. Regra de dependências

```
apps → normordis-kernel (fachada)
         │
         ├─► core/    (domínio, sem I/O)
         │     │
         │     └─► support/ (primitivos headless)
         │               │
         │               └─► infra/ (adapters — SQLite, PDF, etc.)
         │
         └─► domain/  (numerador, MEF)
         └─► runtime/ (contexto de mini-app)
```

**Invariante:** `core/` nunca depende de `infra/`. As dependências fluem sempre
de fora para dentro.

---

## 6. API pública (fachada)

A fachada em `crates/normordis-kernel/src/lib.rs` expõe módulos namespaced:

```rust
normordis_kernel::rh         // Recursos humanos e contexto de utilizador
normordis_kernel::org        // Estrutura orgânica
normordis_kernel::audit      // Auditoria append-only
normordis_kernel::documental // Ciclo de vida documental
normordis_kernel::validation // Validadores canónicos
normordis_kernel::security   // Políticas de acesso
normordis_kernel::config     // Configuração de perfis
normordis_kernel::metrics    // Métricas de runtime
normordis_kernel::exports    // Exportação de dados
normordis_kernel::ingest     // Entrada de documentos
normordis_kernel::numerador  // Numeração sequencial
normordis_kernel::mef        // Classificação MEF
normordis_kernel::runtime    // Contexto de mini-apps
normordis_kernel::errors     // Erros partilhados
normordis_kernel::ids        // Identificadores únicos
normordis_kernel::clock      // Abstracção de tempo
```

Os adapters de infraestrutura (`*-sqlite`, PDF, secrets) são detalhes de
implementação — não estão expostos na fachada.

---

## 7. Autoridade local

O `normordis-kernel` tem autoridade local e operacional. Pode criar documentos,
eventos, numerações e artefactos locais, mas esses artefactos não devem ser
confundidos com custódia institucional central quando esta existir.

Princípio:

```
Local executa e regista; central, quando existir, revalida e consolida.
```

---

## 8. Critério para novos crates

Um novo crate só deve ser criado quando:

- a capacidade for reutilizável por mais de uma app consumidora;
- tiver contrato claro e testável isoladamente;
- não pertencer exclusivamente a um produto;
- não duplicar funcionalidade existente.

Ver [ADR-NK-002](../adr/ADR-NK-002-crate-layers.md) para a política de camadas.

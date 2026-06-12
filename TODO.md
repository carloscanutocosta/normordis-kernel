# Roadmap — normordis-kernel

Estado em `devel`, versão `0.3.0` do workspace / spec `0.9.0`.

---

## Visão: AP document-centric

Os três crates `core-ingest → core-documental → core-exports` formam um bloco coerente:
**receber com rastreabilidade → guardar com autoridade → produzir com reprodutibilidade**.
O caminho crítico para materializar este bloco é `core-documental` — enquanto não tiver
contrato, `ingest-documental-adapter` não pode ser implementado e `ExportSnapshot` continua bloqueado.

---

## Horizonte 1 — Próximos passos

### core-ingest ✅ concluído (2026-06-12)

Redesign + contrato normordis-spec completo e enterprise-grade.

| Item | Estado |
|------|--------|
| Redesign `IngestBundle` (raw bytes, `IngestStoragePort`, `ContentValidator`) | ✅ |
| normordis-spec 0.9.0 — 7 schemas, 16 fixtures, R01–R16 | ✅ |
| `IngestBundle.raw` serializado como base64 (RFC 4648) | ✅ |
| Invariantes layer-3: R13 (`document_ref`), R14 (temporal), R15 (hash) | ✅ |
| Kinds canónicos documentados (`cius-pt-invoice`, `saft-pt`, `iap-pi-message`, …) | ✅ |
| Bug corrigido: `hash_verified=false` em mismatch | ✅ |
| 34 testes `core-ingest` + 12 testes `spec-conformance` verdes | ✅ |

Ver contexto legal: [docs/pt/compliance/interop-ap-tecnico.md](docs/pt/compliance/interop-ap-tecnico.md).

---

### core-documental — CAMINHO CRÍTICO ⬅ próximo

`core-documental` é o elo que desbloqueia tudo o resto do bloco document-centric.

**O que falta definir:**

- [ ] **Domínio**: o que é um `DocumentPackage`? Estrutura mínima:
  `document_id`, `blob_ref` (chave MinIO), `content_type`, `hash`, `source_bundle_id`,
  `received_at`, `meta`
- [ ] **Port de armazenamento**: `DocumentStore` trait (write) + `DocumentReader` trait (read)
- [ ] **normordis-spec**: schema `DocumentPackage` executável — sem ele `document_ref`
  em `IngestEvidence` continua opaco e `ExportSnapshot` não pode ser especificado
- [ ] **Infra**: `crates/kernel/infra/ingest-documental-adapter` — implementa
  `IngestStoragePort` com `core-documental` + MinIO; só depois de spec estar definida
- [ ] **Facade de pipeline** (depois de adapter): orquestra ingest→documental com
  `correlation_id` único; ver TODO [AuditFacade](memory/project_core_audit_future_audit_facade.md)

**Ordem de execução:**
1. Definir `DocumentPackage` no domínio + porta
2. Spec normordis-spec (schema + fixtures + regras DOC-R*)
3. `documental-sqlite` adapter ou adapter MinIO (infra)
4. `ingest-documental-adapter` (implementa `IngestStoragePort`)
5. Desbloqueio de `ExportSnapshot` + `core-exports` completo

---

### normordis-spec — lacunas restantes

- [ ] **core-documental**: primeiro schema executável (`DocumentPackage`) — **bloqueante**
- [ ] **core-exports**: completar `ExportSnapshot` — bloqueado por `DocumentPackage`
- [ ] **core-metrics**: primeiro schema executável — independente, pode avançar em paralelo
- [ ] Scenario fixtures `core-ingest` (INGEST-R* inter-registo) — base pronta, aguarda uso real

---

## Horizonte 2 — Targets de deploy

O domínio (`core-*`) já não tem dependências de plataforma em runtime.
A camada infra (`*-sqlite`) está isolada por design.

### Tauri (desktop / mobile)

- [ ] Criar `crates/runtime/tauri-runtime` — comandos Tauri finos sobre as facades do kernel
- [ ] Ligar `normordis-kernel` com feature `bootstrap` como backend do Tauri
- Sem alterações ao domínio; os adaptadores SQLite já funcionam nativamente.

### Web app — servidor HTTP

- [ ] Criar `crates/runtime/http-runtime` com [axum](https://github.com/tokio-rs/axum)
- [ ] Expor facades do kernel como endpoints REST/JSON
- [ ] Adicionar `http-runtime` como feature opcional em `normordis-kernel`
- Os adaptadores SQLite existentes funcionam sem alteração como backend.

### Browser — WASM (futuro, sem data)

- [ ] Auditar dependências `support-crypto` / `support-clock` para compatibilidade WASM
- [ ] Criar adaptadores WASM-compatíveis para storage (IndexedDB / OPFS) em alternativa ao SQLite
- [ ] Adicionar feature flags `wasm` nos crates de suporte que usam `chrono::Local` ou rand OS
- Base favorável: `core-*` está limpo, a separação portas/adaptadores suporta swap.

---

## Horizonte 3 — Interoperabilidade Go

Modelo: **serviços autónomos** que partilham contratos via `normordis-spec`. Não misturar Go no workspace Rust.

- [ ] Runner de conformance Go para normordis-spec (antecipado em `conformance/README.md`)
- [ ] Serviço Go opcional como gateway HTTP de alta concorrência (se carga justificar)
- [ ] CLI Go para tooling de distribuição fácil (binário estático sem runtime Rust no host)
- A fronteira de interoperabilidade é o schema — um serviço Go que emita `AuditEvent` conforme ao schema é um cidadão de primeira classe.

---

## Manutenção técnica

- [ ] Mover `windows-sys` de `[workspace.dependencies]` para dependência local em `secrets` com `cfg(target_os = "windows")` — evita visibilidade ao nível do workspace quando compilar para targets não-Windows
- [ ] Extrair `normordis-spec/` para repositório independente (guia em `normordis-spec/EXTRACTING.md`)
- [ ] Activar CI da spec no repo autónomo (`normordis-spec/ci/spec-ci.yml`)

---

## Legenda de horizontes

| Horizonte | Critério |
|-----------|----------|
| **1** | Próximos commits — sem novos runtimes ou dependências externas |
| **2** | Novas crates de runtime — arquitectura suporta, executar quando houver app consumidora |
| **3** | Decisões estratégicas — requerem acordo sobre deploy e infra externa |

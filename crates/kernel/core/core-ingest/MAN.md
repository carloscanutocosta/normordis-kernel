# Manual do Programador — core-ingest

Estado: Estável  
Tipo: Manual técnico por componente  
Âmbito: Ingestão auditável de bundles exportados  
Data: 2026-05-16  
Versão: v0.3.0

---

## Objectivo

`core-ingest` implementa o pipeline de ingestão de `ExportSnapshot` com o
fluxo **validate → hash → scan → route → audit**.

Aceita ou rejeita um bundle de forma auditável, produzindo sempre um
`IngestEvidence` completo e um `AuditEvent` correspondente.

---

## Estrutura do componente

```
crates/kernel/core/core-ingest/
├── Cargo.toml
├── MAN.md
└── src/
    ├── lib.rs          — re-exports públicos
    ├── error.rs        — IngestError com código e is_retryable()
    ├── types.rs        — tipos soberanos, traits, IngestOutcome
    ├── service.rs      — process_export_snapshot e validadores
    └── adapters.rs     — DeterministicScanner, MemoryRouter (uso em testes)
tests/
    └── ingest_tests.rs
```

---

## Contrato público

### Função principal

```rust
pub fn process_export_snapshot(
    req: &IngestRequest,
    correlation_id: &str,
    cfg: &IngestConfig,
) -> IngestOutcome
```

Retorna `IngestOutcome::Accepted(Outcome)` ou `IngestOutcome::Rejected { outcome, error }`.
Em ambos os casos, `Outcome` contém `evidence` e `audit_event` completos.

### Tipos soberanos

| Tipo | Descrição |
|---|---|
| `IngestRequest` | Pedido de ingestão (bundle + hash esperado + source) |
| `IngestEvidence` | Evidência completa do processamento |
| `IngestOutcome` | Resultado tipado: `Accepted` ou `Rejected` |
| `Outcome` | Evidence + audit event (transportado por `IngestOutcome`) |
| `IngestConfig` | Scanner, router, actor, now, limite de bytes |
| `IngestSource` | kind / subject_id / version do pedido |

### Traits de extensão

```rust
pub trait ScanAdapter: Send + Sync {
    fn scan(&self, input: &ScanInput) -> Result<ScanResult, IngestError>;
    fn adapter_id(&self) -> &str;
}

pub trait Router: Send + Sync {
    fn route(&self, input: &RouteInput) -> Result<RouteResult, IngestError>;
}
```

### Validadores directos

```rust
pub fn validate_ingest_request(req: &IngestRequest) -> Result<(), IngestError>
pub fn validate_ingest_evidence(evidence: &IngestEvidence) -> Result<(), IngestError>
pub fn build_ingest_audit_event(evidence: &IngestEvidence, actor: &str) -> Result<AuditEvent, IngestError>
```

### Adaptadores de teste

| Tipo | Descrição |
|---|---|
| `DeterministicScanner` | Rejeita hashes específicos, aceita os restantes |
| `MemoryRouter` | Persiste routes em memória (idempotente por hash) |

---

## Erros canónicos

| Código | Variant | Retryable |
|---|---|---|
| `MINI.INGEST.MISSING_FIELD` | `MissingField { field }` | Não |
| `MINI.INGEST.INVALID_REQUEST` | `InvalidRequest { message }` | Não |
| `MINI.INGEST.HASH_MISMATCH` | `HashMismatch { expected, actual }` | Não |
| `MINI.INGEST.SCAN_FAILED` | `ScanFailed` | **Sim** |
| `MINI.INGEST.SCAN_REJECTED` | `ScanRejected { adapter, verdict }` | Não |
| `MINI.INGEST.ROUTE_UNAVAILABLE` | `RouteUnavailable` | **Sim** |
| `MINI.INGEST.OVERSIZED` | `Oversized { limit_bytes, actual_bytes }` | Não |
| `MINI.INGEST.MARSHAL_FAILED` | `MarshalFailed(String)` | Não |

---

## Integração e limites

- O `correlation_id` é responsabilidade do boundary consumidor.
- Scanner e router são injectados via `IngestConfig`; o módulo não fixa antimalware nem storage.
- O slice actual aceita apenas `source.kind == "config_export_bundle"`.
- `source.subject_id` e `source.version` do pedido devem corresponder ao bundle.
- `audit.emitted` fica `true` apenas quando o `AuditEvent` é construído pelo caminho canónico;
  o evento de emergência (evidence inválida) não marca `emitted`.

---

## Dependências

```
core-audit      — AuditEvent, AuditActor, AuditTarget
core-exports    — ExportSnapshot, canonical_bytes, validate_export_snapshot
core-validation — sha256_bytes
support-errors  — MiniError, ErrorCode, Component
```

Sem dependências de sqlite, tauri, ou qualquer infra.

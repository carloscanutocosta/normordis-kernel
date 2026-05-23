# core-ingest

Ingestão auditável de bundles exportados com pipeline validate → hash → scan → route → audit.

## Responsabilidade

- Validar e processar um `IngestRequest` (bundle + hash esperado + source) de forma atómica.
- Verificar o hash SHA-256 do bundle antes de qualquer scan ou encaminhamento.
- Delegar a análise do bundle a um `ScanAdapter` injectado (antimalware, políticas, etc.).
- Delegar o encaminhamento a um `Router` injectado (storage, fila, outro serviço).
- Produzir sempre um `IngestEvidence` completo e um `AuditEvent` correspondente, tanto em caso de aceitação como de rejeição.
- Classificar erros de infra como retryable (`ScanFailed`, `RouteUnavailable`).

## Não responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não implementa antimalware nem qualquer lógica de scan — delega em `ScanAdapter`.
- Não persiste o bundle nem a evidence — delega em `Router` e no caller.
- Não decide quem pode fazer ingest — a autorização é responsabilidade do caller.
- Não valida permissões organizacionais nem perfis de utilizador.

## Exemplo mínimo

```rust
use core_ingest::{
    process_export_snapshot, DeterministicScanner, IngestConfig,
    IngestOutcome, IngestRequest, IngestSource, MemoryRouter,
};

let cfg = IngestConfig {
    scanner: Some(Box::new(DeterministicScanner::default())),
    router:  Some(Box::new(MemoryRouter::new("ingest/config-bundle"))),
    max_bundle_bytes: None,
    actor: "daemon:apid".into(),
    now: None,
};

match process_export_snapshot(&req, &correlation_id, &cfg) {
    IngestOutcome::Accepted(outcome) => {
        // persistir outcome.evidence e emitir outcome.audit_event
    }
    IngestOutcome::Rejected { outcome, error } => {
        // registar rejeição; error.is_retryable() indica se vale retentar
        // outcome.evidence descreve o ponto exacto de falha
    }
}
```

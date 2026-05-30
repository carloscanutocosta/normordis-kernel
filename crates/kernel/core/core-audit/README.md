# core-audit

Núcleo de auditoria institucional do kernel normordis.

Grava eventos como evidência imutável, liga-os numa cadeia de hashes verificável (SHA-256) e permite exportar e assinar manifestos com Ed25519.

## Fronteira conceptual

```
support-logging  → diagnóstico técnico operacional
core-audit       → evidência institucional auditável
```

`core-audit` não usa `support-logging`, não conhece SQLite, filesystem, Tauri nem configuração de runtime. A persistência física é injectada pelo `runtime/bootstrap`.

## Funcionalidades

- **Append-only:** `event_id` duplicado é rejeitado
- **Integridade por registo:** cada leitura recomputa o SHA-256 e rejeita eventos adulterados
- **Cadeia de hashes:** cada registo encadeia o hash do anterior
- **Verificação completa:** `verify_chain()` valida toda a cadeia de origem
- **Verificação incremental:** `verify_chain_since(N)` verifica apenas eventos novos — escalável para dezenas de milhões de registos
- **Consulta temporal:** `list_by_date_range(from, to, limit, offset)` — intervalo `[from, to[`
- **Manifesto exportável:** `export_manifest()` gera hash do manifesto sobre contagem + cabeça da cadeia
- **Assinatura Ed25519:** `sign_and_export(key, key_id)` produz manifesto assinado verificável por terceiros sem a chave privada

## Utilização rápida

```rust
use core_audit::{AuditActor, AuditService, AuditStoreConfig, AuditTarget, StorageAuditStore};
use support_storage::StorageNamespace;

let config = AuditStoreConfig::new(StorageNamespace::new("audit.events")?);
let store  = StorageAuditStore::new(storage, config);
let svc    = AuditService::new(store);

// Gravar
let event = svc.record_event(
    "document.created",
    AuditActor::new("user-123")?,
    AuditTarget::new("document", "doc-456")?,
    None,
)?;

// Consultar por intervalo temporal
let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
let to   = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
let eventos = svc.list_by_date_range(from, to, 100, 0)?;

// Verificar cadeia completa
let report = svc.verify_chain()?;

// Verificar apenas eventos novos (incremental)
let report_inc = svc.verify_chain_since(ultimo_seq_verificado + 1)?;

// Exportar e assinar manifesto
let key    = AuditSigningKey::from_bytes(raw_bytes);
let signed = svc.sign_and_export(&key, Some("audit-key-1".to_string()))?;
verify_signed_manifest(&signed)?;
```

## Backends

| Backend | Crate | Recomendado para |
|---|---|---|
| `StorageAuditStore<S>` | `core-audit` | Testes e volumes pequenos |
| `AuditSqliteStore` | `adapter-audit-sqlite` | Produção — qualquer volume |

Para produção, use `adapter-audit-sqlite`: suporta verificação incremental eficiente,
`list_by_date_range` com índice SQL e atomicidade nas escritas via SAVEPOINT.

## Documentação detalhada

Ver [MAN.md](MAN.md) para referência completa do contrato, índices internos,
política de dados, tabela de erros e separação de responsabilidades.

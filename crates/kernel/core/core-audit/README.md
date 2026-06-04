# core-audit

Camada de evidência auditável do kernel NORMORDIS.

Grava eventos como evidência imutável, encadeia-os num hash chain SHA-256 verificável, mantém um Registo de Controlos alinhado com COSO e mede conformidade.

## Fronteira conceptual

```
support-logging  → diagnóstico técnico operacional (pode rodar, expirar, ser filtrado)
core-audit       → evidência institucional auditável (append-only, verificável, exportável)
```

O `core-audit` não executa controlos nem decide compliance. Regista, relaciona e preserva evidência da sua execução.

## Dois serviços

| Serviço | Store | Propósito |
|---|---|---|
| `AuditService<S>` | `AuditStore` | Grava eventos, verifica cadeia, exporta manifesto |
| `ControlRegistryService<S>` | `ControlRegistryStore` | Gere catálogo de controlos, regista execuções, mede conformidade |

## Início rápido

### Registar um evento simples

```rust
use core_audit::{
    AuditActor, AuditOutcome, AuditService, AuditStoreConfig, AuditTarget,
    RecordAuditEventRequest, StorageAuditStore,
};

let svc = AuditService::new(
    StorageAuditStore::new(storage, AuditStoreConfig::default()),
);

let event = svc.record_event(
    RecordAuditEventRequest::new(
        "document.created",
        AuditActor::new("user-123")?,
        AuditTarget::new("document", "doc-456")?,
        AuditOutcome::Success,
    )
    .with_details(serde_json::json!({"origem": "interface-web"})),
)?;
```

`record_event` gera UUID v4 para `event_id` e usa `Utc::now()` como timestamp.

### Com controlo COSO e execuções secundárias

```rust
use core_audit::{
    ControlExecutionResult, ControlRegistryConfig, ControlRegistryService,
    StorageControlRegistryStore, builtin_control_catalog,
};

let ctrl_svc = ControlRegistryService::new(
    StorageControlRegistryStore::new(ctrl_storage, ControlRegistryConfig::default()),
);

// Carregar o catálogo base de 50 controlos
for control in builtin_control_catalog() {
    ctrl_svc.define_control(&control)?;
}

// Evento com controlo primário
let event = audit_svc.record_event(
    RecordAuditEventRequest::new(
        "document.classification.changed",
        AuditActor::new("inspector-7")?,
        AuditTarget::new("document", "doc-123")?,
        AuditOutcome::Success,
    )
    .with_control_id("CTRL-AUTH-004"),  // controlo primário: Delegação válida
)?;

// Controlos secundários verificados no mesmo evento
ctrl_svc.record_control_execution(
    "CTRL-TRACE-001", &event.event_id, ControlExecutionResult::Passed, None, None,
)?;
ctrl_svc.record_control_execution(
    "CTRL-INT-001", &event.event_id, ControlExecutionResult::Passed,
    Some("sha256:a3f9...".to_string()), None,
)?;

// Todos os controlos do evento
let execucoes = ctrl_svc.list_executions_by_event(&event.event_id)?;
```

### Conformidade e Balanced Scorecard

```rust
let summary = ctrl_svc.conformance_summary("CTRL-AUTH-004")?;
println!(
    "CTRL-AUTH-004: {}/{} passed ({:.1}%)",
    summary.passed,
    summary.passed + summary.failed,
    summary.conformance_rate().unwrap_or(0.0) * 100.0
);
// CTRL-AUTH-004: 124/125 passed (99.2%)
```

`Dispensed` é contabilizado separadamente — não entra no denominador da taxa de conformidade.

### Verificação e exportação

```rust
// Verificação completa
let report = svc.verify_chain()?;

// Verificação incremental — apenas eventos novos (O(novos_eventos))
let report_inc = svc.verify_chain_since(ultimo_seq + 1)?;

// Manifesto assinado com Ed25519
let key    = AuditSigningKey::from_bytes(raw_key_bytes);
let signed = svc.sign_and_export(&key, Some("audit-key-prod".to_string()))?;
verify_signed_manifest(&signed)?;  // verificável por terceiros sem a chave privada
```

## Catálogo base de controlos

`builtin_control_catalog()` devolve 50 controlos canónicos em 10 categorias:

| Categoria | Prefixo | Pergunta central |
|---|---|---|
| AUTH | `CTRL-AUTH-` | Quem pode fazer? |
| VAL | `CTRL-VAL-` | O ato foi validado? |
| TRACE | `CTRL-TRACE-` | Posso provar? |
| DOC | `CTRL-DOC-` | O documento é válido e controlado? |
| INT | `CTRL-INT-` | Foi alterado? |
| PRIV | `CTRL-PRIV-` | Os dados pessoais estão protegidos? |
| SEC | `CTRL-SEC-` | Foi protegido? |
| ING | `CTRL-ING-` | A entrada de dados foi controlada? |
| EXP | `CTRL-EXP-` | A saída de dados foi autorizada? |
| CONT | `CTRL-CONT-` | Consigo recuperar? |

Referências normativas: COSO · ISO 27001 · ISO 9001 · ISO 15489 · RGPD · eIDAS.

Controlos específicos de domínio de negócio vivem nos respectivos módulos — não neste catálogo.

## Backends

| Backend | Crate | Recomendado para |
|---|---|---|
| `StorageAuditStore<S>` | `core-audit` | Testes e volumes pequenos |
| `StorageControlRegistryStore<S>` | `core-audit` | Testes e volumes pequenos |
| `AuditSqliteStore` + `ControlRegistrySqliteStore` | `adapter-audit-sqlite` | Produção — qualquer volume |

## Documentação detalhada

Ver [MAN.md](MAN.md) para referência completa do contrato, erros, índices internos e separação de responsabilidades.

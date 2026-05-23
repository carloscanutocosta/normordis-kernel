# Manual do modulo core-exports

## Objetivo

`core-exports` implementa o Gate F do ciclo documental: produz um snapshot exportável
de um `DocumentPackage` com hash de manifesto determinístico e evidência de audit
atómica. Contexto AP, princípio zero-trust: todo o export identifica obrigatoriamente
o principal que o originou e a operação de correlação.

## Contrato publico

```rust
// Tipos de dados
ExportSnapshot
ExportReceipt
Manifest
SourceRef
BuildSnapshotConfig

// Erros
ExportError

// Port de persistência
ExportSnapshotPort

// Port de materialização interoperável
ExportFormat
InteroperabilityProfile
TabularDataset
ExportMaterializationRequest
ExportMaterializerPort

// API principal
build_export_receipt(pkg, source, cfg) -> Result<ExportReceipt, ExportError>
validate_export_snapshot(snapshot)     -> Result<(), ExportError>
canonical_bytes(snapshot)              -> Result<Vec<u8>, ExportError>
snapshots_to_csv(snapshots)            -> String

// Bridge de compatibilidade
build_package_from_document_instance(instance, cfg) -> Result<DocumentPackage, ExportError>
```

## Como usar

### Construir um ExportReceipt

```rust
use core_exports::{build_export_receipt, BuildSnapshotConfig, SourceRef};

let receipt = build_export_receipt(
    package,
    SourceRef {
        kind: "config_profile".into(),
        subject_id: "dev".into(),
        version: "1.0.0".into(),
    },
    BuildSnapshotConfig {
        exported_at: None,              // None -> usa package.created_at
        actor: "daemon:apid".into(),    // obrigatorio
        correlation_id: "corr-001".into(), // obrigatorio
        transport: None,                // None -> "inline"
    },
)?;
// receipt.snapshot + receipt.audit_event produzidos atomicamente
```

### Usar o port de persistência

```rust
pub trait ExportSnapshotPort {
    fn save_receipt(&self, receipt: &ExportReceipt) -> Result<(), ExportError>;
    fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<ExportSnapshot>, ExportError>;
    fn list_for_subject(&self, subject_id: &str, limit: usize, offset: usize)
        -> Result<Vec<ExportSnapshot>, ExportError>;
}
```

### Bridge de support-documental

```rust
use core_exports::{build_package_from_document_instance, InstanceBridgeConfig};

let pkg = build_package_from_document_instance(&instance, &InstanceBridgeConfig::default())?;
```

### Export CSV

```rust
use core_exports::snapshots_to_csv;

let csv = snapshots_to_csv(&snapshots); // RFC 4180, CRLF, campos citados se necessario
```

## Invariantes

- `actor` e `correlation_id` sao obrigatorios — nao existe `ExportReceipt` sem identificacao do principal.
- `ExportReceipt` e um par indivisivel: snapshot + audit_event produzidos na mesma chamada.
- O hash do manifesto e determinístico: baseado em SHA-256 sobre `(source, document_package)` com chaves JSON ordenadas recursivamente (BTreeMap). Dois snapshots com o mesmo conteudo produzem sempre o mesmo hash.
- `validate_export_snapshot` re-computa o hash e rejeita snapshots com `manifest.hash` adulterado.
- `item_count` do manifesto deve coincidir com `document_package.artefacts.len()`.
- `canonical_bytes` aplica a mesma normalizacao de chaves que o hash do manifesto — os bytes produzidos sao determinísticos para conteudos equivalentes.

## Formato do snapshot_id

```
exp:{source.kind}:{source.subject_id}:{source.version}:{16 hex chars do hash}
```

Os 16 primeiros caracteres hexadecimais do hash SHA-256 (64 bits) sao usados como
sufixo do identificador. Dentro do mesmo namespace `(kind, subject_id, version)` a
probabilidade de colisao acidental entre conteudos distintos e negligenciavel para os
volumes esperados em mini-apps AP. O hash completo esta sempre em `manifest.hash`.

## Erros

| Codigo                        | Situacao                                                  |
|-------------------------------|-----------------------------------------------------------|
| `MINI.EXPORTS.MISSING_FIELD`  | `actor`, `correlation_id` ou campo de `source` em falta  |
| `MINI.EXPORTS.INVALID_SNAPSHOT` | Snapshot estruturalmente invalido ou hash adulterado    |
| `MINI.EXPORTS.MARSHAL_FAILED` | Falha a serializar para bytes ou JSON                     |
| `MINI.EXPORTS.INVALID_PACKAGE`| `DocumentPackage` invalido (delegado de `core-documental`)|
| `MINI.EXPORTS.AUDIT_ERROR`    | Falha a construir `AuditEvent` (actor ou target invalido) |

## Dependencias

```
core-audit         — AuditEvent, AuditActor, AuditTarget
core-documental    — DocumentPackage, validate_document_package
core-validation    — sha256_bytes
support-errors     — MiniError, ErrorCode, Component
support-documental — DocumentInstance (bridge apenas)
```

`core-exports` nao depende de SQLite, Tauri, filesystem ou UI.

## Decisao de design — audit trail

`ExportReceipt` transporta sempre um `AuditEvent` produzido atomicamente com o
snapshot. O adapter `exports-sqlite` persiste esse evento localmente na tabela
`export_audit_events` como copia de integridade — garante que nenhum snapshot
e guardado sem rasto do evento que o originou.

No entanto, `export_audit_events` nao e a fonte autoritativa de audit: e uma tabela
write-only sem API de leitura no port. Os relatórios e evidencias de audit devem
ser obtidos a partir de `core-audit`.

**Responsabilidade do caller**: apos chamar `port.save_receipt(&receipt)`, o caller
deve encaminhar `receipt.audit_event` para `core-audit` (ou para o servico de audit
do runtime) se pretender que o evento faca parte do trail institucional consultavel.
O `exports-sqlite` nao faz esse encaminhamento — e agnóstico do backend de audit.

```
Fluxo correto:
  1. build_export_receipt(...) → ExportReceipt { snapshot, audit_event }
  2. port.save_receipt(&receipt)              → persiste snapshot + copia local do evento
  3. audit_service.record(receipt.audit_event) → regista no trail institucional (core-audit)
  Passos 2 e 3 sao independentes; o caller e responsavel por ambos.
```

## Limites atuais

- `list_for_subject` e paginada por `limit`/`offset`, mas nao tem cursor opaco nem
  garantia de consistencia entre paginas se houver insercoes concorrentes.
- Sem suporte a outros algoritmos de hash alem de SHA-256.
- Sem revogacao ou invalidacao de snapshots.
- Bridge `build_package_from_document_instance` constroi apenas artefactos de
  `payload_json` e `rendered_html`; outros tipos de artefacto exigem extensao manual.

## ToDo

- Adicionar cursor opaco ou `snapshot_id` como anchor de paginacao para garantir
  estabilidade entre paginas.
- Suporte a outros algoritmos de hash quando houver requisito institucional.
- Mecanismo de revogacao/invalidacao de snapshots (ex: auditoria de retirada).
- Validacao do formato de `source.version` (ex: semver ou padrao AP).
## Materializacao interoperavel

`core-exports` define `ExportFormat`, `InteroperabilityProfile`,
`TabularDataset`, `ExportMaterializationRequest` e `ExportMaterializerPort`.
Estes tipos sao o contrato comum que as mini-apps e `support-interoperability`
devem conhecer. A escrita concreta dos formatos vive em adapters de infra, como
`infra-export`.

Regras:

- `snapshot_id`, `output_ref`, `columns` e `rows` sao obrigatorios.
- `output_ref` e uma referencia abstrata de destino; o adapter decide se a
  interpreta como path local, URI interna ou outro backend.
- O port devolve artefactos com `kind`, `output_ref` e `hash`.

# core-exports

Snapshot exportável de documentos institucionais e governação de Gate F.

## Responsabilidade

- Construir `ExportReceipt` (snapshot + evento de audit) de forma atómica a partir de um `DocumentPackage`.
- Garantir que todo o export tem `actor` e `correlation_id` identificados (invariante zero-trust para AP).
- Calcular e validar o hash determinístico do manifesto (SHA-256, chaves ordenadas).
- Exportar snapshots para CSV (RFC 4180).
- Expor o port `ExportSnapshotPort` para adapters de persistência.
- Definir o contrato comum de materialização interoperável via
  `ExportMaterializerPort`, sem acoplar a filesystem ou adapters concretos.

## Não responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não faz persistência — delega em adapters via `ExportSnapshotPort`.
- Não materializa CSV/XML/SQLite/XLSX diretamente — delega em adapters via
  `ExportMaterializerPort`.
- Não decide quem pode exportar — a autorização é responsabilidade do caller.
- Não valida permissões organizacionais nem perfis de utilizador.

## Exemplo mínimo

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
        exported_at: None,          // usa package.created_at
        actor: "daemon:apid".into(),
        correlation_id: "corr-001".into(),
        transport: None,            // default: "inline"
    },
)?;

// Persistir via adapter
port.save_receipt(&receipt)?;
```

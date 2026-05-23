# core-documental

Domínio de custódia documental institucional do Mini-Kernel RS.

## Responsabilidade

- Ciclo de vida completo de documentos institucionais: criação, edição, aprovação, finalização, arquivo e anulação.
- Captura imutável de autoridade jurídica no momento de finalização (`AuthorityContext`).
- Templates NDT versionados e write-once após activação.
- Arquivo NDF write-once com verificação de integridade por hash.
- Gestão de anexos binários endereçados por conteúdo (SHA-256).
- Log de eventos append-only com cadeia de hashes verificável.
- Envelope canónico `DocumentPackage` para exportação (Gate F).
- Ports de persistência (hexagonal): `DocumentCustodyRepository`, `TemplateRepository`, `NdfArchive`, `DocumentEventLog`, `AttachmentStore`.

## Não responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não contém lógica de autorização (quem pode fazer o quê) — essa responsabilidade é do service layer ou do host.
- Não gera identificadores — os IDs são gerados externamente e passados ao domínio.
- Valida `DocumentId` como chave canónica segura para persistência: ASCII alfanumérico,
  hífen, underscore ou ponto, sem espaços, `..` ou separadores de caminho.
- Não gera hashes — delega o cálculo ao caller (infra/service layer) e verifica.
- Não implementa fluxos de assinatura multi-fase — esse contrato pertence a um futuro `DocumentSigningService`.

## Exemplo mínimo

```rust
use core_documental::{
    DocumentCustody, DocumentId, DocumentStatus, TemplateId,
};

let doc = DocumentCustody {
    id: DocumentId::new("doc-oficio-001")?,
    document_type: "oficio".into(),
    template_id: TemplateId::new("tpl-oficio-v1")?,
    template_version: "v1".into(),
    status: DocumentStatus::Draft,
    payload_json: serde_json::json!({ "assunto": "Pedido de informação" }),
    authority_context: None,
    document_number: None,
    created_at: chrono::Utc::now(),
    updated_at: chrono::Utc::now(),
};

// Para finalizar, capturar autoridade e número primeiro:
// doc.authority_context = Some(authority);
// doc.document_number = Some("2026/001".into());
// doc.status = DocumentStatus::Approved;
// let next_status = doc.finalize()?; // → DocumentStatus::Finalized
```

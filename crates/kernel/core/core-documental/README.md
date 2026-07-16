# core-documental

**Custodiante institucional de documentos finalizados** — bounded context do
Mini-Kernel RS responsável pela guarda, rastreabilidade, ciclo de vida custodial e
conformidade RADA (Regulamento de Arquivo e Documentação Administrativa) de documentos
produzidos pelos vários pipelines da plataforma Normordis.

---

## Bounded context e responsabilidades

### O que este core faz

| Responsabilidade | Como |
|---|---|
| Aceitar documentos finalizados em custódia institucional | `DocumentCustody::accept(IntakeSpec)` |
| Garantir identidade única e verificável | `DocumentId` + `ValidationCode` NORD-XXXX-XXXX-XX |
| Gerir o ciclo de vida custodial (Active → Archived/Revoked/Superseded) | `DocumentCustodyService` |
| Registar o actor e a autoridade de cada acto | `AuthoritySource`, `EventActor`, `AuthoritySnapshot` |
| Rastrear todo o acesso com propósito declarado | `AccessPurpose` + `DocumentEventLog` |
| Enforçar a política de retenção RADA | `RetentionPolicy::temporary(years)` / `::permanent()` |
| Guardar anexos binários com integridade por hash (SHA-256) | `AttachmentStore` + `MAX_ATTACHMENT_BYTES` |
| Registar renders NDF de forma write-once | `NdfArchive` |
| Manter log de eventos com cadeia de hashes verificável | `DocumentEvent.previous_hash` |
| Validar envelopes de exportação (Gate F) | `validate_document_package` |

### O que este core **não** faz

- **Não** gere rascunhos, versões, aprovações ou assinaturas — esses conceitos
  pertencem a `core-ingest`, `core-exports` e aos mini-apps de produção.
- **Não** controla permissões de acesso — a autorização é delegada ao caller
  (`core-security`, camada de aplicação).
- **Não** valida o schema do conteúdo JSON dos documentos — apenas verifica que
  é JSON sintacticamente válido.
- **Não** executa operações assíncronas — todos os ports são síncronos (escolha
  deliberada para testabilidade e previsibilidade).
- **Não** depende de nenhum outro bounded context do kernel (`core-rh`, `core-org`,
  `core-audit`, `core-security`).

---

## Início rápido

### Intake por utilizador humano

```rust
use core_documental::{
    AuthoritySnapshot, AuthoritySource, DocumentContent, DocumentCustody,
    DocumentId, DocumentOrigin, DocumentStatus, DocumentTypeCode,
    EntryChannel, IntakeSpec, RetentionPolicy, TemplateId, ValidationCode,
};
use chrono::Utc;

let spec = IntakeSpec {
    id: DocumentId::new("doc-certidao-2026-001")?,
    document_type: DocumentTypeCode::new("certidao-residencia")?,
    validation_code: ValidationCode::generate(),   // NORD-XXXX-XXXX-XX
    origin: DocumentOrigin::Normordis,
    entry_channel: EntryChannel::new("miniapp-certidoes")?,
    authority: AuthoritySource::User(AuthoritySnapshot {
        user_id: "joao.silva".into(),
        position_id: "pos-chefe-sf-001".into(),
        unit_id: "unit-sf-vfx".into(),
        competency_id: "comp-certidoes".into(),
        delegation_id: None,
        captured_at: Utc::now(),
    }),
    content: Some(DocumentContent::new(
        r#"{"assunto":"Certidão de residência","requerente":"Maria José"}"#,
    )?),
    template_id: Some(TemplateId::new("tpl-certidao-v2")?),
    template_version: Some("v2".into()),
    retention_policy: RetentionPolicy::temporary(10, Utc::now()),
    received_at: Utc::now(),
    custodied_at: Utc::now(),
};

let doc = DocumentCustody::accept(spec)?;
assert_eq!(doc.status, DocumentStatus::Active);
// doc.validation_code.as_str() → "NORD-K7P9-4Q2M-8X"  (único, verificável)
```

### Intake automatizado (batch, scanner, migração)

```rust
authority: AuthoritySource::System {
    system_id: "core-ingest-batch".into(),
    justification: "importação automática — lote 2026-06-13 #42".into(),
},
```

### Transições custodiais via serviço de aplicação

```rust
use core_documental::service::DocumentCustodyService;

let service = DocumentCustodyService::new(repository.clone(), event_log.clone());

// Arquivar
service.archive(
    &doc_id, actor.clone(), event_id.clone(), Utc::now(), last_hash.clone()
)?;

// Revogar (Active ou Archived → Revoked, terminal)
service.revoke(
    &doc_id, actor.clone(), event_id.clone(), Utc::now(), last_hash.clone()
)?;

// Acesso rastreado com propósito declarado
let maybe_doc = service.get_with_access_log(
    &doc_id, actor, AccessPurpose::LegalProceeding, event_id, Utc::now(), last_hash,
)?;
```

---

## Modelo de estados custodiais

```
        accept()
  ─────────────────────────► Active
                               │
             archive() ────────┤──────────────────────────► Archived
             revoke()  ────────┼─────────────────────────────────────► Revoked   (terminal)
             supersede() ──────┘──────────────────────────────────────► Superseded (terminal)
                                                               │
                                           revoke() ◄──────────┘
                                               ▼
                                           Revoked (terminal)
```

A máquina de estados é validada em domínio por `DocumentStatus::can_transition_to`.
O adapter persiste estado + evento numa única transacção — atomicidade garantida pelo port.

---

## Modelo de autoridade

Todo o acto em custódia é atribuído a um principal — humano ou sistémico:

```rust
// Utilizador com posição orgânica (snapshot jurídico completo)
AuthoritySource::User(AuthoritySnapshot {
    user_id, position_id, unit_id, competency_id,
    delegation_id,   // Some("del-123") se por delegação
    captured_at,
})

// Processo autónomo sem posição orgânica
AuthoritySource::System { system_id, justification }
```

O `AuthoritySnapshot` é imutável e contém apenas primitivos — sem referências a
objectos externos. A construção a partir do contexto de sessão pertence à camada de
aplicação (`miniapp-runtime`), não ao domínio.

No log de eventos, o actor é representado por `EventActor`:

| Variante | Uso |
|---|---|
| `Operator { user_id, position_id }` | Operações de runtime sem snapshot completo |
| `Authority(AuthoritySnapshot)` | Actos com relevância legal (snapshot completo) |
| `System { system_id, justification }` | Processos autónomos |

---

## Conformidade e rastreabilidade

| Requisito | Solução |
|---|---|
| RADA — retenção documental | `RetentionPolicy::temporary(years)` com aritmética de calendário real |
| RADA — classes de retenção | `RetentionClass` (enum definido em `RetentionPolicy`) |
| Autenticidade e integridade | `ValidationCode` NORD-XXXX-XXXX-XX (Crockford Base32) |
| Imutabilidade de eventos | Log append-only; adapters proíbem UPDATE/DELETE em eventos |
| Cadeia de prova auditável | `DocumentEvent.previous_hash` verificável por `verify_event_chain` |
| Integridade de anexos | SHA-256 content-addressed; `verify_content_integrity` |
| Integridade de NDF | `NdfRecord.ndf_hash`; `verify_integrity(computed_hash)` |
| Acesso com propósito declarado | `AccessPurpose` (6 variantes) em `get_with_access_log` |
| Actor de cada acto identificado | `AuthoritySource` no intake; `EventActor` no log |
| COSO — evidência de controlo | `EventActor`, snapshot de autoridade, cadeia de eventos |

---

## Dependências

```
core-documental
  ├── core-doc-types        (shared kernel neutro — DocumentPackage, TemplateRef, …)
  ├── serde / serde_json    (serialização)
  ├── chrono                (datas e aritmética de retenção)
  └── rand                  (geração de ValidationCode)
```

**Zero dependências de outros bounded contexts.** A construção de snapshots de
autoridade a partir de `core_rh::UserContext` é feita na camada de runtime, não aqui.

---

## Ports e adapters

| Port | Contrato | Adapter disponível |
|---|---|---|
| `DocumentCustodyRepository` | intake, lookup, transições atómicas, relações | `documental-sqlite` |
| `DocumentEventLog` | append-only, pesquisa por filtro | `documental-sqlite` |
| `AttachmentStore` | store + retrieve + integridade | `documental-sqlite` |
| `NdfArchive` | write-once, verify | `documental-sqlite` |
| `TemplateRepository` | templates versionados, write-once após activação | `documental-sqlite` |

Todos os ports são traits síncronos. O adapter `documental-sqlite` implementa todos
os cinco numa única ligação SQLite com WAL activado.

---

## Cobertura de testes

**80 testes** no crate de domínio (`src/tests.rs`), sem dependências externas:

- Factory `accept()` e validação de invariantes
- Máquina de estados — todas as transições válidas e inválidas
- `ValidationCode` — geração, parsing, rejeição de formato inválido
- `RetentionPolicy` — aritmética de calendário (incluindo 29 de Fevereiro)
- `DocumentContent` — aceitação de JSON válido, rejeição de não-JSON
- `AuthoritySource::System` — invariantes de campos obrigatórios
- `EventActor::System` — serialização serde
- `NdfRecord` e `DocumentAttachment` — validação e integridade
- Cadeia de eventos — `canonical_bytes` determinístico, `verify_event_chain`
- Relações entre documentos

Testes de integração com SQLite real em `crates/kernel/infra/documental-sqlite`.

---

## Lacunas conhecidas

Dois gaps arquitectónicos estão documentados e serão resolvidos em iteração futura.
Ver [MAN.md § Lacunas conhecidas e TODO](MAN.md#lacunas-conhecidas-e-todo) para
descrição completa e critérios de resolução.

1. **[TODO-1] Campos públicos no agregado** — `DocumentCustody.status` e outros
   campos são `pub`; enforcement de invariantes por convenção, não pelo compilador.

2. **[TODO-2] Dois caminhos para evento `CustodyAccepted`** — o service usa
   `EventActor::Authority(snapshot)` (completo); o helper de runtime usa
   `EventActor::Operator` (snapshot incompleto).

---

## Documentação de referência

- [MAN.md](MAN.md) — manual técnico completo: contrato API, invariantes, decisões
  de design, todos os ports, cadeia de eventos, erros e TODO
- `crates/kernel/infra/documental-sqlite` — adapter SQLite (schema, migrações,
  testes de integração)
- `crates/kernel/core/core-doc-types` — tipos partilhados de envelope de exportação
- `crates/runtime/miniapp-runtime` — construção de `AuthoritySnapshot` a partir de
  contexto de sessão; integração com `DocumentCustodyService`

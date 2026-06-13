# Manual técnico — core-documental

## Objetivo e âmbito

`core-documental` é o **custodiante institucional** de documentos finalizados no
Mini-Kernel RS. Recebe documentos já produzidos e aprovados, guarda-os com prova de
autoridade, rastreia o acesso e gere o ciclo de vida custodial.

**Não** é um motor de workflow. Não conhece rascunhos, aprovações, assinaturas nem
fases de produção. Esses conceitos pertencem ao pipeline de produção (core-ingest,
core-exports, miniapp-runtime). O core-documental só entra em cena quando o documento
está pronto para guarda institucional definitiva.

---

## Contrato público

### Tipos principais

```rust
// Custódia
DocumentCustody        // agregado central — ciclo de vida custodial
IntakeSpec             // input da factory DocumentCustody::accept()
DocumentId             // identificador path-safe (ASCII alfanum, -, _, .)
DocumentStatus         // Active | Archived | Revoked | Superseded
DocumentTypeCode       // código normalizado (minúsculas, ASCII alfanum + - _)
DocumentContent        // conteúdo JSON opaco imutável (validado em new())
ValidationCode         // NORD-XXXX-XXXX-XX Crockford Base32
EntryChannel           // canal de entrada (livre, não vazio)
DocumentOrigin         // Normordis | Email | Scanner | Upload | Api | Interop | Legacy
DocumentRelation       // relação entre documentos (from_id, to_id, RelationType)
RelationType           // ReplyTo | References | Supersedes | Annuls | AnnexDocument
RetentionPolicy        // política RADA (Permanent | Temporary { years })
RetentionClass

// Autoridade
AuthoritySource        // User(AuthoritySnapshot) | System { system_id, justification }
AuthoritySnapshot      // snapshot jurídico imutável (user_id, position_id, unit_id, …)

// Templates
DocumentTemplate       // template NDT versionado — write-once após activação
TemplateId
TemplateStatus         // Active | Deprecated

// Arquivo NDF
NdfRecord              // registo de render NDF — write-once
NdfRecordId

// Anexos
DocumentAttachment     // metadados de blob binário em guarda
AttachmentId
AttachmentKind         // Annex | Incoming

// Eventos
DocumentEvent          // evento append-only com cadeia de hashes
DocumentEventId
DocumentEventType      // CustodyAccepted | NumberAssigned | Accessed |
                       // RelationAdded | AttachmentAdded | StatusChanged
EventActor             // Operator { user_id, position_id }
                       // | Authority(AuthoritySnapshot)
                       // | System { system_id, justification }
AccessPurpose          // Consultation | Inspection | Export | LegalProceeding |
                       // Audit | InternalReview
EventFilter            // filtro de pesquisa no log de eventos

// Envelope de exportação (re-exportado de core-doc-types)
DocumentPackage        // envelope canónico para Gate F
TemplateRef
EngineRef
Artefact
HashResult

// Erro
DocumentalError

// Constante
MAX_ATTACHMENT_BYTES   // 50 MiB — limite para AttachmentStore::store
```

### Ports de persistência

```rust
DocumentCustodyRepository  // intake, lookup e transições custodiais
TemplateRepository         // templates versioned + write-once
NdfArchive                 // registos NDF write-once
DocumentEventLog           // log de eventos append-only
AttachmentStore            // blobs binários content-addressed
```

### Funções utilitárias

```rust
validate_document_package(pkg) -> Result<(), DocumentalError>
verify_event_chain(events)     -> Result<(), DocumentalError>
```

### Serviço de aplicação

```rust
DocumentCustodyService<R, L>
  ::accept_into_custody(doc, event_id, occurred_at) -> Result<()>
  ::archive(id, actor, event_id, occurred_at, prev_hash) -> Result<()>
  ::revoke(id, actor, event_id, occurred_at, prev_hash) -> Result<()>
  ::supersede(id, successor_id, actor, …) -> Result<()>
  ::assign_number(id, number, actor, …) -> Result<()>
  ::get_with_access_log(id, actor, purpose, …) -> Result<Option<DocumentCustody>>
```

---

## Factory — DocumentCustody::accept

`DocumentCustody::accept(IntakeSpec)` é o único caminho legítimo para aceitar um
documento em custódia. Valida todos os invariantes antes de construir o agregado.

```rust
use core_documental::{
    AuthoritySnapshot, AuthoritySource, DocumentContent, DocumentCustody, DocumentId,
    DocumentOrigin, DocumentTypeCode, EntryChannel, IntakeSpec, RetentionPolicy,
    TemplateId, ValidationCode,
};
use chrono::Utc;

// Intake por utilizador humano
let spec = IntakeSpec {
    id: DocumentId::new("doc-oficio-2026-001")?,
    document_type: DocumentTypeCode::new("oficio")?,
    validation_code: ValidationCode::generate(),
    origin: DocumentOrigin::Normordis,
    entry_channel: EntryChannel::new("miniapp")?,
    authority: AuthoritySource::User(AuthoritySnapshot {
        user_id: "joao.silva".into(),
        position_id: "pos-001".into(),
        unit_id: "unit-sf-vfx".into(),
        competency_id: "comp-despacho".into(),
        delegation_id: None,
        captured_at: Utc::now(),
    }),
    content: Some(DocumentContent::new(r#"{"assunto":"Certidão de residência"}"#)?),
    template_id: Some(TemplateId::new("tpl-certidao-v1")?),
    template_version: Some("v1".into()),
    retention_policy: RetentionPolicy::temporary(10, Utc::now()),
    received_at: Utc::now(),
    custodied_at: Utc::now(),
};
let doc = DocumentCustody::accept(spec)?;
assert_eq!(doc.status, DocumentStatus::Active);
```

Para intake automatizado (batch, scanner, migração):

```rust
authority: AuthoritySource::System {
    system_id: "core-ingest".into(),
    justification: "importação automática — lote 2026-06-13 #42".into(),
},
```

Struct literal directo de `DocumentCustody` só deve ser usado por adapters ao
**reconstituir** documentos da base de dados. Qualquer reconstitução deve chamar
`doc.validate()` antes de usar o agregado.

---

## Máquina de estados

```
Active ──→ Archived
Active ──→ Revoked
Active ──→ Superseded
Archived ──→ Revoked
```

`Revoked` e `Superseded` são estados terminais sem retorno.

Regras validadas por `DocumentStatus::can_transition_to`:
- `Active → Archived | Revoked | Superseded` ✓
- `Archived → Revoked` ✓
- Todos os outros pares: inválidos

A persistência da transição é responsabilidade do adapter (deve ser atómica com o
evento correspondente).

---

## ValidationCode

Formato canónico: `NORD-XXXX-XXXX-XX` com alfabeto Crockford Base32 (exclui I, L, O, U
para eliminar ambiguidade de leitura humana). 32^10 ≈ 1,1 × 10¹⁵ combinações.

```rust
let code = ValidationCode::generate();     // gerado aleatoriamente
let code = ValidationCode::new("NORD-K7P9-4Q2M-8X")?; // validado e normalizado
```

Análogo ao ATCUD português — identificador público para leitura de QR code em
fiscalização ou verificação de autenticidade.

---

## RetentionPolicy — conformidade RADA

```rust
RetentionPolicy::permanent()               // conservação definitiva
RetentionPolicy::temporary(10, Utc::now()) // 10 anos a partir da custódia
```

`expires_at` é computado com aritmética de calendário real (`NaiveDate::from_ymd_opt`)
— sem o erro de usar `Duration::days(anos × 365)` que ignora anos bissextos. Em 29
de Fevereiro, o fallback é `366 dias × anos`.

---

## AuthoritySource e AuthoritySnapshot

`AuthoritySource` representa quem aceitou o documento em custódia:

```rust
// Utilizador humano com posição orgânica activa
AuthoritySource::User(AuthoritySnapshot {
    user_id: "joao.silva",
    position_id: "pos-001",
    unit_id: "unit-sf-vfx",
    competency_id: "comp-despacho",
    delegation_id: Some("del-123"), // se por delegação
    captured_at: Utc::now(),
})

// Processo de sistema sem posição orgânica
AuthoritySource::System {
    system_id: "scanner-batch",
    justification: "digitalização automática — lote nocturno 2026-06-13",
}
```

`AuthoritySnapshot` armazena apenas primitivos (String, DateTime) — sem dependência de
tipos de outros bounded contexts. A construção a partir de `core_rh::UserContext`
pertence à camada de aplicação (ex: `miniapp-runtime::authority_from_user_context`).

`stored_by` em `DocumentAttachment` e `rendered_by` em `NdfRecord` usam igualmente
`AuthoritySource` — processos de sistema podem armazenar anexos e registos NDF sem
requerer posição orgânica humana.

---

## EventActor

```rust
// Actor humano com posição (sem snapshot completo) — operações de runtime
EventActor::Operator { user_id: String, position_id: String }

// Actor com snapshot jurídico completo — actos com relevância legal
EventActor::Authority(AuthoritySnapshot)

// Processo autónomo — sem posição orgânica
EventActor::System { system_id: String, justification: String }
```

`DocumentCustodyService::accept_into_custody` mapeia automaticamente:
- `AuthoritySource::User(snapshot)` → `EventActor::Authority(snapshot)`
- `AuthoritySource::System { … }` → `EventActor::System { … }`

---

## DocumentContent

Conteúdo estruturado imutável — armazenado como JSON opaco. O domínio não interpreta
a estrutura; a validação de esquema é responsabilidade do produtor (core-ingest).

```rust
DocumentContent::new(r#"{"assunto":"Certidão"}"#)?  // valida JSON em new()
DocumentContent::new("não sou json")?               // → Err(InvalidContent)
DocumentContent::new("42")?                         // ✓ qualquer valor JSON válido
```

---

## Ports de persistência

### DocumentCustodyRepository

```rust
// Intake
fn accept(&self, doc: &DocumentCustody, event: &DocumentEvent) -> Result<()>

// Lookup
fn get(&self, id: &DocumentId) -> Result<Option<DocumentCustody>>
fn lookup_by_validation_code(&self, code: &ValidationCode) -> Result<Option<DocumentCustody>>
fn lookup_by_document_number(&self, number: &str) -> Result<Option<DocumentCustody>>
fn list_by_type(&self, doc_type: &DocumentTypeCode, limit, offset) -> Result<Vec<_>>
fn list_by_status(&self, status: &DocumentStatus, limit, offset) -> Result<Vec<_>>
fn count_by_status(&self, status: &DocumentStatus) -> Result<u64>

// Transições atómicas (estado + evento numa transacção)
fn transition_status(&self, id, status, event) -> Result<()>
fn assign_number(&self, id, number, event) -> Result<()>
fn supersede(&self, id, relation, status_event, relation_event) -> Result<()>

// Relações
fn add_relation(&self, relation, event) -> Result<()>
fn list_relations(&self, id) -> Result<Vec<DocumentRelation>>
```

**Todos os métodos de escrita são atómicos**: o adapter persiste o documento e o
evento correspondente numa única transacção. Documento e evento nunca podem divergir.

### AttachmentStore

```rust
fn store(&self, attachment: &DocumentAttachment, content: &[u8]) -> Result<()>
fn store_from_path(&self, attachment, path: &Path) -> Result<()>  // default
fn retrieve_content(&self, id) -> Result<Option<Vec<u8>>>
fn get_metadata(&self, id) -> Result<Option<DocumentAttachment>>
fn list_for_document(&self, id) -> Result<Vec<DocumentAttachment>>
fn delete_if_unreferenced(&self, id) -> Result<bool>
```

`store` deve verificar `sha256(content) == attachment.content_hash` e rejeitar com
`ContentHashMismatch` se não coincidir. Deve rejeitar com `AttachmentTooLarge` se
`content.len() > MAX_ATTACHMENT_BYTES` (50 MiB).

`store_from_path` lê o ficheiro, verifica o tamanho antes de alocar, e delega para
`store`.

### NdfArchive

Write-once — `write_once` deve rejeitar com `NdfRecordAlreadyExists` se já existir
registo com o mesmo id.

### DocumentEventLog

Append-only — implementações devem proibir UPDATE ou DELETE nos eventos persistidos.

---

## Cadeia de eventos

`DocumentEvent.previous_hash` encadeia eventos em sequência verificável:

```
ev-1 (previous_hash: None / GENESIS)
ev-2 (previous_hash: sha256(canonical_bytes(ev-1)))
ev-3 (previous_hash: sha256(canonical_bytes(ev-2)))
```

`canonical_bytes()` produz bytes determinísticos:
```
{event_id}:{document_id}:{event_type}:{occurred_at_iso8601}:{prev_hash_or_GENESIS}
```

`verify_event_chain(events)` valida **estrutura** (primeiro evento sem `previous_hash`,
sequência tem `previous_hash`, timestamps monotónicos). **Não** recomputa hashes
criptográficos — essa verificação é responsabilidade do adapter que conhece o
algoritmo de hash usado.

---

## Invariantes

| Invariante | Enforcement |
|---|---|
| `DocumentId` path-safe (sem `..`, `/`, `\`) | `DocumentId::new` + `validate()` |
| `ValidationCode` formato Crockford Base32 | `ValidationCode::new` + `validate()` |
| `DocumentContent` é JSON válido | `DocumentContent::new` |
| `document_number` atribuído no máximo uma vez | `assign_number` + adapter |
| `AuthoritySource::System` exige `system_id` e `justification` não vazios | `accept` + `validate()` |
| Transições custodiais respeitam máquina de estados | `transition_to` |
| Estados terminais (`Revoked`, `Superseded`) sem retorno | `can_transition_to` |
| `NdfRecord` write-once | `NdfArchive::write_once` (adapter) |
| Templates `Active`/`Deprecated` imutáveis | `TemplateImmutable` (adapter) |
| Log de eventos append-only | proibição de UPDATE/DELETE no adapter |
| `sha256(content) == content_hash` antes de persistir | `AttachmentStore::store` (adapter) |
| Tamanho máximo de anexo: 50 MiB | `AttachmentStore::store` (adapter) |

---

## Erros

| Variante | Situação |
|---|---|
| `DocumentNotFound` | Documento não encontrado por id |
| `InvalidStatusTransition(from, to)` | Transição não permitida pela máquina de estados |
| `DocumentImmutable` | Atribuição de número em documento não-Active |
| `NumberAlreadyAssigned` | Segunda atribuição de número |
| `EmptyDocumentNumber` | Número vazio em `assign_number` |
| `TemplateImmutable` | Modificação de template Active ou Deprecated |
| `NdfRecordAlreadyExists` | Segunda escrita com mesmo id em NdfArchive |
| `NdfHashMismatch` | Hash NDF não coincide com o registado |
| `ContentHashMismatch` | Hash de conteúdo (anexo/template) não coincide |
| `EventChainBroken` | Cadeia de eventos com previous_hash inconsistente |
| `EmptyField` | Campo obrigatório vazio |
| `InvalidIdentifier { field, reason }` | Identificador fora da forma canónica |
| `InvalidContent` | Conteúdo não é JSON válido |
| `InvalidPackage` | DocumentPackage inválido |
| `AttachmentTooLarge { size_bytes, max_bytes }` | Anexo excede MAX_ATTACHMENT_BYTES |
| `OperationFailed` | Erro de infra ou operação genérica |
| `ActiveTemplateExists` | Template Active já existe para o tipo documental |

`DocumentalError` não implementa `From<MiniError>`. É um erro de domínio puro.
A conversão para erros de infra é responsabilidade dos adapters ou do service layer.

---

## Modelo de segurança

### O que este crate garante

- **Integridade referencial** — um documento só existe em custódia depois de passar
  pela factory `accept()`. Não é possível persistir um `DocumentCustody` inválido
  via os ports sem que o adapter encontre o invariante violado.
- **Imutabilidade do histórico** — `DocumentEvent` nunca é modificado após escrita.
  O adapter que violar isso compromete a cadeia de evidência.
- **Rastreabilidade de actor** — cada escrita regista quem a realizou (`EventActor`)
  e em que momento (`occurred_at`). Não existe operação de escrita sem actor.
- **Integridade de conteúdo binário** — `AttachmentStore::store` rejeita conteúdo
  cujo SHA-256 não coincide com `attachment.content_hash`.
- **Limite de tamanho** — `MAX_ATTACHMENT_BYTES = 50 MiB` enforçado antes de qualquer
  alocação de buffer.

### O que o caller deve garantir

- **Autorização** — `core-documental` não verifica se o utilizador tem permissão
  para realizar a operação. O caller (service layer, `miniapp-runtime`, API handler)
  deve validar a autorização *antes* de invocar os ports.
- **Unicidade do `DocumentId`** — gerado externamente. O caller deve garantir que
  não reutiliza IDs entre documentos distintos.
- **Unicidade do `EventId`** — gerado externamente (UUID v4 recomendado). IDs
  duplicados resultam em violação de constraint no adapter.
- **Valor de `previous_hash`** — o caller deve passar o hash do último evento
  persistido. `DocumentCustodyService` aceita este valor sem recomputar; a
  verificação criptográfica completa é responsabilidade do adapter ou de um job
  de auditoria periódico.
- **Atomicidade das transacções** — os ports de escrita são definidos como atómicos
  por contrato, mas é o adapter que implementa a transacção. Um adapter defeituoso
  que não use transacções pode deixar documento e evento dessincronizados.
- **Confidencialidade do conteúdo** — `DocumentContent` é armazenado em texto claro
  no domínio. Cifra em repouso é responsabilidade do adapter de storage.

### Pressupostos de segurança

- `AuthoritySnapshot` é construído a partir de uma sessão autenticada e autorizada.
  O domínio assume que o snapshot é genuíno — não o valida contra `core-rh`.
- `ValidationCode` é um identificador de autenticidade, não um segredo. É adequado
  para QR code público e verificação por terceiros. Não confere autorização.

---

## Concorrência e thread safety

Todos os tipos do domínio (`DocumentCustody`, `DocumentEvent`, value objects) são
`Send + Sync` implicitamente (contêm apenas `String`, `DateTime<Utc>`, enums simples).

Os ports (`DocumentCustodyRepository`, `DocumentEventLog`, etc.) são traits — a
implementação de thread safety é responsabilidade do adapter. O adapter SQLite usa
`Arc<Mutex<Connection>>` para serializar acessos.

**Atenção a race conditions de estado custodial**: se dois callers tentarem
`transition_status` concorrentemente para o mesmo documento, o adapter deve usar
transacção com locking apropriado (ex: `BEGIN IMMEDIATE` em SQLite) para garantir
que apenas uma transição prevalece. O domínio não implementa Optimistic Concurrency
Control (OCC) — essa responsabilidade é do adapter ou do service layer.

---

## Fluxo de integração típico

### Caso 1 — intake via mini-app (utilizador humano)

```
miniapp-runtime
  ├─ 1. Valida autorização (core-security)
  ├─ 2. Constrói AuthoritySnapshot a partir de UserContext
  ├─ 3. Chama DocumentCustody::accept(IntakeSpec) → DocumentCustody
  ├─ 4. Constrói DocumentEvent (CustodyAccepted, EventActor::Authority)
  └─ 5. Chama DocumentCustodyService::accept_into_custody(doc, event_id, occurred_at)
            └─ adapter persiste doc + evento numa transacção
```

### Caso 2 — intake batch (processo de sistema)

```
core-ingest / batch job
  ├─ 1. Lê ficheiro de lote
  ├─ 2. Constrói IntakeSpec com AuthoritySource::System { system_id, justification }
  ├─ 3. Chama DocumentCustody::accept(spec) → DocumentCustody
  └─ 4. Chama DocumentCustodyService::accept_into_custody(…)
```

### Caso 3 — acesso rastreado

```
miniapp / API handler
  ├─ 1. Valida autorização
  ├─ 2. Chama service.get_with_access_log(id, actor, purpose, event_id, at, prev_hash)
  │        ├─ repo.get(id) → DocumentCustody
  │        └─ event_log.append(DocumentEvent { Accessed, actor, purpose })
  └─ 3. Retorna documento ao caller
```

### Caso 4 — transição custodial (arquivo)

```
miniapp / API handler
  ├─ 1. Valida autorização (apenas gestores podem arquivar)
  ├─ 2. Consulta prev_hash do último evento para encadeamento
  └─ 3. Chama service.archive(id, actor, event_id, occurred_at, prev_hash)
            └─ adapter: transition_status atómica (doc + evento)
```

---

## Contrato do adapter

Todo o adapter que implementa os ports deste crate deve respeitar os seguintes
invariantes, sob pena de corromper a integridade da custódia:

| Invariante | Obrigação do adapter |
|---|---|
| **Atomicidade de escrita** | Cada método de escrita persiste entidade + evento numa única transacção. Nunca um sem o outro. |
| **Append-only de eventos** | Proibir UPDATE e DELETE na tabela de eventos. Não expor métodos de remoção via o port. |
| **Write-once de NDF** | `NdfArchive::write_once` deve falhar com `NdfRecordAlreadyExists` se o id já existir. |
| **Write-once de template activo** | `TemplateRepository` deve falhar com `TemplateImmutable` em qualquer modificação de template `Active` ou `Deprecated`. |
| **Verificação de hash de conteúdo** | `AttachmentStore::store` deve calcular SHA-256 do conteúdo e rejeitar com `ContentHashMismatch` se não coincidir com `attachment.content_hash`. |
| **Limite de tamanho** | `AttachmentStore::store` deve rejeitar antes de persistir se `content.len() > MAX_ATTACHMENT_BYTES`. |
| **Reconstituição válida** | Após `repo.get(id)`, o `DocumentCustody` retornado deve passar `validate()` sem erro. |

Estas obrigações são validadas nos testes de integração de `documental-sqlite`.
Qualquer adapter alternativo deve passar o mesmo conjunto de testes.

---

## Dependências externas

```toml
[dependencies]
serde       = { workspace = true }
serde_json  = { workspace = true }
thiserror   = { workspace = true }
chrono      = { workspace = true }
rand        = { workspace = true }
core-doc-types = { path = "../core-doc-types" }
```

Zero dependências de outros bounded contexts (`core-rh`, `core-org`, `core-audit`,
`core-security`). A construção de `AuthoritySnapshot` a partir de `core_rh::UserContext`
pertence à camada de aplicação — não ao domínio.

---

## Decisões de design

### Custodiante puro, não motor de workflow

O modelo anterior tinha estados Draft → PendingApproval → Approved → Finalized.
Esse workflow foi removido: o core-documental recebe documentos já finalizados.
A razão é separação de responsabilidades — produção e aprovação pertencem a
core-ingest, miniapp-runtime e ao futuro DocumentSigningService.

### AuthoritySource em vez de AuthoritySnapshot directo

`IntakeSpec.authority: AuthoritySource` permite intake por utilizadores humanos
(`User`) e por processos batch (`System`) com o mesmo port. Antes, só era possível
intake por utilizadores com posição orgânica activa.

### EventActor com três variantes

`Operator` (string-only, sem snapshot completo) para operações de runtime.
`Authority` (snapshot completo) para actos com relevância legal.
`System` (system_id + justification) para processos autónomos.
A distinção é semântica e auditável: o log de eventos regista exactamente que tipo
de actor realizou cada acto.

### DocumentPackage em core-doc-types

`DocumentPackage` e os seus tipos auxiliares vivem em `core-doc-types` (shared kernel
neutro) para que `core-documental` e `core-exports` possam ambos depender deles sem
criar dependência circular. `core-documental` re-exporta todos os tipos de
`core-doc-types` para que os seus consumidores não precisem de dependência directa.

### verify_event_chain não recomputa hashes

A verificação estrutural (ordem, previous_hash presente/ausente, timestamps) está
no domínio para ser partilhada entre adapters. A verificação criptográfica dos valores
de hash pertence ao adapter que conhece o algoritmo — o domínio não deve assumir SHA-256.

### store_from_path como método default no port

O port `AttachmentStore` oferece `store_from_path` como implementação default para
evitar que cada caller gerencie leitura de ficheiro. Usa `std::fs` (universalmente
disponível) e verifica o tamanho pelo filesystem metadata antes de alocar o buffer.

---

## Lacunas conhecidas e TODO

### Structuralmente documentadas — a corrigir em iteração futura

**[TODO-1] Campos públicos no agregado**
`DocumentCustody` tem todos os campos `pub`. Código com `&mut doc` pode mutar
`status` ou `document_number` sem passar por `transition_to()` ou `assign_number()`.
O enforcement é por convenção, não pelo compilador. Resolução: campos privados com
getters, ou type-state pattern para estados terminais.
*Impacto real*: baixo enquanto o número de callers é pequeno e controlado.

**[TODO-2] Dois caminhos para o evento CustodyAccepted**
`DocumentCustodyService::accept_into_custody` constrói o evento com
`EventActor::Authority(snapshot)` (correcto — snapshot completo). O helper
`miniapp-runtime::create_document_created_event` constrói o mesmo tipo de evento
com `EventActor::Operator { user_id, position_id }` (snapshot incompleto).
Quem usar o runtime directamente em vez do service perde o snapshot de autoridade
no event log. Resolução: deprecar `create_document_created_event` ou fazê-lo
aceitar `AuthoritySource` e mapear para o actor correcto.

### Funcionalmente em falta — a implementar quando houver necessidade

**[TODO-3] Sem `list_by_actor` no DocumentEventLog**
Não existe método `filter_by_actor(actor_id, limit, offset)`. Para compliance
("todos os documentos acedidos pelo actor X") é necessário scan completo ou
query ad-hoc no adapter. Adicionar ao port quando a necessidade for documentada.

**[TODO-4] Sem `list_expired_documents` no DocumentCustodyRepository**
`is_retention_expired(at)` existe no agregado mas não há port para encontrar
documentos cuja retenção expirou. Um job de enforcement de retenção precisa de
`list_by_retention_expired(at, limit, offset)`.

**[TODO-5] verify_event_chain apenas estrutural**
Não recomputa nem verifica os valores de `previous_hash`. Para auditoria completa,
considerar `verify_full_chain(events, hash_fn: impl Fn(&[u8]) -> String)` no domínio.

**[TODO-6] Sem validação de formato do número de documento**
`document_number` aceita qualquer string não vazia. O formato institucional
(ex: `2026/001`, `SFVFX/2026/001`) não é validado pelo domínio.

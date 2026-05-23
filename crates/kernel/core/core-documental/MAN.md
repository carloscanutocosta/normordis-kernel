# Manual do modulo core-documental

## Objetivo

`core-documental` e o nucleo de dominio para custódia documental institucional no
Mini-Kernel RS. Cobre o ciclo de vida completo de documentos: desde a criação em
rascunho até à finalização com autoridade jurídica, arquivo e anulação. Exporta
tipos, invariantes, ports de persistência e o envelope canónico `DocumentPackage`
usado em Gate F (`core-exports`).

## Contrato publico

### Tipos principais

```rust
// Custódia
DocumentCustody        // agregado central — ciclo de vida
DocumentId             // identificador do documento
DocumentStatus         // máquina de estados
DocumentRelation       // relação entre dois documentos
RelationType           // tipo de relação (ReplyTo, References, Supersedes, Annuls, AnnexDocument)

// Templates
DocumentTemplate       // template NDT versionado e write-once após activação
TemplateId
TemplateStatus         // Draft | Active | Deprecated

// Arquivo NDF
NdfRecord              // registo de render NDF write-once
NdfRecordId

// Anexos
DocumentAttachment     // metadados de blob binário em guarda institucional
AttachmentId
AttachmentKind         // Annex | Incoming

// Autoridade
AuthorityContext       // snapshot jurídico imutável de quem finalizou

// Eventos
DocumentEvent          // evento append-only com cadeia de hashes
DocumentEventId
DocumentEventType      // Created | StatusChanged | PayloadUpdated | NumberAssigned |
                       // NdfRendered | Signed | RelationAdded | AttachmentAdded |
                       // Archived | Annulled
EventActor             // Operator (pré-finalização) | Authority (pós-finalização)

// Envelope de exportação
DocumentPackage        // envelope canónico para Gate F
TemplateRef
EngineRef
Artefact
HashResult

// Erro
DocumentalError
```

### Ports de persistência

```rust
DocumentCustodyRepository  // CRUD do agregado + relações
TemplateRepository         // versões de template write-once
NdfArchive                 // registo NDF write-once
DocumentEventLog           // log de eventos append-only
AttachmentStore            // armazenamento de blobs binários
```

### Funções utilitárias

```rust
validate_document_package(pkg) -> Result<(), DocumentalError>
verify_event_chain(events)     -> Result<(), DocumentalError>
```

## Máquina de estados

```
Draft ──→ PendingApproval ──→ Approved ──→ Finalized ──→ Archived
  │                │              │                         (terminal)
  └──→ Archived    └──→ Draft     └──→ Draft
  (terminal)                                      Finalized ──→ Annulled
                                                                (terminal)
```

Regras:
- `Draft` pode retroceder para `Draft` a partir de `PendingApproval` ou `Approved` (rejeição).
- `Archived` e `Annulled` sao estados terminais — nenhuma transição e válida a partir deles.
- A transição e validada por `DocumentStatus::can_transition_to`; a persistência e responsabilidade do adapter.

## Finalização

```rust
// Pré-condições obrigatórias:
doc.authority_context = Some(authority_snapshot); // capturado no momento do acto
doc.document_number   = Some("2026/001".into());  // atribuído exactamente uma vez
doc.status            = DocumentStatus::Approved;

let next = doc.finalize()?; // → DocumentStatus::Finalized
```

`finalize()` e atómico: valida pré-condições e devolve o próximo estado numa única
chamada. O chamador não deve compor `check_ready_to_finalize()` + `transition_to()`
separadamente — `finalize()` garante a indivisibilidade.

## Identificadores documentais

`DocumentId::new` define a forma canónica de identificador aceite pelo domínio:

- obrigatório e sem espaços no início ou no fim;
- máximo de 128 bytes;
- apenas ASCII alfanumérico, hífen (`-`), underscore (`_`) ou ponto (`.`);
- sem `..`, `.` isolado, `..` isolado, `/` ou `\`.

Esta regra existe para que o identificador seja seguro como chave técnica caso um
adapter o materialize futuramente em nomes de ficheiro ou diretórios. Mesmo quando
um caller constrói `DocumentId(...)` diretamente por compatibilidade histórica,
`DocumentCustody::validate`, `DocumentRelation::validate`, `NdfRecord::validate`,
`DocumentAttachment::validate` e `DocumentEvent::validate` devem rejeitar valores
fora desta forma antes de persistência ou uso técnico.

## Invariantes

- `document_number` e atribuído exactamente uma vez; `assign_number` rejeita segunda atribuição.
- `DocumentId` não pode conter sequências ou caracteres que permitam navegação de
  diretórios (`..`, `/`, `\`) nem caracteres fora da chave canónica.
- `AuthorityContext` congela quem (`user_id`), em que posição (`position_id`), na unidade
  (`unit_id`), com que competência (`competency_id`) e eventual delegação. E imutável após captura.
- Templates `Active` e `Deprecated` sao imutáveis — qualquer alteração cria nova versão.
- `NdfRecord` e write-once; `NdfArchive::write_once` deve rejeitar duplicados.
- `DocumentEventLog` e append-only; nenhum adapter deve fazer UPDATE ou DELETE em eventos.
- `verify_event_chain` valida estrutura da cadeia (primeiro evento sem `previous_hash`,
  ordem cronológica); nao recomputa hashes criptográficos (responsabilidade da infra).
- `AttachmentStore::store` deve verificar `sha256(content) == attachment.content_hash`
  antes de persistir.
- `validate_document_package` exige `document_id` não vazio, template e engine com id
  e versão preenchidos, e pelo menos um artefacto com kind, ref e hash não vazios.

## Decisões de design

### Autoridade jurídica exigida em DocumentTemplate.created_by e NdfRecord.rendered_by

`DocumentTemplate.created_by` e `NdfRecord.rendered_by` usam `AuthorityContext` mesmo
para operações que poderiam ser técnicas ou automatizadas. Esta decisão preserva o
rastreio completo de quem criou um template ou renderizou um NDF, exigindo que o acto
seja sempre atribuído a uma pessoa com posição e unidade orgânica identificadas.

Limitação conhecida: para renders automatizados (ex: daemons, pipelines batch) esta
exigência é difícil de cumprir sem um actor de sistema explícito. Avaliar `EventActor`
como alternativa quando houver necessidade documentada.

### Sem service layer no crate

`core-documental` exporta apenas o modelo de domínio e os ports. A orquestração
(ex: finalizar um documento em transação — load + finalize + persist + append event)
e responsabilidade do service layer da app ou de um futuro `DocumentCustodyService`.

### Fluxo multi-assinatura (informação → parecer → despacho)

`DocumentCustody.finalize()` não conhece nem enforça fases de assinatura. Essa lógica
pertence a um futuro `DocumentSigningService` que:
1. Lê o template NDT para extrair fases obrigatórias/facultativas.
2. Verifica o event log (eventos `Signed` com `data_json.fase`).
3. Só chama `doc.finalize()` se todas as fases obrigatórias estiverem satisfeitas.

Decisão pendente: formato no NDT para declarar fases de assinatura.

### Audit trail

Eventos de domínio (`DocumentEvent`) devem ser encaminhados para `core-audit` pelo
service layer ou host após persistência local via `DocumentEventLog`. O log de eventos
documental e um log de domínio, não a fonte autoritativa de auditoria institucional.

## Erros

`DocumentalError` cobre todos os erros de domínio. Os erros mais relevantes:

| Variante                      | Situação                                                   |
|-------------------------------|------------------------------------------------------------|
| `InvalidStatusTransition`     | Transição de estado não permitida pela máquina de estados  |
| `DocumentFinalized`           | Tentativa de modificar documento já finalizado             |
| `NumberAlreadyAssigned`       | Segunda atribuição de número de documento                  |
| `MissingDocumentNumber`       | Finalização sem número atribuído                           |
| `MissingAuthorityContext`     | Finalização sem autoridade jurídica capturada              |
| `TemplateImmutable`           | Modificação de template `Active` ou `Deprecated`           |
| `NdfHashMismatch`             | Hash NDF não coincide com o registado                      |
| `ContentHashMismatch`         | Hash de conteúdo não coincide com o registado              |
| `EventChainBroken`            | Cadeia de eventos com previous_hash inconsistente          |
| `InvalidIdentifier`           | Identificador fora da forma canónica segura                |
| `InvalidPackage`              | `DocumentPackage` inválido                                 |

Nota: `DocumentalError` não implementa `From<MiniError>` nem expõe códigos `MINI.*`.
E um erro de domínio puro. A conversão para `MiniError` e responsabilidade dos
adapters ou do service layer consumidor.

## Dependências

```
core-rh  — UserId (identificador de utilizador)
core-org — OrgPositionId, OrgUnitId, CompetencyId, DelegationId
```

`core-documental` não depende de `core-audit`, `core-config`, `core-exports`,
SQLite, filesystem, Tauri ou UI.

## Análise de completude

### O que está implementado

- Agregado `DocumentCustody` com máquina de estados, finalização e invariantes.
- `DocumentId` validado contra path traversal e caracteres fora da chave canónica.
- Templates versionados e write-once.
- Arquivo NDF write-once com verificação de hash.
- Anexos binários endereçados por conteúdo.
- Log de eventos com cadeia verificável.
- Relações entre documentos.
- `DocumentPackage` para exportação.
- Ports para todos os conceitos acima.
- 42 testes unitários de invariantes de domínio.

### Lacunas conhecidas

- `DocumentCustodyRepository` não tem método `list` (por tipo, estado ou data) — adapters
  não têm contrato standard para queries; listagens ficam fora do port.
- Sem `DocumentCustodyService` — a orquestração transaccional (load + mutate + persist +
  event) e responsabilidade do caller.
- `DocumentTemplate.created_by` e `NdfRecord.rendered_by` exigem `AuthorityContext`,
  o que dificulta criação automática por daemons (ver Decisões de design).
- `verify_event_chain` valida apenas estrutura — não recomputa hashes criptográficos.
- `DocumentEventLog` não tem método de contagem ou listagem por actor.
- Sem validação de formato de `document_number` (estrutura, prefixo institucional).

## ToDo

- Contrato de listagem em `DocumentCustodyRepository` (com paginação).
- `DocumentCustodyService` ou equivalente para orquestração transaccional.
- `DocumentSigningService` para fluxos multi-assinatura com fases NDT.
- Avaliar substituição de `AuthorityContext` por `EventActor` em `DocumentTemplate`
  e `NdfRecord` para suportar actores de sistema em operações automatizadas.
- Verificação criptográfica de hashes na cadeia de eventos (actualmente apenas estrutural).
- Validação de formato do número de documento.

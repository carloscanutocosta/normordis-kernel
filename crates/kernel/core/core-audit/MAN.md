# Manual técnico — core-audit

## Propósito

`core-audit` é a camada de evidência auditável do kernel NORMORDIS. Grava eventos como evidência imutável, encadeia-os num hash chain SHA-256 verificável, mantém um Registo de Controlos alinhado com COSO e mede conformidade.

Não conhece SQLite, filesystem, Tauri, logging técnico nem configuração de runtime. Recebe implementações de `AuditStore` e `ControlRegistryStore` por injecção.

---

## Arquitectura

```
ControlDefinition (catálogo COSO — 50 controlos base)
       ↓
ControlExecution (passou? falhou? foi dispensado?)
       ↓
AuditEvent (evidência: outcome + control_id + actor + target + timestamp)
       ↓
SHA-256 Chain + Ed25519 (prova verificável por terceiros)
```

### Dois serviços independentes

```
AuditService<S: AuditStore>
  ├── record_event(RecordAuditEventRequest) → AuditEvent
  ├── list_by_actor / list_by_target / list_all / list_by_date_range
  ├── verify_chain / verify_chain_since / verify_chain_from_checkpoint
  └── export_manifest / sign_and_export

ControlRegistryService<S: ControlRegistryStore>
  ├── define_control(ControlDefinition)
  ├── get_control / list_controls / list_controls_by_category
  ├── record_control_execution(...)
  ├── list_executions_by_control / list_executions_by_event
  └── conformance_summary(control_id) → ConformanceSummary
```

---

## Tipos de domínio

### AuditEvent

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `event_id` | `String` | Gerado (UUID v4) | Identificador único |
| `event_type` | `String` | Sim | Classificação semântica (`domínio.entidade.acção`) |
| `actor` | `AuditActor` | Sim | Quem actuou |
| `target` | `AuditTarget` | Sim | Sobre quê |
| `occurred_at_utc` | `DateTime<Utc>` | Gerado (`Utc::now()`) | Quando |
| `outcome` | `AuditOutcome` | Sim | Resultado da operação |
| `control_id` | `Option<String>` | Não | Controlo primário exercido |
| `details_json` | `Option<Value>` | Não | Contexto adicional em JSON |

`outcome` e `control_id` são omitidos da serialização JSON quando têm valores por omissão (`NotApplicable` e `None`) para preservar compatibilidade retroactiva do hash chain.

### AuditOutcome

| Variante | Serialização | Significado |
|---|---|---|
| `Success` | `"success"` | Operação concluída com todos os efeitos pretendidos |
| `Failure` | `"failure"` | Operação falhou; nenhum efeito permanente |
| `PartialSuccess` | `"partial_success"` | Alguns efeitos ocorreram, outros não |
| `NotApplicable` | *(omitido)* | Evento informativo sem resultado binário — valor por omissão |

### AuditActor

| Campo | Tipo | Descrição |
|---|---|---|
| `actor_id` | `String` | Identificador (obrigatório, não vazio) |
| `actor_name` | `Option<String>` | Nome legível |
| `actor_type` | `Option<String>` | Tipo (ex.: `"user"`, `"system"`, `"job"`) |

### AuditTarget

| Campo | Tipo | Descrição |
|---|---|---|
| `target_type` | `String` | Tipo da entidade (ex.: `"document"`, `"session"`) |
| `target_id` | `String` | Identificador da entidade |

### RecordAuditEventRequest

Builder para criação de eventos. Campos opcionais encadeáveis:

```rust
RecordAuditEventRequest::new(event_type, actor, target, outcome)
    .with_control_id("CTRL-AUTH-004")
    .with_details(serde_json::json!({ "campo": "valor" }))
```

---

## Control Registry

### ControlDefinition

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `control_id` | `String` | Sim | Identificador canónico (`CTRL-{CAT}-{NNN}`) |
| `name` | `String` | Sim | Nome curto e descritivo |
| `description` | `Option<String>` | Não | Descrição detalhada do propósito |
| `category` | `ControlCategory` | Sim | Categoria funcional |
| `severity` | `ControlSeverity` | Sim | Impacto potencial de uma falha |
| `owner` | `Option<String>` | Não | Responsável pelo controlo |
| `implemented_by` | `Vec<String>` | Sim | Componentes implementadores |
| `references` | `Vec<String>` | Sim | Normas referenciadas |
| `version` | `String` | Sim | Versão semver |
| `valid_from` | `DateTime<Utc>` | Sim | Início de vigência |
| `valid_to` | `Option<DateTime<Utc>>` | Não | Fim de vigência (`None` = indefinido) |
| `active` | `bool` | Sim | Operacional |

`define_control` é idempotente — cria ou substitui. Use `version` e `valid_from`/`valid_to` para gerir histórico.

### ControlCategory e prefixos canónicos

| Categoria | Prefixo | Pergunta central | Referências |
|---|---|---|---|
| `Auth` | `CTRL-AUTH-` | Quem pode fazer? | COSO, ISO 27001, eIDAS |
| `Validation` | `CTRL-VAL-` | O ato foi validado? | COSO, ISO 9001 |
| `Traceability` | `CTRL-TRACE-` | Posso provar? | COSO, ISO 15489, RGPD |
| `Documentary` | `CTRL-DOC-` | O documento é válido e controlado? | ISO 15489, ISO 9001 |
| `Integrity` | `CTRL-INT-` | Foi alterado? | ISO 27001 |
| `Privacy` | `CTRL-PRIV-` | Os dados pessoais estão protegidos? | RGPD, eIDAS |
| `Security` | `CTRL-SEC-` | Foi protegido? | ISO 27001, eIDAS |
| `Ingestion` | `CTRL-ING-` | A entrada de dados foi controlada? | ISO 27001, ISO 9001 |
| `Export` | `CTRL-EXP-` | A saída foi autorizada e registada? | RGPD, eIDAS, ISO 27001 |
| `Continuity` | `CTRL-CONT-` | Consigo recuperar? | ISO 27001, ISO 9001 |

### ControlSeverity

| Variante | Significado |
|---|---|
| `High` | Falha com impacto significativo ou violação regulatória |
| `Medium` | Falha com impacto moderado que requer resposta coordenada |
| `Low` | Falha com impacto limitado e recuperação simples |

### ControlExecution

| Campo | Tipo | Obrigatório | Descrição |
|---|---|---|---|
| `execution_id` | `String` | Gerado (UUID v4) | Identificador único |
| `control_id` | `String` | Sim | Controlo verificado |
| `event_id` | `String` | Sim | Evento de auditoria associado |
| `result` | `ControlExecutionResult` | Sim | Resultado da verificação |
| `executed_at_utc` | `DateTime<Utc>` | Gerado | Instante do registo |
| `evidence_ref` | `Option<String>` | Não | Referência externa à evidência |
| `notes` | `Option<String>` | Obrigatório se `Dispensed` | Justificação |

### ControlExecutionResult

| Variante | Significado | `notes` |
|---|---|---|
| `Passed` | Controlo verificado com sucesso | Opcional |
| `Failed` | Controlo falhou | Opcional |
| `Dispensed` | Controlo formalmente dispensado | **Obrigatório** |

`Dispensed` sem `notes` é rejeitado com `MINI.AUDIT.INVALID_CONTROL_EXECUTION`.

### ConformanceSummary e Balanced Scorecard

```rust
let summary = ctrl_svc.conformance_summary("CTRL-AUTH-004")?;

// Campos disponíveis
summary.total      // total de execuções
summary.passed     // resultado Passed
summary.failed     // resultado Failed
summary.dispensed  // resultado Dispensed (não entra no denominador)

// Taxa de conformidade: passed / (passed + failed)
// Devolve None se não houver execuções Passed ou Failed
summary.conformance_rate() // → Some(0.992)
```

---

## Catálogo base — `builtin_control_catalog()`

Devolve os 50 controlos canónicos transversais. Carregar num `ControlRegistryService`:

```rust
for control in builtin_control_catalog() {
    ctrl_svc.define_control(&control)?;
}
```

Controlos específicos de domínio de negócio (IVA, fiscalização, contraordenações, etc.) **não pertencem a este catálogo** — devem ser definidos pelos respectivos módulos de domínio.

---

## Trait AuditStore

```rust
pub trait AuditStore: Send + Sync {
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;
    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError>;

    fn list_by_actor(&self, actor_id: &str, limit: usize, offset: usize)
        -> Result<Vec<AuditEvent>, AuditError>;
    fn list_by_target(&self, target: &AuditTarget, limit: usize, offset: usize)
        -> Result<Vec<AuditEvent>, AuditError>;
    fn list_all(&self, limit: usize, offset: usize)
        -> Result<Vec<AuditEvent>, AuditError>;
    fn list_by_date_range(&self, from: DateTime<Utc>, to: DateTime<Utc>,
                          limit: usize, offset: usize)
        -> Result<Vec<AuditEvent>, AuditError>;

    fn verify_chain(&self) -> Result<AuditChainReport, AuditError>;
    fn verify_chain_since(&self, from_sequence: u64)
        -> Result<AuditChainReport, AuditError>;
    fn verify_chain_from_checkpoint(&self, checkpoint_sequence: u64,
                                    checkpoint_hash: &str)
        -> Result<AuditChainReport, AuditError>;
    fn export_manifest(&self) -> Result<AuditExportManifest, AuditError>;
}
```

`list_by_date_range`: intervalo `[from, to[` — `from` inclusivo, `to` exclusivo.

`verify_chain_since(N)`: verifica a partir da sequência N, usando o hash do evento N-1 como âncora. Útil para verificações periódicas — O(novos_eventos).

`verify_chain_from_checkpoint(seq, hash)`: verifica que o evento na posição `seq` tem exactamente `hash`. Se divergir, rejeita — prova que o prefixo não foi adulterado. Use com hash guardado externamente (ficheiro assinado, HSM).

## Trait ControlRegistryStore

```rust
pub trait ControlRegistryStore: Send + Sync {
    fn define_control(&self, definition: &ControlDefinition) -> Result<(), AuditError>;
    fn get_control(&self, control_id: &str) -> Result<Option<ControlDefinition>, AuditError>;
    fn list_controls(&self, limit: usize, offset: usize) -> Result<Vec<ControlDefinition>, AuditError>;
    fn list_controls_by_category(&self, category: ControlCategory, limit: usize, offset: usize)
        -> Result<Vec<ControlDefinition>, AuditError>;

    fn record_execution(&self, execution: &ControlExecution) -> Result<(), AuditError>;
    fn list_executions_by_control(&self, control_id: &str, limit: usize, offset: usize)
        -> Result<Vec<ControlExecution>, AuditError>;
    fn list_executions_by_event(&self, event_id: &str)
        -> Result<Vec<ControlExecution>, AuditError>;
}
```

---

## Integridade e imutabilidade

```
record_hash = SHA-256({ schema_version, sequence, previous_record_hash, event })
```

| Garantia | Mecanismo |
|---|---|
| Append-only lógico | `record` rejeita `event_id` duplicado com `DUPLICATE_EVENT` |
| Tamper-evident por registo | Cada leitura recomputa o hash; falha com `INTEGRITY_FAILED` se divergir |
| Cadeia verificável | `verify_chain` valida sequência, `previous_record_hash` e `record_hash` de cada elo |
| Verificação incremental | `verify_chain_since(N)` — O(novos_eventos) |
| Checkpoint externo | `verify_chain_from_checkpoint` prova que o prefixo não foi adulterado |
| Manifesto exportável | `export_manifest` gera hash do manifesto sobre contagem + cabeça da cadeia |
| Assinatura Ed25519 | `sign_and_export` produz assinatura verificável por terceiros sem a chave privada |

---

## Política de dados

| Regra | Detalhe |
|---|---|
| `details_json` é opcional | Pode ser `None` |
| Tamanho máximo | 16 KiB (serializado) |
| Chaves sensíveis bloqueadas | `password`, `passphrase`, `secret`, `token`, `key`, `plaintext`, `ciphertext`, `payload`, `authorization`, `cookie`, `recovery` |
| `control_id` em `AuditEvent` | Máx. 64 chars, não vazio, sem espaços nas extremidades |
| `control_id` em `ControlDefinition` | Mesmas regras; deve seguir convenção `CTRL-{CAT}-{NNN}` |
| `name` em `ControlDefinition` | Máx. 256 chars |
| `notes` em `ControlExecution` | Máx. 1024 chars; obrigatório quando `result == Dispensed` |

---

## Backends disponíveis

| Backend | Crate | Recomendado para |
|---|---|---|
| `StorageAuditStore<S>` | `core-audit` | Testes, volumes pequenos, backends abstractos |
| `StorageControlRegistryStore<S>` | `core-audit` | Testes, volumes pequenos |
| `AuditSqliteStore` | `adapter-audit-sqlite` | Produção — qualquer volume |
| `ControlRegistrySqliteStore` | `adapter-audit-sqlite` | Produção — qualquer volume |

> `StorageAuditStore` e `StorageControlRegistryStore` carregam os índices inteiros em memória. Para volumes acima de ~100K registos, use os adapters SQLite.

---

## Índices internos (StorageAuditStore)

```
{epoch_ms}.{event_id}          ← record completo com chain_link
by-id.{event_id}               ← lookup por event_id
by-actor.{sha256(actor_id)}    ← índice por actor (ordenado por tempo)
by-target.{sha256(type+id)}    ← índice por target (ordenado por tempo)
chain.head                     ← estado da cabeça (sequence + head_record_hash)
chain.events                   ← índice completo da cadeia (em sequência)
```

## Índices internos (StorageControlRegistryStore)

```
def.{control_id}               ← definição de controlo
exec.{execution_id}            ← registo de execução
by-ctrl.{sha256(control_id)}   ← índice de execuções por controlo
by-event.{sha256(event_id)}    ← índice de execuções por evento
defs.index                     ← índice global de definições
```

---

## Erros

| Código | Variante | Causa |
|---|---|---|
| `MINI.AUDIT.INVALID_EVENT_TYPE` | `InvalidEventType` | Tipo de evento vazio, com espaços ou excede 128 chars |
| `MINI.AUDIT.INVALID_ACTOR` | `InvalidActor` | `actor_id` inválido ou excede 256 chars |
| `MINI.AUDIT.INVALID_TARGET` | `InvalidTarget` | `target_type`/`target_id` inválidos ou excedem 256 chars |
| `MINI.AUDIT.INVALID_CONTROL_ID` | `InvalidControlId` | `control_id` inválido (vazio, espaços, excede 64 chars) |
| `MINI.AUDIT.INVALID_CONTROL_DEFINITION` | `InvalidControlDefinition` | Definição de controlo inválida (campos vazios, `valid_to` antes de `valid_from`, etc.) |
| `MINI.AUDIT.INVALID_CONTROL_EXECUTION` | `InvalidControlExecution` | Execução inválida (`Dispensed` sem `notes`, `notes` demasiado longos) |
| `MINI.AUDIT.DETAILS_TOO_LARGE` | `DetailsTooLarge` | `details_json` excede 16 KiB |
| `MINI.AUDIT.SENSITIVE_DETAILS` | `SensitiveDetails` | `details_json` contém chave sensível |
| `MINI.AUDIT.DUPLICATE_EVENT` | `DuplicateEvent` | `event_id` já existe |
| `MINI.AUDIT.DUPLICATE_CONTROL_EXECUTION` | `DuplicateControlExecution` | `execution_id` já existe |
| `MINI.AUDIT.INTEGRITY_FAILED` | `IntegrityFailed` | Hash do registo não coincide com o calculado |
| `MINI.AUDIT.CHAIN_VERIFICATION_FAILED` | `ChainVerificationFailed` | Cadeia inconsistente (sequência, hash ou checkpoint) |
| `MINI.AUDIT.SIGN_FAILED` | `SignFailed` | Falha ao serializar manifesto para assinatura |
| `MINI.AUDIT.SIGNATURE_VERIFICATION_FAILED` | `SignatureVerificationFailed` | Assinatura Ed25519 inválida |
| `MINI.AUDIT.STORE_FAILED` | `StoreFailed` | Erro no backend de armazenamento |
| `MINI.AUDIT.OPERATION_FAILED` | `OperationFailed` | Erro interno genérico |

---

## Separação de responsabilidades

```
core-config
  └─ define namespace e storage_profile

runtime/bootstrap
  └─ resolve storage_profile → Storage concreto
  └─ constrói AuditSqliteStore + ControlRegistrySqliteStore
  └─ injeta em AuditService + ControlRegistryService

core-audit
  └─ grava eventos, verifica cadeia, exporta manifesto
  └─ gere catálogo de controlos e regista execuções
  └─ mede conformidade
  └─ não conhece SQLite, filesystem, Tauri, logging
```

---

## O que o core-audit não faz

- **Não executa controlos** — regista que foram executados
- **Não decide compliance** — mede e expõe; quem decide é a camada superior
- **Não valida que o `control_id` existe** — é uma referência; a validação de existência é responsabilidade do chamador
- **Não loga diagnosticamente** — usa `support-logging` para isso
- **Não conhece SQLite, filesystem ou Tauri** — recebe storage por injecção

---

## Diferença para logging técnico

```
support-logging  → "O job demorou 3s e falhou com timeout"
core-audit       → "O utilizador X alterou o documento Y em 2026-05-11T10:00Z"
```

O logging é efémero e operacional. A auditoria é institucional e permanente.

---

## Restrições de dependências (guardas em testes)

`core-audit` nunca pode depender de:

- `rusqlite` / `adapter-sqlite` — isolamento da infra
- `tauri` — isolamento da UI
- `core-config` — isolamento da configuração de runtime
- `support-logging` — fronteira entre auditoria e logging

Violações são detectadas pelos testes em `manifest_tests` do crate.

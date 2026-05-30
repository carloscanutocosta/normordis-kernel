# Manual técnico — core-audit

## Propósito

`core-audit` é o núcleo de auditoria institucional do kernel normordis. Grava eventos como evidência imutável, liga-os numa cadeia de hashes verificável e permite exportar e assinar manifestos para custódia ou transmissão.

Não conhece SQLite, filesystem, Tauri, logging técnico nem configuração de runtime. Recebe um `AuditStore` por injecção.

---

## Contrato público

### Tipos de domínio

| Tipo | Papel |
|---|---|
| `AuditEvent` | Evento auditável — `event_id`, `event_type`, `actor`, `target`, `occurred_at_utc`, `details_json` |
| `AuditActor` | Quem executou — `actor_id` obrigatório; `actor_name`, `actor_type` opcionais |
| `AuditTarget` | Sobre o quê — `target_type` + `target_id` |
| `AuditService<S>` | Fachada de alto nível sobre qualquer `AuditStore` |
| `StorageAuditStore<S>` | Implementação genérica sobre `support_storage::Storage` |
| `AuditStoreConfig` | Namespace de armazenamento |
| `AuditError` | Erros tipados com código canónico `MINI.AUDIT.*` |

### Trait AuditStore

```rust
pub trait AuditStore: Send + Sync {
    // Escrita
    fn record(&self, event: &AuditEvent) -> Result<(), AuditError>;

    // Leitura por chave
    fn get(&self, event_id: &str) -> Result<Option<AuditEvent>, AuditError>;

    // Listagens
    fn list_by_actor(&self, actor_id: &str, limit: usize, offset: usize)         -> Result<Vec<AuditEvent>, AuditError>;
    fn list_by_target(&self, target: &AuditTarget, limit: usize, offset: usize)  -> Result<Vec<AuditEvent>, AuditError>;
    fn list_all(&self, limit: usize, offset: usize)                              -> Result<Vec<AuditEvent>, AuditError>;
    fn list_by_date_range(&self, from: DateTime<Utc>, to: DateTime<Utc>,
                          limit: usize, offset: usize)                           -> Result<Vec<AuditEvent>, AuditError>;

    // Verificação e exportação
    fn verify_chain(&self)                                                        -> Result<AuditChainReport, AuditError>;
    fn verify_chain_since(&self, from_sequence: u64)                             -> Result<AuditChainReport, AuditError>;
    /// Verifica o sufixo após um checkpoint externo de confiança.
    /// Rejeita se o evento na posição checkpoint_sequence não tiver exactamente
    /// checkpoint_hash — prova que o prefixo não foi adulterado.
    /// Rejeita checkpoint_sequence == 0 com ChainVerificationFailed.
    fn verify_chain_from_checkpoint(&self, checkpoint_sequence: u64,
                                    checkpoint_hash: &str)                       -> Result<AuditChainReport, AuditError>;
    fn export_manifest(&self)                                                     -> Result<AuditExportManifest, AuditError>;
}
```

### AuditService — métodos públicos

```rust
impl<S: AuditStore> AuditService<S> {
    fn record_event(event_type, actor, target, details_json) -> Result<AuditEvent>
    fn get(event_id) -> Result<Option<AuditEvent>>
    fn list_by_actor(actor_id, limit, offset) -> Result<Vec<AuditEvent>>
    fn list_by_target(target, limit, offset) -> Result<Vec<AuditEvent>>
    fn list_all(limit, offset) -> Result<Vec<AuditEvent>>
    fn list_by_date_range(from, to, limit, offset) -> Result<Vec<AuditEvent>>
    fn verify_chain() -> Result<AuditChainReport>
    fn verify_chain_since(from_sequence) -> Result<AuditChainReport>
    fn verify_chain_from_checkpoint(checkpoint_sequence, checkpoint_hash) -> Result<AuditChainReport>
    fn export_manifest() -> Result<AuditExportManifest>
    fn sign_and_export(signing_key, key_id) -> Result<SignedAuditExportManifest>
}
```

---

## Utilização

### Gravação de um evento

```rust
use core_audit::{AuditActor, AuditService, AuditStoreConfig, AuditTarget, StorageAuditStore};
use support_storage::StorageNamespace;

let config = AuditStoreConfig::new(StorageNamespace::new("audit.events")?);
let store  = StorageAuditStore::new(storage, config);
let svc    = AuditService::new(store);

let event = svc.record_event(
    "document.created",
    AuditActor::new("user-123")?,
    AuditTarget::new("document", "doc-456")?,
    Some(json!({"acção": "criação", "origem": "interface-web"})),
)?;
```

`record_event` gera UUID v4 para `event_id` e registra `occurred_at_utc = Utc::now()`.
Para testes deterministas use `AuditEvent::with_id_and_time`.

### Consulta por intervalo temporal

```rust
use chrono::{TimeZone, Utc};

let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
let to   = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();

let eventos_janeiro = svc.list_by_date_range(from, to, 100, 0)?;
```

O intervalo é `[from, to[` — `from` inclusivo, `to` exclusivo.
Com `adapter-audit-sqlite` a consulta usa `idx_audit_time` e é eficiente para qualquer volume.

### Verificação incremental da cadeia

```rust
// Primeira verificação completa — guarda o resultado
let report = svc.verify_chain()?;
let checkpoint_seq = report.checked_events as u64;

// ... novos eventos gravados ...

// Verificação incremental — apenas os novos
let report_inc = svc.verify_chain_since(checkpoint_seq + 1)?;
println!("{} novos eventos verificados", report_inc.checked_events);
```

`verify_chain_since(N)` verifica apenas eventos a partir da sequência N, calculando
o hash âncora a partir do evento N-1 já gravado. Para dezenas de milhões de eventos,
este padrão reduz o custo de verificação periódica para O(novos_eventos).

### Exportação e assinatura

```rust
// Manifesto verificável (sem chave)
let manifest = svc.export_manifest()?;

// Manifesto assinado com Ed25519
let key    = AuditSigningKey::from_bytes(raw_key_bytes);
let signed = svc.sign_and_export(&key, Some("audit-key-prod".to_string()))?;

// Verificação por terceiro (não precisa da chave privada)
verify_signed_manifest(&signed)?;
```

A chave privada é mantida em `AuditSigningKey` com `Debug` redigido e zeroização automática em drop.

---

## Integridade e imutabilidade

Cada evento gravado cria um **record de cadeia**:

```
record_hash = SHA-256({schema_version, sequence, previous_record_hash, event})
```

| Garantia | Mecanismo |
|---|---|
| Append-only lógico | `record` rejeita `event_id` duplicado com `DUPLICATE_EVENT` |
| Tamper-evident por registo | Leitura recomputa hash; falha com `INTEGRITY_FAILED` se divergir |
| Cadeia verificável | `verify_chain` valida sequência, `previous_record_hash` e `record_hash` de cada elo |
| Verificação incremental | `verify_chain_since(N)` verifica a partir da sequência N, usando âncora segura |
| Manifesto exportável | `export_manifest` gera hash do manifesto sobre contagem + cabeça da cadeia |
| Assinatura Ed25519 | `sign_and_export` produz assinatura destacada verificável por terceiros |

---

## Política de dados

| Regra | Detalhe |
|---|---|
| `details_json` é opcional | Pode ser `None` |
| Tamanho máximo | 16 KiB (serializado) |
| Chaves sensíveis bloqueadas | `password`, `passphrase`, `secret`, `token`, `key`, `plaintext`, `ciphertext`, `payload`, `authorization`, `cookie`, `recovery` |
| Payloads completos proibidos | Documentos, plantext sensível e credenciais não devem constar em auditoria |

---

## Índices internos (StorageAuditStore)

O `StorageAuditStore` mantém no namespace configurado:

```
{epoch_ms}.{event_id}          ← record completo com chain_link
by-id.{event_id}               ← lookup por event_id
by-actor.{sha256(actor_id)}    ← índice por actor (ordenado por tempo)
by-target.{sha256(type+id)}    ← índice por target (ordenado por tempo)
chain.head                     ← estado da cabeça (sequence + head_record_hash)
chain.events                   ← índice completo da cadeia (em sequência)
```

As chaves de índice usam SHA-256 do identificador para não expor dados directamente em `StorageKey`.

> **Nota de escalabilidade:** `StorageAuditStore` carrega o `chain.events` inteiro em memória para `verify_chain` e `list_by_date_range`. Para volumes acima de ~100K eventos, recomenda-se `adapter-audit-sqlite` (ver abaixo).

---

## Backends disponíveis

| Backend | Crate | Recomendado para |
|---|---|---|
| `StorageAuditStore<S>` | `core-audit` | Testes, volumes pequenos, backends abstractos |
| `AuditSqliteStore` | `adapter-audit-sqlite` | Produção — qualquer volume, verificação incremental eficiente |

---

## Separação de responsabilidades

```
core-config
  └─ define namespace e storage_profile

runtime/bootstrap
  └─ resolve storage_profile → Storage concreto
  └─ cria AuditStoreConfig
  └─ constrói AuditSqliteStore ou StorageAuditStore
  └─ injeta em AuditService

core-audit
  └─ grava, consulta, verifica cadeia, exporta manifesto
  └─ não conhece SQLite, filesystem, Tauri, logging
```

---

## Diferença para logging técnico

```
support-logging  → diagnóstico técnico operacional (pode rodar, expirar, ser filtrado)
core-audit       → evidência institucional auditável (append-only, verificável, exportável)
```

---

## Erros

Todos os erros são variantes de `AuditError` com código canónico `MINI.AUDIT.*`:

| Código | Significado |
|---|---|
| `MINI.AUDIT.INVALID_EVENT_TYPE` | Tipo de evento vazio, com espaços ou excede 128 chars |
| `MINI.AUDIT.INVALID_ACTOR` | actor_id inválido ou excede 256 chars |
| `MINI.AUDIT.INVALID_TARGET` | target_type/target_id inválidos ou excedem 256 chars |
| `MINI.AUDIT.DETAILS_TOO_LARGE` | details_json excede 16 KiB |
| `MINI.AUDIT.SENSITIVE_DETAILS` | details_json contém chave sensível |
| `MINI.AUDIT.DUPLICATE_EVENT` | event_id já existe |
| `MINI.AUDIT.INTEGRITY_FAILED` | Hash do registo não coincide com o calculado |
| `MINI.AUDIT.CHAIN_VERIFICATION_FAILED` | Cadeia inconsistente (sequência ou hash) |
| `MINI.AUDIT.SIGN_FAILED` | Falha ao serializar manifesto para assinatura |
| `MINI.AUDIT.SIGNATURE_VERIFICATION_FAILED` | Assinatura Ed25519 inválida |
| `MINI.AUDIT.STORE_FAILED` | Erro no backend de armazenamento |
| `MINI.AUDIT.OPERATION_FAILED` | Erro interno genérico |

---

## Restrições de dependências (guardas em testes)

`core-audit` nunca pode depender de:

- `rusqlite` / `adapter-sqlite` — isolamento da infra
- `tauri` — isolamento da UI
- `core-config` — isolamento da configuração de runtime
- `support-logging` — fronteira entre auditoria e logging

Violações são detectadas por testes no `manifest_tests` do crate.

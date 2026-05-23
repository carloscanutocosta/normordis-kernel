# Manual do modulo core-audit

## Objetivo

`core-audit` fornece o contrato institucional/local de auditoria do Mini-Kernel RS. O modulo grava eventos auditaveis em `support-storage`, sem conhecer o backend fisico.

## Contrato publico

Tipos principais:

```rust
AuditActor
AuditTarget
AuditEvent
AuditStore
StorageAuditStore
AuditStoreConfig
AuditService
AuditError
```

`AuditService::record_event` cria `event_id` UUID v4, define `occurred_at_utc`, valida o evento e delega a persistencia no `AuditStore`.

Consultas disponiveis nesta fase:

```rust
AuditStore::get(event_id)
AuditStore::list_by_actor(actor_id)
AuditStore::list_by_target(target)
AuditStore::verify_chain()
AuditStore::export_manifest()
sign_manifest(manifest, signing_key, key_id)
verify_signed_manifest(signed_manifest)
```

## Como usar

```rust
use core_audit::{AuditActor, AuditService, AuditStoreConfig, AuditTarget, StorageAuditStore};
use support_storage::StorageNamespace;

let config = AuditStoreConfig::new(StorageNamespace::new("audit.events")?);
let store = StorageAuditStore::new(storage, config);
let service = AuditService::new(store);

let event = service.record_event(
    "document.created",
    AuditActor::new("user-1")?,
    AuditTarget::new("document", "doc-1")?,
    None,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Para compatibilidade retroativa do namespace, `AuditStoreConfig::default()` usa `audit.events`.

## Storage

O namespace e externo e vem de `AuditStoreConfig`. `core-audit` nao conhece `core-config`, `AuditProfile`, storage profiles, SQLite, paths, filesystem, Tauri ou UI.

## Configuracao externa

```text
core-config decide o que:
  namespace = "audit.events"
  storage_profile = "audit"

runtime/bootstrap constroi como:
  resolve storage_profile
  cria support-storage::Storage concreto
  cria AuditStoreConfig
  injeta StorageAuditStore

core-audit executa:
  grava, consulta, verifica cadeia e exporta manifesto no namespace recebido
```

A conversao entre `core_config::AuditProfile` e `AuditStoreConfig` deve acontecer fora deste crate.

Cada evento e guardado como JSON protegido via `support-storage`. O core guarda tambem uma entrada tecnica de lookup por `event_id`, porque o contrato atual de `support-storage` ainda nao expoe listagem ou indices.

O store guarda tambem indices protegidos:

```text
by-id.<event_id>
by-actor.<sha256(actor_id)>
by-target.<sha256(target_type + target_id)>
```

As chaves de indice usam hash para nao expor identificadores diretamente em `StorageKey` e para cumprir a validacao de `support-storage`.

## Integridade e imutabilidade

- `record` rejeita `event_id` duplicado.
- O `StorageAuditStore` serializa cada evento dentro de um record com `schema_version` e `event_hash`.
- Cada record contem `AuditChainLink` com `sequence`, `previous_record_hash` e `record_hash`.
- `record_hash` e SHA-256 canonico do evento, sequencia e hash anterior.
- `get` e listagens verificam o hash antes de devolver eventos.
- Se o conteudo armazenado for alterado, a leitura falha com `MINI.AUDIT.INTEGRITY_FAILED`.
- `verify_chain` valida sequencia completa, hash anterior, hashes de records e cabeca da cadeia.
- `export_manifest` devolve contagem, head hash e hash do manifesto para export posterior.
- `sign_manifest` produz uma assinatura digital Ed25519 destacada para o manifesto.
- `verify_signed_manifest` valida assinatura, algoritmo, chave publica e bytes canonicos do manifesto.

## Assinatura digital

Tipos publicos:

```rust
AuditSigningKey
AuditManifestSignature
SignedAuditExportManifest
```

A assinatura usa Ed25519. A chave privada e mantida em `AuditSigningKey` com `Debug` redigido e zeroizacao em drop. O manifesto assinado inclui a chave publica em Base64 para permitir verificacao por terceiros.

O formato assinado nao inclui payloads auditaveis completos; assina o manifesto, que referencia a cabeca da cadeia hash e a contagem de eventos.

Nota: a imutabilidade atual e garantida no contrato do store e serializada no processo por mutex. Quando composto por `runtime-bootstrap` com `audit.db`, `adapter-sqlite` usa escrita condicional atomica para rejeitar overwrite no backend fisico.

## Politica de dados

- `details_json` e opcional.
- `details_json` tem limite default de 16 KiB.
- Chaves sensiveis sao rejeitadas por nome: `password`, `passphrase`, `secret`, `token`, `key`, `plaintext`, `ciphertext`, `payload`, `authorization`, `cookie`, `recovery`.
- Payloads documentais completos, plaintext sensivel e secrets nao devem ser colocados em auditoria.

## Limites atuais

- Sem export institucional.
- Sem export para ficheiro evidencial.
- Sem retencao auditavel.
- Sem importacao automatica de bases antigas criadas pelo adapter legado `support-audit-sqlite`.
- Sem dependencia de SQLite, Tauri, filesystem ou UI.

## Diferenca para logging tecnico

```text
support-logging = diagnostico tecnico operacional
core-audit = evidencia institucional/local auditavel
```

Logs tecnicos podem rodar e expirar. Eventos de auditoria nao sao apagados automaticamente nesta fase.

## ToDo

- Retencao auditavel e nao silenciosa.
- Importador controlado para bases antigas criadas pelo adapter legado `support-audit-sqlite`, se for preciso preservar historico local.

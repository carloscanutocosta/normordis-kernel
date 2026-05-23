# core-audit

`core-audit` e o core de auditoria institucional/local do Mini-Kernel RS.

Este componente regista eventos auditaveis como evidencia institucional local. O backend fisico e abstraido por `support-storage`, que guarda os valores protegidos por criptografia atraves do pipeline do kernel.

## Fronteira conceptual

```text
support-logging = diagnostico tecnico operacional
core-audit = evidencia institucional/local auditavel
```

`core-audit` nao deve usar `support-logging` como mecanismo de auditoria e nao deve aplicar retencao destrutiva automatica nesta fase.

## Relacao com auditoria legada

`core-audit` substitui o contrato legado `support-audit` e o adapter
`support-audit-sqlite`. Novos consumidores devem usar este crate e compor a
persistencia em `runtime-bootstrap`.

## Configuracao externa

O `core-audit` recebe `AuditStoreConfig` com o namespace de auditoria que deve usar:

```rust
use core_audit::{AuditStoreConfig, StorageAuditStore};
use support_storage::StorageNamespace;

let config = AuditStoreConfig::new(StorageNamespace::new("audit.events")?);
let store = StorageAuditStore::new(storage, config);
# Ok::<(), support_storage::StorageError>(())
```

O default de `AuditStoreConfig` continua a usar `audit.events` apenas para compatibilidade retroativa.

O `core-audit` nao conhece `core-config`, `AuditProfile`, storage profiles, SQLite, filesystem, Tauri ou UI. A composicao correta e:

```text
core-config
  define namespace e storage_profile

runtime/bootstrap
  resolve storage_profile para um support-storage::Storage concreto
  cria AuditStoreConfig
  injeta StorageAuditStore no core-audit

core-audit
  executa auditoria no namespace recebido
```

## Persistencia

O core usa `support-storage::Storage` no namespace recebido por `AuditStoreConfig`.

## Endurecimento atual

- Eventos sao append-only no processo: um `event_id` existente e rejeitado.
- Cada evento e armazenado num record com hash SHA-256 de integridade.
- Leituras validam o hash antes de devolver o evento.
- Eventos sao ligados por cadeia hash sequencial.
- `verify_chain` valida sequencia, hash anterior e cabeca da cadeia.
- `export_manifest` gera manifesto verificavel com contagem de eventos, head hash e hash do manifesto.
- Manifestos podem ser assinados com Ed25519 e verificados por terceiros com a chave publica embutida.
- `details_json` tem limite de tamanho.
- `details_json` rejeita chaves com nomes sensiveis como password, secret, token, key, plaintext, ciphertext e payload.
- O store mantem indices protegidos por ator e por alvo para consultas basicas sem SQL.
- Nao existe retencao destrutiva automatica.

## Base de dados audit.db

E recomendado compor este core com uma base dedicada `audit.db` no bootstrap/runtime.

O `core-audit` nao deve abrir essa base diretamente. A composicao correta e:

```text
runtime/bootstrap
  abre audit.db com adapter-sqlite
  cria RawStorage SQLite
  monta support-storage protegido
  injeta StorageAuditStore em core-audit
```

Esta separacao permite tratar a auditoria como evidencia institucional sem acoplar o core a SQLite.

## Nivel probatorio local

O modulo fornece prova local de integridade por:

- append-only logico;
- rejeicao fisica de overwrite quando usado com `audit.db`;
- hash por record;
- cadeia hash sequencial;
- manifesto de exportacao verificavel.
- assinatura digital Ed25519 destacada do manifesto.

Export para ficheiro evidencial fica para uma fase posterior; a assinatura e verificacao do manifesto ja sao headless e independentes de filesystem.

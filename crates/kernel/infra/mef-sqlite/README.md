# mef-sqlite

Adaptador SQLite para o domínio MEF (Matriz de Estrutura Funcional) — classificação funcional portuguesa com tabela temporal e suporte a diploma legal.

## Objectivo

Implementa `MefRepository` de `domain-mef` sobre SQLite com versionamento temporal (`effective_from`/`effective_to`), registo de diploma legal e migração transparente de tabelas legadas via view de compatibilidade.

## Posição arquitectural

`crates/kernel/infra` — adaptador de infraestrutura. Depende de `adapter-sqlite`, `domain-mef` e `rusqlite`.

## Responsabilidade

- Persistir entradas MEF com histórico temporal completo.
- Fechar versões anteriores e abrir novas (upsert idempotente se o conteúdo não mudou).
- Desactivar entradas abolidas por diploma.
- Resolver o caminho hierárquico de um código até à raiz.
- Migrar de tabelas legadas e criar view de compatibilidade (`platform_mef_classification_current`).

## Não-responsabilidade

- Não define a lógica de negócio MEF — essa está em `domain-mef`.
- Não valida a hierarquia da estrutura funcional (ex.: que o parent existe).
- Não é o repositório canónico de estrutura orgânica — use `org-sqlite` para isso.

## Exemplo mínimo

```rust
use mef_sqlite::MefSqliteStore;
use adapter_sqlite::SqliteRelationalConfig;

let config = SqliteRelationalConfig::read_write_create("platform.db");
let store = MefSqliteStore::open(&config)?;
let entries = store.get_current()?;
```

## Validação

```sh
cargo test -p mef-sqlite
```

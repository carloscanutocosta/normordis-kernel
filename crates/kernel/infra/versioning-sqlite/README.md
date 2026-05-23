# versioning-sqlite

Gestão de versões semânticas de módulos de aplicação com histórico de changelog, persistida em SQLite.

## Objectivo

Regista e consulta a versão SemVer de cada módulo/componente da aplicação, com suporte a bump automático (Major/Minor/Patch), changelog e garantia de versão mínima.

## Posição arquitectural

`crates/kernel/infra` — utilitário de infraestrutura autónomo. Não depende de nenhum domínio de negócio — serve qualquer módulo que precise de gerir a sua própria versão em base de dados.

## Responsabilidade

- Criar e manter o registo de versão de um módulo (`ensure_app`).
- Fazer bump de versão com changelog (`bump_version`).
- Garantir que um módulo está na versão mínima exigida (`ensure_min_version`).
- Listar o histórico de versões e changelog.

## Não-responsabilidade

- Não gere deploys nem CI/CD.
- Não compara versões entre instâncias remotas.
- Não bloqueia execução se a versão for inferior à mínima — apenas devolve erro para o caller decidir.

## Exemplo mínimo

```rust
use versioning_sqlite::VersioningSqliteStore;

let store = VersioningSqliteStore::open("app.db")?;
store.ensure_app("numerador", "1.0.0")?;
store.bump_version("numerador", BumpType::Minor, "Adicionado suporte a reset mensal")?;
```

## Validação

```sh
cargo test -p versioning-sqlite
```

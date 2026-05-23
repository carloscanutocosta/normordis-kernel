# domain-mef

Domínio MEF (Matriz de Estrutura Funcional) — classificação funcional portuguesa com versionamento temporal e referência a diploma legal.

## Objectivo

Define os tipos, port e erros para a classificação MEF: hierarquia de códigos funcionais com vigência temporal (`effective_from`/`effective_to`), identificação do diploma legal que fundamenta cada versão e operações de consulta histórica.

## Posição arquitectural

`crates/domain` — domínio de negócio puro. Sem dependências de infraestrutura. A persistência é delegada no port `MefRepository` (implementado em `mef-sqlite`).

## Responsabilidade

- Definir `MefCode` (código validado, não vazio).
- Definir `DiplomaRef` (referência ao diploma legal).
- Definir `MefEntry` (entrada com vigência temporal e auditoria).
- Definir `UpsertMefEntryRequest` (pedido de inserção/actualização validado).
- Definir o trait `MefRepository` com operações de leitura e escrita.
- Definir `MefError` com os códigos de erro do domínio.

## Não-responsabilidade

- Não persiste dados — toda a persistência é delegada no port.
- Não valida a coerência da hierarquia (ex.: que o `parent_code` existe).
- Não é um repositório de estrutura orgânica — use `domain-org` para entidades jurídicas.

## Exemplo mínimo

```rust
use domain_mef::{MefCode, MefRepository, UpsertMefEntryRequest};

let code = MefCode::new("0401")?;
let entry = repo.get_entry(&code)?;
```

## Validação

```sh
cargo test -p domain-mef
```

# support-errors

Crate headless de erros tecnicos canonicos do Mini-Kernel RS.

Este modulo fornece tipos pequenos e reutilizaveis para representar erros
tecnicos internos e a sua conversao para uma forma publica segura. Deve ser
usado por crates de `core`, `support`, `infra` e `runtime/bootstrap` quando
precisarem de expor um contrato de erro transversal e estavel.

## Responsabilidade

- Definir codigos de erro estaveis e legiveis por maquina.
- Identificar o componente emissor do erro.
- Transportar mensagem e detalhes tecnicos controlados dentro de `MiniError`.
- Converter erros internos para `PublicError` sem expor `details`.

## Nao responsabilidade

- Nao decide politicas de UI, Tauri, CLI ou API.
- Nao persiste erros.
- Nao conhece SQLite, filesystem ou adapters concretos.
- Nao substitui erros de dominio especificos quando estes tornam o dominio mais claro.

## Exemplo rapido

```rust
use support_errors::{Component, ErrorCode, MiniError};

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let err = MiniError::new(
        ErrorCode::new("MINI.SQLITE.OPEN_FAILED")?,
        Component::new("adapter-sqlite")?,
        "failed to open sqlite database",
    );

    let public = err.to_public();
    assert_eq!(public.code, "MINI.SQLITE.OPEN_FAILED");
    Ok(())
}
```

## Validacao

```text
cargo test -p support-errors
```

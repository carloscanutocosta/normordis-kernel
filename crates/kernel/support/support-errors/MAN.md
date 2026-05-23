# Manual do modulo `support-errors`

## Objetivo

`support-errors` e o contrato tecnico canonico de erros do Mini-Kernel RS.

O modulo existe para dar aos crates reutilizaveis uma forma comum de comunicar
erros tecnicos sem depender de Tauri, SQLite, filesystem, `anyhow` ou qualquer
host concreto.

## Posicao arquitetural

```text
crates/kernel/support/support-errors
```

Este crate pertence a `kernel/support` porque fornece uma primitive tecnica
transversal, headless e reutilizavel.

Consumidores previstos:

- `crates/kernel/support/support-storage`
- `crates/kernel/infra/adapter-sqlite`
- `crates/kernel/core/core-audit`
- `crates/kernel/core/core-documental`
- `runtime/bootstrap`
- hosts CLI, API ou Tauri atraves de conversoes explicitas nas fronteiras

## Contrato publico

O crate exporta:

```rust
pub use code::{ErrorCode, ErrorCodeError};
pub use component::{Component, ComponentError};
pub use error::MiniError;
pub use public::PublicError;
```

### `ErrorCode`

Codigo tecnico estavel e legivel por maquina.

Formato recomendado:

```text
MINI.<CRATE>.<SLUG>
```

Exemplos:

```text
MINI.SQLITE.OPEN_FAILED
MINI.FILES.INVALID_PATH
MINI.PDF.RENDER_FAILED
MINI.RUNTIME.BOOTSTRAP_FAILED
```

Validacao atual:

- nao pode ser vazio;
- tem de comecar por `MINI.`;
- nao pode conter whitespace.

Construtor:

```rust
ErrorCode::new("MINI.SQLITE.OPEN_FAILED")?;
```

O tipo implementa `Serialize`, `Deserialize`, `Display`, `Clone`, `Debug`,
`PartialEq`, `Eq` e `Hash`.

Importante: a desserializacao tambem passa pela validacao, para impedir que JSON
externo crie codigos invalidos.

### `Component`

Identifica o componente emissor do erro.

Exemplos:

```text
support-errors
support-storage
adapter-sqlite
support-files
support-pdf-pipeline
runtime-bootstrap
```

Validacao atual:

- nao pode ser vazio;
- nao pode conter whitespace.

Construtor:

```rust
Component::new("adapter-sqlite")?;
```

O tipo implementa `Serialize`, `Deserialize`, `Display`, `Clone`, `Debug`,
`PartialEq`, `Eq` e `Hash`.

Tal como `ErrorCode`, a desserializacao passa pela validacao.

### `MiniError`

Erro tecnico interno canonico.

Contrato:

```rust
pub struct MiniError {
    pub code: ErrorCode,
    pub component: Component,
    pub message: String,
    pub details: serde_json::Value,
}
```

Construtores:

```rust
MiniError::new(code, component, message)
MiniError::with_details(code, component, message, details)
```

Conversao publica:

```rust
let public = err.to_public();
```

`MiniError` implementa `std::error::Error` via `thiserror`.

### `PublicError`

Erro seguro para fronteiras.

Contrato:

```rust
pub struct PublicError {
    pub code: String,
    pub message: String,
}
```

Objetivo:

- devolver erro por Tauri, CLI ou API;
- preservar codigo tecnico estavel;
- nao expor `details`;
- nao expor paths, queries, dados pessoais, segredos ou causas internas.

## Como usar

### Criar erro sem detalhes

```rust
use support_errors::{Component, ErrorCode, MiniError};

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let err = MiniError::new(
        ErrorCode::new("MINI.RUNTIME.BOOTSTRAP_FAILED")?,
        Component::new("runtime-bootstrap")?,
        "failed to bootstrap runtime",
    );

    let public = err.to_public();
    assert_eq!(public.code, "MINI.RUNTIME.BOOTSTRAP_FAILED");
    Ok(())
}
```

### Criar erro com detalhes internos controlados

```rust
use serde_json::json;
use support_errors::{Component, ErrorCode, MiniError};

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let err = MiniError::with_details(
        ErrorCode::new("MINI.SQLITE.OPEN_FAILED")?,
        Component::new("adapter-sqlite")?,
        "failed to open sqlite database",
        json!({
            "operation": "open_database"
        }),
    );

    assert_eq!(err.details["operation"], "open_database");
    Ok(())
}
```

### Converter para fronteira publica

```rust
use serde_json::json;
use support_errors::{Component, ErrorCode, MiniError};

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let err = MiniError::with_details(
        ErrorCode::new("MINI.SQLITE.OPEN_FAILED")?,
        Component::new("adapter-sqlite")?,
        "failed to open sqlite database",
        json!({ "internal": "not exported" }),
    );

    let public = err.to_public();

    assert_eq!(public.code, "MINI.SQLITE.OPEN_FAILED");
    assert_eq!(public.message, "failed to open sqlite database");
    Ok(())
}
```

## Regras de uso

- `message` deve ser segura para fronteiras se for usada em `to_public()`.
- `details` deve conter apenas diagnostico tecnico controlado.
- Dados sensiveis nao devem entrar em `PublicError`.
- Crates de dominio podem manter erros locais e converter explicitamente para
  `MiniError` quando fizer sentido.
- Adapters concretos devem usar codigos estaveis e componentes explicitos.
- Novos codigos canonicos devem ser registados em `ERRORS.json`.

## Catalogo de erros

O catalogo canonico vive em `ERRORS.json`.

Esse ficheiro lista codigos reservados, ativos e depreciados do Mini-Kernel RS.
Ele e documentacao operacional, nao uma API Rust. A API continua a ser
`ErrorCode`, com validacao minima e estabilidade textual.

Sempre que um componente introduzir um novo `ErrorCode` que faca parte do seu
contrato observavel, deve atualizar o catalogo e o `MAN.md` do componente
consumidor.

## Limitacoes atuais

- Nao existe campo `source` em `MiniError`.
- Nao ha enum canonico global de codigos; os codigos sao strings validadas.
- `to_public()` copia a `message` sem mascaramento adicional.
- Nao ha severidade, categoria, retryability ou classificacao HTTP/CLI.
- Nao ha helpers especificos para Tauri, CLI, logs estruturados ou APIs.
- A validacao de formato e minima; nao valida segmentos, uppercase ou slug
  completo.

## ToDo

- Adicionar suporte a causa interna nao serializavel quando houver necessidade
  real de wrapping entre crates.
- Avaliar helpers para criar `MiniError` a partir de erros locais com
  preservacao controlada de causa.
- Definir politica opcional para mensagens publicas separadas de mensagens
  tecnicas internas.
- Avaliar categorias tecnicas como `storage`, `filesystem`, `runtime`,
  `validation` e `serialization`.
- Avaliar metadados controlados para retryability, severidade e observabilidade.
- Adicionar testes de contrato para serializacao/desserializacao de
  `MiniError` e `PublicError` quando surgirem consumidores reais.

## Validacao

Comando recomendado:

```text
cargo test -p support-errors
```

O crate deve continuar sem dependencias de Tauri, SQLite, filesystem ou
`anyhow`.

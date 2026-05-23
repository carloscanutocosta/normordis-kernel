# adapter-sqlite

Adapter SQLite de infraestrutura do Mini-Kernel RS.

Este crate fornece uma base tecnica pequena, headless e reutilizavel para abrir
bases SQLite, inicializar um schema tecnico minimo e executar batches SQL de
forma controlada.

## Responsabilidade

- Abrir uma base SQLite a partir de `SqliteConfig`.
- Criar o diretorio pai quando configurado.
- Inicializar a tabela tecnica `mini_kernel_metadata`.
- Aplicar pragmas tecnicos de seguranca operacional.
- Executar batches em transacao controlada.
- Ler e escrever metadata tecnica do Mini-Kernel.
- Executar `optimize` e checkpoint WAL de forma controlada.
- Serializar escritas concorrentes atraves de `SqliteWriteQueue`.
- Agrupar escritas pequenas em batches transacionais na writer queue.
- Aplicar retry/backoff para `SQLITE_BUSY` e `SQLITE_LOCKED` na writer queue.
- Abrir conexoes read-only por operacao atraves de `SqliteReader`.
- Implementar `support_storage::RawStorage` atraves de `SqliteRawStorage`.
- Implementar escrita condicional atomica para `put_raw_if_absent` com `INSERT OR IGNORE`.
- Expor metricas internas da writer queue para observabilidade local.
- Expor uma ponte relacional minima para migracao faseada dos adapters `*-sqlite` existentes.
- Expor uma camada de compatibilidade com nomes equivalentes a `support-sqlite`
  para tornar migracoes mecanicas.
- Converter falhas tecnicas para `MiniError` de `support-errors`.

## Nao responsabilidade

- Nao contem logica de dominio de storage.
- Nao contem logica de dominio.
- Nao depende de Tauri ou UI.
- Nao expoe `rusqlite` na API publica do storage protegido.
- A ponte relacional expoe `rusqlite` deliberadamente para codigo infra confiavel durante a migracao de adapters relacionais.
- Nao migra crates existentes.

## Exemplo rapido

```rust
use adapter_sqlite::{SqliteConfig, SqliteReader, SqliteWriteQueue};

fn example() -> Result<(), support_errors::MiniError> {
    let config = SqliteConfig::new("app.db");
    let queue = SqliteWriteQueue::start(config.clone())?;
    let reader = SqliteReader::new(config);

    queue.set_metadata("schema_version", "1")?;
    queue.execute_batch("CREATE TABLE IF NOT EXISTS demo (id TEXT PRIMARY KEY);")?;
    let schema_version = reader.get_metadata("schema_version")?;
    let metrics = queue.metrics();

    assert_eq!(schema_version, Some("1".to_owned()));
    assert!(metrics.processed_commands >= 2);
    queue.shutdown()?;
    Ok(())
}
```

## Ponte relacional

Para migrar consumidores de `support-sqlite` sem reescrever todos os adapters de
uma vez, o crate tambem expoe:

- `SqliteOptions`
- `SqliteOpenMode`
- `open_connection`
- `apply_pragmas`
- `run_migrations`
- `with_transaction`
- `sqlite_uri`
- `WriteBatch`
- `RetryPolicy`
- `WriteBatchError`
- `SqliteRelationalConfig`
- `SqliteRelationalOpenMode`
- `open_relational_connection`
- `apply_relational_pragmas`
- `run_relational_migrations`
- `with_relational_transaction`
- `RelationalWriteBatch`
- `MultiDbRelationalWriteBatch`
- `AttachedRelationalDatabase`
- `RelationalRetryPolicy`

Os nomes sem o prefixo `relational` existem apenas como ponte de compatibilidade
para trocas mecanicas de imports. Codigo novo deve preferir os nomes
`*relational*`, que tornam explicita a fronteira temporaria de migracao.

Esta API e uma ponte de infra. Nao deve receber SQL vindo diretamente da UI e
nao substitui `SqliteRawStorage` para storage protegido.

Para escritas que precisam tocar varias bases no mesmo bloco, usar
`MultiDbRelationalWriteBatch`: a API abre a base primaria, anexa as restantes
com `ATTACH DATABASE`, executa `BEGIN IMMEDIATE` e faz rollback se alguma
operacao falhar.

## Mixed-mode storage (RGPD / NIS2)

A feature `encrypted` habilita armazenamento misto: dados não sensíveis em
SQLite plain (`app.db`) e dados sensíveis em SQLCipher AES-256 (`app.secure.db`).

### Requisitos de sistema

A feature `encrypted` usa `bundled-sqlcipher-vendored-openssl`: compila SQLCipher
e OpenSSL a partir do source sem dependências de sistema adicionais.
É necessário um compilador C (MSVC no Windows, gcc/clang no Linux/macOS).

```bash
# Linux — compilador C (se não tiver)
sudo apt install build-essential
```

### Variável de ambiente

```
NORMAXIS_DB_KEY=<chave-segura>   # obrigatória nos modos Encrypted e Mixed
```

A chave nunca deve ser hardcoded. Usa `db_key_from_env()` para lê-la.

### Modos de armazenamento

| Modo | Quando usar |
|------|-------------|
| `Plain` | Mini-apps sem dados pessoais ou sensíveis |
| `Encrypted` | Toda a informação é sensível |
| `Mixed` | Mistura de dados operacionais e dados pessoais/sensíveis |

### Exemplo (Mixed)

```rust
use adapter_sqlite::{DbManager, StorageMode, db_key_from_env};
use std::path::Path;

let key = db_key_from_env()?;
let manager = DbManager::init(
    Path::new("./data"),
    StorageMode::Mixed { secure_key: key },
)?;

let plain_pool = manager.plain()?;   // → app.db
let secure_pool = manager.secure()?; // → app.secure.db
```

### Execução da migração

```bash
NORMAXIS_DB_KEY=<chave> cargo run --features encrypted --bin migrate_to_mixed -- \
  --source ./data/app.db \
  --data-dir ./data/migrated \
  --sensitive-tables "utilizadores,audit_log,dados_pessoais"
```

O script não remove a DB de origem: renomeia-a para `app_backup_<timestamp>.db`.

### Build sem encriptação (padrão)

```text
cargo build -p adapter-sqlite
```

### Build com encriptação

```text
cargo build -p adapter-sqlite --features encrypted
```

### SBOM

SQLCipher deve ser adicionado ao `SBOM-BASE` com versão, hash e licença (BSL 1.1).

## Validacao

```text
cargo test -p adapter-sqlite
```

Testes com encriptação (requer SQLCipher instalado):

```text
cargo test -p adapter-sqlite --features encrypted
```

Stress test SQLite com `support-storage` durante 5 minutos:

```text
cargo test -p adapter-sqlite --test storage_sqlite_stress_tests -- --ignored --nocapture
```

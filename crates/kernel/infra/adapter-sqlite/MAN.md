# Manual do modulo `adapter-sqlite`

## Objetivo

`adapter-sqlite` e o adapter SQLite de infraestrutura do Mini-Kernel RS.

Nesta fase, o modulo fornece a base tecnica segura para abertura de base de
dados, inicializacao de schema minimo, configuracao operacional da conexao,
execucao controlada de batches SQL, metadata tecnica do Mini-Kernel e fila de
escrita serializada com batching. Para leituras, fornece conexoes read-only por
operacao atraves de `SqliteReader`. Para operacao, expoe metricas internas da
writer queue. Implementa tambem `support_storage::RawStorage` atraves de
`SqliteRawStorage`.

## Posicao arquitetural

```text
crates/kernel/infra/adapter-sqlite
```

Este crate pertence a `kernel/infra` porque materializa uma tecnologia concreta:
SQLite.

## Contrato publico

O crate exporta:

```rust
pub use config::SqliteConfig;
pub use config::SqliteJournalMode;
pub use config::SqliteSynchronous;
pub use connection::SqliteAdapter;
pub use reader::SqliteReader;
pub use compat::{
    apply_pragmas,
    open_connection,
    run_migrations,
    sqlite_uri,
    with_transaction,
    RetryPolicy,
    SqliteOpenMode,
    SqliteOptions,
    WriteBatch,
    WriteBatchError,
};
pub use relational::{
    apply_relational_pragmas,
    open_relational_connection,
    relational_sqlite_uri,
    run_relational_migrations,
    with_relational_transaction,
    AttachedRelationalDatabase,
    MultiDbRelationalWriteBatch,
    RelationalRetryPolicy,
    RelationalWriteBatch,
    SqliteRelationalConfig,
    SqliteRelationalOpenMode,
};
pub use storage_raw::SqliteRawStorage;
pub use writer::{SqliteWriteQueue, SqliteWriteQueueMetrics};
```

### `SqliteConfig`

```rust
pub struct SqliteConfig {
    pub database_path: PathBuf,
    pub create_parent_dir: bool,
    pub busy_timeout_ms: u64,
    pub enable_foreign_keys: bool,
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronous,
    pub wal_autocheckpoint_pages: u32,
    pub write_queue_capacity: usize,
    pub write_batch_max_commands: usize,
    pub write_batch_max_delay_ms: u64,
    pub write_retry_max_attempts: u32,
    pub write_retry_base_delay_ms: u64,
    pub write_retry_max_delay_ms: u64,
    pub write_retry_jitter_ms: u64,
}
```

`SqliteConfig::new(path)` cria uma configuracao com:

- `create_parent_dir = true`;
- `busy_timeout_ms = 5000`;
- `enable_foreign_keys = true`;
- `journal_mode = SqliteJournalMode::Wal`.
- `synchronous = SqliteSynchronous::Normal`;
- `wal_autocheckpoint_pages = 1000`.
- `write_queue_capacity = 1024`.
- `write_batch_max_commands = 64`.
- `write_batch_max_delay_ms = 5`.
- `write_retry_max_attempts = 5`.
- `write_retry_base_delay_ms = 5`.
- `write_retry_max_delay_ms = 250`.
- `write_retry_jitter_ms = 5`.

`SqliteJournalMode` suporta:

```rust
pub enum SqliteJournalMode {
    Wal,
    Delete,
}
```

`SqliteSynchronous` suporta:

```rust
pub enum SqliteSynchronous {
    Normal,
    Full,
}
```

### `SqliteAdapter`

```rust
pub struct SqliteAdapter;
```

Metodos publicos:

```rust
impl SqliteAdapter {
    pub fn open(config: SqliteConfig) -> Result<Self, MiniError>;
    pub fn initialize(&self) -> Result<(), MiniError>;
    pub fn execute_batch(&self, sql: &str) -> Result<(), MiniError>;
    pub fn execute_batch_in_transaction(&self, sql: &str) -> Result<(), MiniError>;
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<(), MiniError>;
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>, MiniError>;
    pub fn optimize(&self) -> Result<(), MiniError>;
    pub fn checkpoint(&self) -> Result<(), MiniError>;
}
```

O adapter nao expoe `rusqlite::Connection` aos consumidores.

Esta regra aplica-se ao storage protegido e ao adapter tecnico principal. A
ponte relacional documentada abaixo expoe `rusqlite` deliberadamente para codigo
infra confiavel durante a migracao de adapters relacionais existentes.

### `SqliteReader`

Leitor tecnico que abre uma conexao read-only por operacao.

```rust
pub struct SqliteReader;
```

Metodos publicos:

```rust
impl SqliteReader {
    pub fn new(config: SqliteConfig) -> Self;
    pub fn read<T, F>(&self, work: F) -> Result<T, MiniError>
    where
        F: FnOnce(&rusqlite::Connection) -> Result<T, rusqlite::Error>;
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>, MiniError>;
}
```

`read()` e uma API tecnica deliberada: expoe `rusqlite::Connection` apenas
dentro da closure de leitura. A conexao e aberta em modo read-only, recebe
`PRAGMA query_only = ON`, e deve ser usada apenas por codigo infra confiavel.
Nao deve receber SQL vindo diretamente de input de utilizador.

### Ponte relacional

API de compatibilidade para migracao faseada dos consumidores de
`support-sqlite`:

#### Camada com nomes equivalentes a `support-sqlite`

Para permitir migracoes mecanicas de imports, o crate exporta:

```rust
pub enum SqliteOpenMode {
    ReadOnly,
    ReadWriteCreate,
}

pub struct SqliteOptions {
    pub path: PathBuf,
    pub mode: SqliteOpenMode,
    pub enable_foreign_keys: bool,
    pub busy_timeout_ms: u64,
}

open_connection(&options)
apply_pragmas(&conn, &options)
run_migrations(&conn, migrations)
with_transaction(&mut conn, |tx| { ... })
sqlite_uri(path, mode)
```

Tambem exporta `WriteBatch`, `RetryPolicy` e `WriteBatchError`, equivalentes ao
batch transacional com retry usado em `support-sqlite`.

Esta camada existe para reduzir risco durante migracoes faseadas. Codigo novo
deve preferir a API `SqliteRelationalConfig`/`open_relational_connection`.

#### API relacional explicita

```rust
pub enum SqliteRelationalOpenMode {
    ReadOnly,
    ReadWriteCreate,
}

pub struct SqliteRelationalConfig {
    pub database_path: PathBuf,
    pub mode: SqliteRelationalOpenMode,
    pub create_parent_dir: bool,
    pub busy_timeout_ms: u64,
    pub enable_foreign_keys: bool,
    pub journal_mode: SqliteJournalMode,
    pub synchronous: SqliteSynchronous,
    pub wal_autocheckpoint_pages: u32,
}
```

Funcoes publicas:

```rust
open_relational_connection(&config)
apply_relational_pragmas(&conn, &config)
run_relational_migrations(&conn, migrations)
with_relational_transaction(&mut conn, |tx| { ... })
relational_sqlite_uri(path, mode)
```

Batch transacional com retry:

```rust
let mut batch = RelationalWriteBatch::new(&config);
batch.push(|tx| {
    tx.execute("INSERT INTO demo (id) VALUES (?1)", ["demo"])?;
    Ok(())
});
let written = batch.commit()?;
```

Batch transacional multi-DB:

```rust
let mut batch = MultiDbRelationalWriteBatch::new(&primary_config);
batch.attach("archive", &archive_config);
batch.push(|conn| {
    conn.execute("INSERT INTO primary_table (id) VALUES ('1')", [])?;
    Ok(())
});
batch.push(|conn| {
    conn.execute("INSERT INTO archive.archive_table (id) VALUES ('1')", [])?;
    Ok(())
});
let written = batch.commit()?;
```

`MultiDbRelationalWriteBatch` usa `ATTACH DATABASE`, `BEGIN IMMEDIATE`,
`COMMIT` e `ROLLBACK`. Se qualquer operacao devolver erro antes do commit,
todas as escritas do bloco sao revertidas.

Nota operacional: isto confirma sucesso/rollback logico do bloco numa execucao
normal. Garantias de recuperacao apos crash envolvendo varios ficheiros SQLite
seguem as regras do SQLite para `ATTACH DATABASE` e o journal mode ativo.

Esta API:

- usa `MiniError` como erro publico;
- aplica pragmas alinhados com `SqliteConfig`;
- usa `_migrations` com hash FNV-1a do SQL, tal como `support-sqlite`;
- abre read-only com `PRAGMA query_only = ON`;
- suporta batch multi-DB para escritas coordenadas em varias bases SQLite;
- existe para codigo infra confiavel, nao para UI ou dominio direto;
- nao substitui `SqliteRawStorage` para dados protegidos.

### `SqliteWriteQueue`

Fila de escrita sincronizada para serializar operacoes de escrita SQLite numa
unica worker thread.

```rust
pub struct SqliteWriteQueue;
```

Metodos publicos:

```rust
impl SqliteWriteQueue {
    pub fn start(config: SqliteConfig) -> Result<Self, MiniError>;
    pub fn execute_batch(&self, sql: impl Into<String>) -> Result<(), MiniError>;
    pub fn execute_batch_in_transaction(&self, sql: impl Into<String>) -> Result<(), MiniError>;
    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) -> Result<(), MiniError>;
    pub fn shutdown(&self) -> Result<(), MiniError>;
    pub fn metrics(&self) -> SqliteWriteQueueMetrics;
}
```

A queue e clonavel para multiplos produtores. Internamente, todas as escritas
sao executadas por uma unica conexao SQLite de escrita, dentro da worker thread.

O worker junta escritas pequenas ate `write_batch_max_commands` ou ate expirar
`write_batch_max_delay_ms`. Cada comando e executado dentro de um `SAVEPOINT`
proprio numa transacao agregada. Isto permite reduzir commits sem transformar a
falha de um comando isolado em rollback dos comandos validos do mesmo batch.

Se a transacao agregada encontrar `SQLITE_BUSY` ou `SQLITE_LOCKED`, o worker
tenta novamente com backoff exponencial e jitter simples ate
`write_retry_max_attempts`. Ao esgotar o limite, os comandos do batch recebem
`MINI.SQLITE.BUSY_TIMEOUT`.

`shutdown()` envia um comando de encerramento, executa `optimize()` e
`checkpoint()` no worker, e aguarda o fim da thread. Depois do shutdown, novas
tentativas de envio falham com erro canonico.

### `SqliteWriteQueueMetrics`

Snapshot das metricas internas da writer queue.

```rust
pub struct SqliteWriteQueueMetrics {
    pub queued_commands: usize,
    pub processed_commands: u64,
    pub failed_commands: u64,
    pub committed_batches: u64,
    pub retries: u64,
    pub full_events: u64,
    pub average_wait_ms: u64,
}
```

As metricas sao locais e opcionais para observabilidade. Nao sao persistidas e
nao devem ser tratadas como contrato de auditoria.

### `SqliteRawStorage`

Backend fisico SQLite para `support-storage`.

```rust
pub struct SqliteRawStorage;
```

Metodos publicos:

```rust
impl SqliteRawStorage {
    pub fn open(config: SqliteConfig) -> Result<Self, StorageError>;
    pub fn shutdown(&self) -> Result<(), StorageError>;
    pub fn metrics(&self) -> SqliteWriteQueueMetrics;
}
```

`SqliteRawStorage` guarda apenas `support_crypto::StorageEnvelope` serializado
como JSON, nunca `StorageValue` em claro. Escritas passam pela
`SqliteWriteQueue`; leituras usam `SqliteReader`.

## Schema tecnico

`initialize()` cria apenas a tabela tecnica:

```sql
CREATE TABLE IF NOT EXISTS mini_kernel_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Nao sao criadas tabelas de storage generico nesta fase.

`SqliteRawStorage::open()` cria a tabela tecnica de envelopes protegidos:

```sql
CREATE TABLE IF NOT EXISTS mini_storage_envelopes (
    namespace TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    envelope_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (namespace, storage_key)
);
```

## Configuracao SQLite aplicada

Ao abrir a conexao, o adapter aplica:

```text
busy_timeout = SqliteConfig::busy_timeout_ms
foreign_keys = ON, quando enable_foreign_keys = true
journal_mode = WAL ou DELETE, conforme SqliteConfig::journal_mode
synchronous = NORMAL ou FULL, conforme SqliteConfig::synchronous
wal_autocheckpoint = SqliteConfig::wal_autocheckpoint_pages
temp_store = MEMORY
mmap_size = 0
```

Esta configuracao procura uma base segura para uso local desktop, com melhor
comportamento em concorrencia simples e sem expor a conexao bruta ao exterior.

`optimize()` executa `PRAGMA optimize` de forma explicita.

`checkpoint()` executa `PRAGMA wal_checkpoint(TRUNCATE)` de forma explicita para
permitir controlo operacional do WAL, por exemplo durante shutdown limpo.

## Erros

Todos os erros publicos do adapter sao `support_errors::MiniError`.

Componente:

```text
adapter-sqlite
```

Codigos usados:

```text
MINI.SQLITE.OPEN_FAILED
MINI.SQLITE.INVALID_PATH
MINI.SQLITE.CREATE_PARENT_DIR_FAILED
MINI.SQLITE.CONFIGURE_FAILED
MINI.SQLITE.INIT_FAILED
MINI.SQLITE.EXECUTE_FAILED
MINI.SQLITE.QUERY_FAILED
MINI.SQLITE.TRANSACTION_FAILED
MINI.SQLITE.METADATA_READ_FAILED
MINI.SQLITE.METADATA_WRITE_FAILED
MINI.SQLITE.INVALID_METADATA_KEY
MINI.SQLITE.OPTIMIZE_FAILED
MINI.SQLITE.CHECKPOINT_FAILED
MINI.SQLITE.BUSY_TIMEOUT
MINI.SQLITE.WRITE_QUEUE_CLOSED
MINI.SQLITE.WRITE_QUEUE_FULL
MINI.SQLITE.WRITE_QUEUE_FAILED
MINI.SQLITE.WRITE_QUEUE_SHUTDOWN_FAILED
MINI.SQLITE.LOCK_FAILED
```

As mensagens sao seguras para `MiniError::to_public()` e nao incluem paths
absolutos, SQL completo ou erros crus de `rusqlite`.

## Como usar

```rust
use adapter_sqlite::{SqliteAdapter, SqliteConfig};

fn example() -> Result<(), support_errors::MiniError> {
    let adapter = SqliteAdapter::open(SqliteConfig::new("app.db"))?;

    adapter.initialize()?;
    adapter.set_metadata("schema_version", "1")?;
    adapter.execute_batch(
        "CREATE TABLE IF NOT EXISTS demo (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        );",
    )?;
    adapter.optimize()?;

    Ok(())
}
```

### Usar fila de escrita

```rust
use adapter_sqlite::{SqliteConfig, SqliteReader, SqliteWriteQueue};

fn example() -> Result<(), support_errors::MiniError> {
    let config = SqliteConfig::new("app.db");
    let queue = SqliteWriteQueue::start(config.clone())?;
    let reader = SqliteReader::new(config);

    queue.execute_batch(
        "CREATE TABLE IF NOT EXISTS demo (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        );",
    )?;
    queue.execute_batch_in_transaction(
        "INSERT INTO demo (id, name) VALUES ('1', 'alpha');",
    )?;

    let count: i64 = reader.read(|conn| {
        conn.query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
    })?;
    let metrics = queue.metrics();

    assert_eq!(count, 1);
    assert!(metrics.processed_commands >= 2);
    queue.shutdown()?;
    Ok(())
}
```

### Escritas concorrentes

`SqliteWriteQueue` pode ser clonada e partilhada por varias threads produtoras.
As operacoes sao recebidas por um canal bounded e executadas sequencialmente no
worker de escrita.

Isto nao transforma SQLite num motor multi-writer. A queue torna explicita a
restricao de um writer efetivo de cada vez e evita que varias threads disputem a
conexao de escrita diretamente.

Em uso de producao, escritas devem passar pela `SqliteWriteQueue`. O
`SqliteAdapter` direto permanece disponivel como base tecnica e para casos
controlados, mas nao deve ser o caminho normal para escrita concorrente.

Quando a fila atinge `write_queue_capacity`, o envio falha com
`MINI.SQLITE.WRITE_QUEUE_FULL`. A politica atual e fail-fast para evitar bloqueio
indefinido de threads produtoras.

### Observabilidade

`metrics()` devolve um snapshot local com:

- comandos atualmente enfileirados;
- comandos processados;
- comandos falhados;
- batches commitados;
- retries efetuados;
- eventos de fila cheia;
- tempo medio de espera em milissegundos.

Estas metricas servem para diagnostico e operacao local. Nao substituem logs de
auditoria nem garantem contabilidade persistente.

### Estrategia de leitura

`SqliteReader` abre uma conexao read-only por operacao. Com WAL ativo, readers e
writer convivem melhor: leituras longas nao devem bloquear a fila de escrita em
condicoes normais de uso local.

Esta estrategia evita partilhar a conexao de escrita com leituras e prepara uma
evolucao futura para read pool, caso haja necessidade real.

### Executar batch em transacao

```rust
use adapter_sqlite::{SqliteAdapter, SqliteConfig};

fn example() -> Result<(), support_errors::MiniError> {
    let adapter = SqliteAdapter::open(SqliteConfig::new("app.db"))?;

    adapter.execute_batch_in_transaction(
        "CREATE TABLE IF NOT EXISTS demo (id TEXT PRIMARY KEY);
         INSERT INTO demo (id) VALUES ('1');",
    )?;

    Ok(())
}
```

### Ler metadata tecnica

```rust
use adapter_sqlite::{SqliteAdapter, SqliteConfig};

fn example() -> Result<(), support_errors::MiniError> {
    let adapter = SqliteAdapter::open(SqliteConfig::new("app.db"))?;

    adapter.set_metadata("schema_version", "1")?;
    let schema_version = adapter.get_metadata("schema_version")?;

    assert_eq!(schema_version, Some("1".to_owned()));
    Ok(())
}
```

### Operacoes de manutencao controladas

```rust
use adapter_sqlite::{SqliteAdapter, SqliteConfig};

fn example() -> Result<(), support_errors::MiniError> {
    let adapter = SqliteAdapter::open(SqliteConfig::new("app.db"))?;

    adapter.optimize()?;
    adapter.checkpoint()?;

    Ok(())
}
```

## Limitacoes atuais

- Nao fornece API de query tipada.
- `SqliteReader::read()` expoe `rusqlite::Connection` dentro de uma closure
  tecnica de leitura.
- Fora da ponte relacional/compatibilidade, nao fornece transacoes com closures
  nem acesso direto a `rusqlite::Transaction`.
- Fora de `SqliteReader` e da ponte relacional/compatibilidade, nao expoe acesso
  direto a `rusqlite::Connection`.
- Nao executa migracoes versionadas.
- O retry/backoff cobre `SQLITE_BUSY` e `SQLITE_LOCKED` na transacao agregada da
  writer queue; nao e uma politica geral para leituras ou APIs diretas.
- As metricas da queue sao em memoria e reiniciam quando o processo reinicia.
- `execute_batch()` recebe SQL do consumidor e deve ser usado apenas por codigo
  tecnico confiavel, nao com input direto de utilizador.
- `execute_batch_in_transaction()` tambem recebe SQL tecnico confiavel; o
  ganho e atomicidade, nao sanitizacao de input.

## ToDo

- Definir API tipada para queries comuns quando houver caso real.
- Avaliar read pool se conexoes por operacao forem insuficientes.
- Avaliar migracoes versionadas para adapters consumidores.
- Avaliar politicas de backup.

## Stress test de storage SQLite

Existe um teste ignorado por defeito para 5 minutos de leituras/escritas
concorrentes usando `support-storage`, `SqliteRawStorage`, writer queue e WAL:

```text
cargo test -p adapter-sqlite --test storage_sqlite_stress_tests -- --ignored --nocapture
```

## Validacao

```text
cargo test -p adapter-sqlite
```

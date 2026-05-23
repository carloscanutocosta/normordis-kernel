use crate::config::SqliteConfig;
use crate::connection::{current_timestamp_text, validate_metadata_key, SqliteAdapter};
use crate::error::{
    sqlite_error, BUSY_TIMEOUT, METADATA_WRITE_FAILED, TRANSACTION_FAILED, WRITE_QUEUE_CLOSED,
    WRITE_QUEUE_FAILED, WRITE_QUEUE_FULL, WRITE_QUEUE_SHUTDOWN_FAILED,
};
use rusqlite::{params, Error as RusqliteError, ErrorCode as RusqliteErrorCode, Transaction};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use support_errors::MiniError;

#[derive(Clone)]
pub struct SqliteWriteQueue {
    sender: SyncSender<WriteCommand>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
    metrics: Arc<QueueMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteWriteQueueMetrics {
    pub queued_commands: usize,
    pub processed_commands: u64,
    pub failed_commands: u64,
    pub committed_batches: u64,
    pub retries: u64,
    pub full_events: u64,
    pub average_wait_ms: u64,
}

#[derive(Default)]
struct QueueMetrics {
    queued_commands: AtomicUsize,
    processed_commands: AtomicU64,
    failed_commands: AtomicU64,
    committed_batches: AtomicU64,
    retries: AtomicU64,
    full_events: AtomicU64,
    total_wait_ms: AtomicU64,
}

struct BatchConfig {
    max_commands: usize,
    max_delay: Duration,
    retry: RetryConfig,
}

struct RetryConfig {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter: Duration,
}

enum WriteCommand {
    ExecuteBatch {
        sql: String,
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<(), MiniError>>,
    },
    SetMetadata {
        key: String,
        value: String,
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<(), MiniError>>,
    },
    PutStorageEnvelope {
        namespace: String,
        storage_key: String,
        envelope_json: String,
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<(), MiniError>>,
    },
    PutStorageEnvelopeIfAbsent {
        namespace: String,
        storage_key: String,
        envelope_json: String,
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<bool, MiniError>>,
    },
    DeleteStorageEnvelope {
        namespace: String,
        storage_key: String,
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<(), MiniError>>,
    },
    Shutdown {
        enqueued_at: Instant,
        reply: mpsc::Sender<Result<(), MiniError>>,
    },
}

enum BatchOperation {
    ExecuteBatch {
        sql: String,
    },
    SetMetadata {
        key: String,
        value: String,
    },
    PutStorageEnvelope {
        namespace: String,
        storage_key: String,
        envelope_json: String,
    },
    PutStorageEnvelopeIfAbsent {
        namespace: String,
        storage_key: String,
        envelope_json: String,
    },
    DeleteStorageEnvelope {
        namespace: String,
        storage_key: String,
    },
}

struct BatchItem {
    operation: BatchOperation,
    enqueued_at: Instant,
    reply: BatchReply,
}

enum BatchReply {
    Unit(mpsc::Sender<Result<(), MiniError>>),
    Bool(mpsc::Sender<Result<bool, MiniError>>),
}

enum BatchOutcome {
    Unit,
    Bool(bool),
}

impl SqliteWriteQueue {
    pub fn start(config: SqliteConfig) -> Result<Self, MiniError> {
        let capacity = config.write_queue_capacity;
        let metrics = Arc::new(QueueMetrics::default());
        let batch_config = BatchConfig {
            max_commands: config.write_batch_max_commands.max(1),
            max_delay: Duration::from_millis(config.write_batch_max_delay_ms),
            retry: RetryConfig {
                max_attempts: config.write_retry_max_attempts.max(1),
                base_delay: Duration::from_millis(config.write_retry_base_delay_ms),
                max_delay: Duration::from_millis(config.write_retry_max_delay_ms),
                jitter: Duration::from_millis(config.write_retry_jitter_ms),
            },
        };
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let (startup_sender, startup_receiver) = mpsc::channel();
        let worker_metrics = Arc::clone(&metrics);

        let worker = thread::spawn(move || {
            let adapter = match SqliteAdapter::open(config).and_then(|adapter| {
                adapter.initialize()?;
                Ok(adapter)
            }) {
                Ok(adapter) => adapter,
                Err(err) => {
                    let _ = startup_sender.send(Err(err));
                    return;
                }
            };

            let _ = startup_sender.send(Ok(()));
            worker_loop(adapter, receiver, batch_config, worker_metrics);
        });

        match startup_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                sender,
                worker: Arc::new(Mutex::new(Some(worker))),
                metrics,
            }),
            Ok(Err(err)) => {
                let _ = worker.join();
                Err(err)
            }
            Err(_) => {
                let _ = worker.join();
                Err(sqlite_error(
                    WRITE_QUEUE_FAILED,
                    "failed to start sqlite write queue",
                ))
            }
        }
    }

    pub fn execute_batch(&self, sql: impl Into<String>) -> Result<(), MiniError> {
        self.send_and_wait(|reply| WriteCommand::ExecuteBatch {
            sql: sql.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub fn execute_batch_in_transaction(&self, sql: impl Into<String>) -> Result<(), MiniError> {
        self.send_and_wait(|reply| WriteCommand::ExecuteBatch {
            sql: sql.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub fn set_metadata(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), MiniError> {
        self.send_and_wait(|reply| WriteCommand::SetMetadata {
            key: key.into(),
            value: value.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub(crate) fn put_storage_envelope(
        &self,
        namespace: impl Into<String>,
        storage_key: impl Into<String>,
        envelope_json: impl Into<String>,
    ) -> Result<(), MiniError> {
        self.send_and_wait(|reply| WriteCommand::PutStorageEnvelope {
            namespace: namespace.into(),
            storage_key: storage_key.into(),
            envelope_json: envelope_json.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub(crate) fn put_storage_envelope_if_absent(
        &self,
        namespace: impl Into<String>,
        storage_key: impl Into<String>,
        envelope_json: impl Into<String>,
    ) -> Result<bool, MiniError> {
        self.send_and_wait_bool(|reply| WriteCommand::PutStorageEnvelopeIfAbsent {
            namespace: namespace.into(),
            storage_key: storage_key.into(),
            envelope_json: envelope_json.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub(crate) fn delete_storage_envelope(
        &self,
        namespace: impl Into<String>,
        storage_key: impl Into<String>,
    ) -> Result<(), MiniError> {
        self.send_and_wait(|reply| WriteCommand::DeleteStorageEnvelope {
            namespace: namespace.into(),
            storage_key: storage_key.into(),
            enqueued_at: Instant::now(),
            reply,
        })
    }

    pub fn shutdown(&self) -> Result<(), MiniError> {
        let result = self.send_and_wait(|reply| WriteCommand::Shutdown {
            enqueued_at: Instant::now(),
            reply,
        });

        let mut worker = self.worker.lock().map_err(|_| {
            sqlite_error(
                WRITE_QUEUE_SHUTDOWN_FAILED,
                "failed to shutdown sqlite write queue",
            )
        })?;

        if let Some(worker) = worker.take() {
            worker.join().map_err(|_| {
                sqlite_error(
                    WRITE_QUEUE_SHUTDOWN_FAILED,
                    "failed to shutdown sqlite write queue",
                )
            })?;
        }

        result
    }

    pub fn metrics(&self) -> SqliteWriteQueueMetrics {
        let processed = self.metrics.processed_commands.load(Ordering::Relaxed);
        let total_wait = self.metrics.total_wait_ms.load(Ordering::Relaxed);
        SqliteWriteQueueMetrics {
            queued_commands: self.metrics.queued_commands.load(Ordering::Relaxed),
            processed_commands: processed,
            failed_commands: self.metrics.failed_commands.load(Ordering::Relaxed),
            committed_batches: self.metrics.committed_batches.load(Ordering::Relaxed),
            retries: self.metrics.retries.load(Ordering::Relaxed),
            full_events: self.metrics.full_events.load(Ordering::Relaxed),
            average_wait_ms: if processed == 0 {
                0
            } else {
                total_wait / processed
            },
        }
    }

    fn send_and_wait<F>(&self, build: F) -> Result<(), MiniError>
    where
        F: FnOnce(mpsc::Sender<Result<(), MiniError>>) -> WriteCommand,
    {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.metrics.queued_commands.fetch_add(1, Ordering::Relaxed);
        match self.sender.try_send(build(reply_sender)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
                self.metrics.full_events.fetch_add(1, Ordering::Relaxed);
                return Err(sqlite_error(WRITE_QUEUE_FULL, "sqlite write queue is full"));
            }
            Err(TrySendError::Disconnected(_)) => {
                self.metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
                return Err(sqlite_error(
                    WRITE_QUEUE_CLOSED,
                    "sqlite write queue is closed",
                ));
            }
        }
        reply_receiver.recv().map_err(|_| {
            sqlite_error(
                WRITE_QUEUE_FAILED,
                "failed to receive sqlite write queue result",
            )
        })?
    }

    fn send_and_wait_bool<F>(&self, build: F) -> Result<bool, MiniError>
    where
        F: FnOnce(mpsc::Sender<Result<bool, MiniError>>) -> WriteCommand,
    {
        let (reply_sender, reply_receiver) = mpsc::channel();
        self.metrics.queued_commands.fetch_add(1, Ordering::Relaxed);
        match self.sender.try_send(build(reply_sender)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
                self.metrics.full_events.fetch_add(1, Ordering::Relaxed);
                return Err(sqlite_error(WRITE_QUEUE_FULL, "sqlite write queue is full"));
            }
            Err(TrySendError::Disconnected(_)) => {
                self.metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
                return Err(sqlite_error(
                    WRITE_QUEUE_CLOSED,
                    "sqlite write queue is closed",
                ));
            }
        }
        reply_receiver.recv().map_err(|_| {
            sqlite_error(
                WRITE_QUEUE_FAILED,
                "failed to receive sqlite write queue result",
            )
        })?
    }
}

fn worker_loop(
    adapter: SqliteAdapter,
    receiver: Receiver<WriteCommand>,
    batch_config: BatchConfig,
    metrics: Arc<QueueMetrics>,
) {
    while let Ok(command) = receiver.recv() {
        match command {
            WriteCommand::ExecuteBatch { .. }
            | WriteCommand::SetMetadata { .. }
            | WriteCommand::PutStorageEnvelope { .. }
            | WriteCommand::PutStorageEnvelopeIfAbsent { .. }
            | WriteCommand::DeleteStorageEnvelope { .. } => {
                let mut batch = vec![command];
                let mut shutdown_reply = None;

                while batch.len() < batch_config.max_commands {
                    match receiver.recv_timeout(batch_config.max_delay) {
                        Ok(WriteCommand::Shutdown { enqueued_at, reply }) => {
                            shutdown_reply = Some((enqueued_at, reply));
                            break;
                        }
                        Ok(next) => batch.push(next),
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }

                execute_batch_group(&adapter, batch, &batch_config.retry, &metrics);

                if let Some((enqueued_at, reply)) = shutdown_reply {
                    record_shutdown(&metrics, enqueued_at);
                    let result = adapter.optimize().and_then(|_| adapter.checkpoint());
                    let _ = reply.send(result);
                    break;
                }
            }
            WriteCommand::Shutdown { enqueued_at, reply } => {
                record_shutdown(&metrics, enqueued_at);
                let result = adapter.optimize().and_then(|_| adapter.checkpoint());
                let _ = reply.send(result);
                break;
            }
        }
    }
}

fn execute_batch_group(
    adapter: &SqliteAdapter,
    commands: Vec<WriteCommand>,
    retry: &RetryConfig,
    metrics: &QueueMetrics,
) {
    let items = commands
        .into_iter()
        .filter_map(|command| match command {
            WriteCommand::ExecuteBatch {
                sql,
                enqueued_at,
                reply,
            } => Some(BatchItem {
                operation: BatchOperation::ExecuteBatch { sql },
                enqueued_at,
                reply: BatchReply::Unit(reply),
            }),
            WriteCommand::SetMetadata {
                key,
                value,
                enqueued_at,
                reply,
            } => Some(BatchItem {
                operation: BatchOperation::SetMetadata { key, value },
                enqueued_at,
                reply: BatchReply::Unit(reply),
            }),
            WriteCommand::PutStorageEnvelope {
                namespace,
                storage_key,
                envelope_json,
                enqueued_at,
                reply,
            } => Some(BatchItem {
                operation: BatchOperation::PutStorageEnvelope {
                    namespace,
                    storage_key,
                    envelope_json,
                },
                enqueued_at,
                reply: BatchReply::Unit(reply),
            }),
            WriteCommand::PutStorageEnvelopeIfAbsent {
                namespace,
                storage_key,
                envelope_json,
                enqueued_at,
                reply,
            } => Some(BatchItem {
                operation: BatchOperation::PutStorageEnvelopeIfAbsent {
                    namespace,
                    storage_key,
                    envelope_json,
                },
                enqueued_at,
                reply: BatchReply::Bool(reply),
            }),
            WriteCommand::DeleteStorageEnvelope {
                namespace,
                storage_key,
                enqueued_at,
                reply,
            } => Some(BatchItem {
                operation: BatchOperation::DeleteStorageEnvelope {
                    namespace,
                    storage_key,
                },
                enqueued_at,
                reply: BatchReply::Unit(reply),
            }),
            WriteCommand::Shutdown { .. } => None,
        })
        .collect::<Vec<_>>();

    let result = execute_batch_group_with_retry(adapter, &items, retry, metrics);

    match result {
        Ok(outcomes) => {
            for (item, outcome) in items.into_iter().zip(outcomes) {
                record_processed(metrics, item.enqueued_at, outcome.is_err());
                send_batch_reply(item.reply, outcome);
            }
        }
        Err(err) => {
            for item in items {
                record_processed(metrics, item.enqueued_at, true);
                send_batch_reply(item.reply, Err(err.clone()));
            }
        }
    }
}

fn send_batch_reply(reply: BatchReply, outcome: Result<BatchOutcome, MiniError>) {
    match (reply, outcome) {
        (BatchReply::Unit(reply), Ok(BatchOutcome::Unit)) => {
            let _ = reply.send(Ok(()));
        }
        (BatchReply::Bool(reply), Ok(BatchOutcome::Bool(value))) => {
            let _ = reply.send(Ok(value));
        }
        (BatchReply::Unit(reply), Ok(BatchOutcome::Bool(_))) => {
            let _ = reply.send(Err(sqlite_error(
                WRITE_QUEUE_FAILED,
                "sqlite write queue result type mismatch",
            )));
        }
        (BatchReply::Bool(reply), Ok(BatchOutcome::Unit)) => {
            let _ = reply.send(Err(sqlite_error(
                WRITE_QUEUE_FAILED,
                "sqlite write queue result type mismatch",
            )));
        }
        (BatchReply::Unit(reply), Err(err)) => {
            let _ = reply.send(Err(err));
        }
        (BatchReply::Bool(reply), Err(err)) => {
            let _ = reply.send(Err(err));
        }
    }
}

enum BatchRun {
    Completed(Vec<Result<BatchOutcome, MiniError>>),
    RetryableBusy,
    Failed(MiniError),
}

fn execute_batch_group_with_retry(
    adapter: &SqliteAdapter,
    items: &[BatchItem],
    retry: &RetryConfig,
    metrics: &QueueMetrics,
) -> Result<Vec<Result<BatchOutcome, MiniError>>, MiniError> {
    for attempt in 1..=retry.max_attempts {
        match execute_batch_group_once(adapter, items) {
            BatchRun::Completed(outcomes) => {
                metrics.committed_batches.fetch_add(1, Ordering::Relaxed);
                return Ok(outcomes);
            }
            BatchRun::Failed(err) => return Err(err),
            BatchRun::RetryableBusy if attempt < retry.max_attempts => {
                metrics.retries.fetch_add(1, Ordering::Relaxed);
                thread::sleep(retry_delay(retry, attempt));
            }
            BatchRun::RetryableBusy => {
                return Err(sqlite_error(
                    BUSY_TIMEOUT,
                    "sqlite writer remained busy after retry limit",
                ));
            }
        }
    }

    Err(sqlite_error(
        BUSY_TIMEOUT,
        "sqlite writer remained busy after retry limit",
    ))
}

fn execute_batch_group_once(adapter: &SqliteAdapter, items: &[BatchItem]) -> BatchRun {
    let mut conn = match adapter.lock_connection() {
        Ok(conn) => conn,
        Err(err) => return BatchRun::Failed(err),
    };
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(err) if is_retryable_busy(&err) => return BatchRun::RetryableBusy,
        Err(_) => {
            return BatchRun::Failed(sqlite_error(
                TRANSACTION_FAILED,
                "failed to start sqlite queued transaction",
            ));
        }
    };

    let mut outcomes = Vec::with_capacity(items.len());
    for item in items {
        let outcome = match &item.operation {
            BatchOperation::ExecuteBatch { sql } => execute_command_savepoint(
                &tx,
                || tx.execute_batch(sql).map(|_| 0),
                TRANSACTION_FAILED,
            )
            .map(|outcome| outcome.map(|_| BatchOutcome::Unit)),
            BatchOperation::SetMetadata { key, value } => {
                if let Err(err) = validate_metadata_key(key) {
                    Ok(Err(err))
                } else {
                    execute_command_savepoint(
                        &tx,
                        || {
                            tx.execute(
                                "INSERT INTO mini_kernel_metadata (key, value, updated_at)
                                 VALUES (?1, ?2, ?3)
                                 ON CONFLICT(key) DO UPDATE SET
                                    value = excluded.value,
                                    updated_at = excluded.updated_at",
                                params![key, value, current_timestamp_text()],
                            )
                        },
                        METADATA_WRITE_FAILED,
                    )
                    .map(|outcome| outcome.map(|_| BatchOutcome::Unit))
                }
            }
            BatchOperation::PutStorageEnvelope {
                namespace,
                storage_key,
                envelope_json,
            } => execute_command_savepoint(
                &tx,
                || {
                    tx.execute(
                        "INSERT INTO mini_storage_envelopes
                            (namespace, storage_key, envelope_json, updated_at)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(namespace, storage_key) DO UPDATE SET
                            envelope_json = excluded.envelope_json,
                            updated_at = excluded.updated_at",
                        params![
                            namespace,
                            storage_key,
                            envelope_json,
                            current_timestamp_text()
                        ],
                    )
                },
                TRANSACTION_FAILED,
            )
            .map(|outcome| outcome.map(|_| BatchOutcome::Unit)),
            BatchOperation::PutStorageEnvelopeIfAbsent {
                namespace,
                storage_key,
                envelope_json,
            } => execute_command_savepoint(
                &tx,
                || {
                    tx.execute(
                        "INSERT OR IGNORE INTO mini_storage_envelopes
                            (namespace, storage_key, envelope_json, updated_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            namespace,
                            storage_key,
                            envelope_json,
                            current_timestamp_text()
                        ],
                    )
                },
                TRANSACTION_FAILED,
            )
            .map(|outcome| outcome.map(|changed| BatchOutcome::Bool(changed > 0))),
            BatchOperation::DeleteStorageEnvelope {
                namespace,
                storage_key,
            } => execute_command_savepoint(
                &tx,
                || {
                    tx.execute(
                        "DELETE FROM mini_storage_envelopes
                         WHERE namespace = ?1 AND storage_key = ?2",
                        params![namespace, storage_key],
                    )
                },
                TRANSACTION_FAILED,
            )
            .map(|outcome| outcome.map(|_| BatchOutcome::Unit)),
        };

        match outcome {
            Ok(outcome) => outcomes.push(outcome),
            Err(BatchRun::RetryableBusy) => return BatchRun::RetryableBusy,
            Err(BatchRun::Failed(err)) => return BatchRun::Failed(err),
            Err(BatchRun::Completed(_)) => unreachable!("nested batch completion is impossible"),
        }
    }

    match tx.commit() {
        Ok(()) => BatchRun::Completed(outcomes),
        Err(err) if is_retryable_busy(&err) => BatchRun::RetryableBusy,
        Err(_) => BatchRun::Failed(sqlite_error(
            TRANSACTION_FAILED,
            "failed to commit sqlite queued transaction",
        )),
    }
}

fn record_processed(metrics: &QueueMetrics, enqueued_at: Instant, failed: bool) {
    metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
    metrics.processed_commands.fetch_add(1, Ordering::Relaxed);
    metrics
        .total_wait_ms
        .fetch_add(enqueued_at.elapsed().as_millis() as u64, Ordering::Relaxed);
    if failed {
        metrics.failed_commands.fetch_add(1, Ordering::Relaxed);
    }
}

fn record_shutdown(metrics: &QueueMetrics, enqueued_at: Instant) {
    metrics.queued_commands.fetch_sub(1, Ordering::Relaxed);
    metrics
        .total_wait_ms
        .fetch_add(enqueued_at.elapsed().as_millis() as u64, Ordering::Relaxed);
}

fn execute_command_savepoint<F>(
    tx: &Transaction<'_>,
    work: F,
    error_code: &str,
) -> Result<Result<usize, MiniError>, BatchRun>
where
    F: FnOnce() -> Result<usize, RusqliteError>,
{
    tx.execute_batch("SAVEPOINT mini_write_queue_item;")
        .map_err(|err| {
            if is_retryable_busy(&err) {
                BatchRun::RetryableBusy
            } else {
                BatchRun::Failed(sqlite_error(
                    TRANSACTION_FAILED,
                    "failed to start sqlite queue savepoint",
                ))
            }
        })?;

    match work() {
        Ok(changed) => tx
            .execute_batch("RELEASE SAVEPOINT mini_write_queue_item;")
            .map(|_| Ok(changed))
            .map_err(|err| {
                if is_retryable_busy(&err) {
                    BatchRun::RetryableBusy
                } else {
                    BatchRun::Failed(sqlite_error(
                        TRANSACTION_FAILED,
                        "failed to release sqlite queue savepoint",
                    ))
                }
            }),
        Err(err) if is_retryable_busy(&err) => {
            let _ = tx.execute_batch("ROLLBACK TO SAVEPOINT mini_write_queue_item;");
            let _ = tx.execute_batch("RELEASE SAVEPOINT mini_write_queue_item;");
            Err(BatchRun::RetryableBusy)
        }
        Err(_) => {
            let _ = tx.execute_batch("ROLLBACK TO SAVEPOINT mini_write_queue_item;");
            let _ = tx.execute_batch("RELEASE SAVEPOINT mini_write_queue_item;");
            Ok(Err(sqlite_error(
                error_code,
                match error_code {
                    METADATA_WRITE_FAILED => "failed to write sqlite metadata",
                    _ => "failed to execute sqlite queued batch",
                },
            )))
        }
    }
}

fn is_retryable_busy(err: &RusqliteError) -> bool {
    match err {
        RusqliteError::SqliteFailure(inner, _) => matches!(
            inner.code,
            RusqliteErrorCode::DatabaseBusy | RusqliteErrorCode::DatabaseLocked
        ),
        _ => false,
    }
}

fn retry_delay(retry: &RetryConfig, attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(20);
    let multiplier = 1_u64 << exponent;
    let base_ms = retry.base_delay.as_millis() as u64;
    let max_ms = retry.max_delay.as_millis() as u64;
    let jitter_ms = retry.jitter.as_millis() as u64;
    let jitter = if jitter_ms == 0 {
        0
    } else {
        attempt as u64 * 13 % (jitter_ms + 1)
    };
    let delay_ms = base_ms.saturating_mul(multiplier).saturating_add(jitter);
    Duration::from_millis(delay_ms.min(max_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn write_queue_executes_batch() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("queue.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue
            .execute_batch(
                "CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 INSERT INTO demo (id, name) VALUES (1, 'alpha');",
            )
            .unwrap();
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM demo WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "alpha");
    }

    #[test]
    fn write_queue_serializes_concurrent_writes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("concurrent.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
            .unwrap();

        let mut handles = Vec::new();
        for id in 0..32 {
            let queue = queue.clone();
            handles.push(thread::spawn(move || {
                queue
                    .execute_batch_in_transaction(format!(
                        "INSERT INTO demo (id, name) VALUES ({id}, 'item-{id}');"
                    ))
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 32);
    }

    #[test]
    fn write_queue_batches_small_writes_and_preserves_per_command_results() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("batched.db");
        let mut config = SqliteConfig::new(&db_path);
        config.write_batch_max_commands = 8;
        config.write_batch_max_delay_ms = 25;
        let queue = SqliteWriteQueue::start(config).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let ok_one = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (1);"))
        };
        let duplicate = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (1);"))
        };
        let ok_two = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (2);"))
        };

        let results = [
            ok_one.join().unwrap(),
            duplicate.join().unwrap(),
            ok_two.join().unwrap(),
        ];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 2);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn write_queue_rolls_back_transaction_batch_on_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("rollback.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let result = queue.execute_batch_in_transaction(
            "INSERT INTO demo (id) VALUES (1);
             INSERT INTO demo (id) VALUES (1);",
        );

        assert!(result.is_err());
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn write_queue_writes_metadata() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("metadata.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue.set_metadata("schema_version", "1").unwrap();
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let value: String = conn
            .query_row(
                "SELECT value FROM mini_kernel_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "1");
    }

    #[test]
    fn write_queue_reports_metrics_snapshot() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("metrics.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();
        queue
            .execute_batch("INSERT INTO demo (id) VALUES (1);")
            .unwrap();
        let _ = queue.execute_batch("INSERT INTO demo (id) VALUES (1);");

        let metrics = queue.metrics();
        queue.shutdown().unwrap();

        assert_eq!(metrics.queued_commands, 0);
        assert!(metrics.processed_commands >= 3);
        assert!(metrics.failed_commands >= 1);
        assert!(metrics.committed_batches >= 2);
    }

    #[test]
    fn write_queue_returns_error_when_queue_is_full() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("full.db");
        let mut config = SqliteConfig::new(&db_path);
        config.write_queue_capacity = 1;
        config.busy_timeout_ms = 0;
        config.write_retry_max_attempts = 20;
        config.write_retry_base_delay_ms = 10;
        config.write_retry_max_delay_ms = 25;
        config.write_retry_jitter_ms = 0;
        let queue = SqliteWriteQueue::start(config).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let blocker = Connection::open(&db_path).unwrap();
        blocker.execute_batch("BEGIN EXCLUSIVE;").unwrap();

        let first = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (1);"))
        };
        thread::sleep(Duration::from_millis(25));

        let second = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (2);"))
        };
        thread::sleep(Duration::from_millis(25));

        let err = queue
            .execute_batch("INSERT INTO demo (id) VALUES (3);")
            .unwrap_err();

        blocker.execute_batch("ROLLBACK;").unwrap();
        assert!(first.join().unwrap().is_ok());
        assert!(second.join().unwrap().is_ok());

        let metrics = queue.metrics();
        queue.shutdown().unwrap();

        assert_eq!(err.code.as_str(), crate::error::WRITE_QUEUE_FULL);
        assert!(metrics.full_events >= 1);
    }

    #[test]
    fn write_queue_shutdown_flushes_queued_writes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("shutdown-flush.db");
        let queue = SqliteWriteQueue::start(SqliteConfig::new(&db_path)).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let mut handles = Vec::new();
        for id in 0..16 {
            let queue = queue.clone();
            handles.push(thread::spawn(move || {
                queue.execute_batch(format!("INSERT INTO demo (id) VALUES ({id});"))
            }));
        }

        for handle in handles {
            handle.join().unwrap().unwrap();
        }
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 16);
    }

    #[test]
    fn write_queue_stress_handles_one_hundred_writes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("stress.db");
        let mut config = SqliteConfig::new(&db_path);
        config.write_batch_max_commands = 16;
        config.write_batch_max_delay_ms = 10;
        let queue = SqliteWriteQueue::start(config).unwrap();

        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let mut handles = Vec::new();
        for id in 0..100 {
            let queue = queue.clone();
            handles.push(thread::spawn(move || {
                queue.execute_batch(format!("INSERT INTO demo (id) VALUES ({id});"))
            }));
        }

        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        let metrics = queue.metrics();
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 100);
        assert!(metrics.processed_commands >= 101);
        assert_eq!(metrics.failed_commands, 0);
    }

    #[test]
    fn write_queue_retries_busy_writer_until_lock_is_released() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("retry.db");
        let mut config = SqliteConfig::new(&db_path);
        config.busy_timeout_ms = 0;
        config.write_retry_max_attempts = 10;
        config.write_retry_base_delay_ms = 10;
        config.write_retry_max_delay_ms = 50;
        config.write_retry_jitter_ms = 0;

        let queue = SqliteWriteQueue::start(config).unwrap();
        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let blocker = Connection::open(&db_path).unwrap();
        blocker.execute_batch("BEGIN EXCLUSIVE;").unwrap();

        let writer = {
            let queue = queue.clone();
            thread::spawn(move || queue.execute_batch("INSERT INTO demo (id) VALUES (1);"))
        };

        thread::sleep(Duration::from_millis(80));
        blocker.execute_batch("ROLLBACK;").unwrap();

        assert!(writer.join().unwrap().is_ok());
        queue.shutdown().unwrap();

        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM demo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn write_queue_returns_busy_timeout_after_retry_limit() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("busy-timeout.db");
        let mut config = SqliteConfig::new(&db_path);
        config.busy_timeout_ms = 0;
        config.write_retry_max_attempts = 2;
        config.write_retry_base_delay_ms = 5;
        config.write_retry_max_delay_ms = 5;
        config.write_retry_jitter_ms = 0;

        let queue = SqliteWriteQueue::start(config).unwrap();
        queue
            .execute_batch("CREATE TABLE demo (id INTEGER PRIMARY KEY);")
            .unwrap();

        let blocker = Connection::open(&db_path).unwrap();
        blocker.execute_batch("BEGIN EXCLUSIVE;").unwrap();

        let err = queue
            .execute_batch("INSERT INTO demo (id) VALUES (1);")
            .unwrap_err();
        blocker.execute_batch("ROLLBACK;").unwrap();
        queue.shutdown().unwrap();

        assert_eq!(err.code.as_str(), crate::error::BUSY_TIMEOUT);
    }
}

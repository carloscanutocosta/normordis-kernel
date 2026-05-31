//! Stress test de throughput de escrita com SQLCipher (5 minutos).
//!
//! Corre duas fases consecutivas e compara:
//!   - Fase 1: plain SQLite via SqliteWriteQueue  (baseline)
//!   - Fase 2: SQLCipher via DbManager pool       (encrypted)
//!
//! Execução:
//!   cargo test -p adapter-sqlite --features encrypted \
//!     --test encrypted_write_stress_tests -- --ignored --nocapture

#![cfg(feature = "encrypted")]

use adapter_sqlite::{DbManager, SqliteConfig, SqliteWriteQueue, StorageMode};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const PHASE_DURATION: Duration = Duration::from_secs(5 * 60);
const WRITER_THREADS: usize = 4;
const REPORT_INTERVAL: Duration = Duration::from_secs(30);
const PAYLOAD_BYTES: &str = r#"{"writer":0,"revision":0,"data":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#;

// ---------------------------------------------------------------------------
// Structs de resultado
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct PhaseResult {
    label: &'static str,
    duration: Duration,
    total_writes: u64,
    total_errors: u64,
    writes_per_sec: f64,
    writes_per_min: f64,
    peak_writes_per_sec: f64,
}

impl PhaseResult {
    fn print(&self) {
        eprintln!();
        eprintln!("══════════════════════════════════════════════════════");
        eprintln!("  {}", self.label);
        eprintln!("══════════════════════════════════════════════════════");
        eprintln!("  Duração real     : {:.1}s", self.duration.as_secs_f64());
        eprintln!("  Total escritas   : {}", self.total_writes);
        eprintln!("  Erros            : {}", self.total_errors);
        eprintln!("  Média writes/s   : {:.0}", self.writes_per_sec);
        eprintln!("  Média writes/min : {:.0}", self.writes_per_min);
        eprintln!("  Pico  writes/s   : {:.0}", self.peak_writes_per_sec);
        eprintln!("══════════════════════════════════════════════════════");
    }
}

// ---------------------------------------------------------------------------
// Teste principal
// ---------------------------------------------------------------------------

#[test]
#[ignore = "stress test de 5 min com SQLCipher; correr com cargo test -p adapter-sqlite --features encrypted --test encrypted_write_stress_tests -- --ignored --nocapture"]
fn compare_plain_vs_encrypted_write_throughput() {
    eprintln!(
        "\n>>> FASE 1: plain SQLite via SqliteWriteQueue ({} writers, {}s)",
        WRITER_THREADS,
        PHASE_DURATION.as_secs()
    );
    let plain = run_plain_phase();
    plain.print();

    eprintln!(
        "\n>>> FASE 2: SQLCipher via DbManager pool ({} writers, {}s)",
        WRITER_THREADS,
        PHASE_DURATION.as_secs()
    );
    let encrypted = run_encrypted_phase();
    encrypted.print();

    // Sumário comparativo
    let overhead_pct = if encrypted.writes_per_sec > 0.0 {
        ((plain.writes_per_sec - encrypted.writes_per_sec) / plain.writes_per_sec) * 100.0
    } else {
        0.0
    };

    eprintln!();
    eprintln!("╔═══════════════════════════════════════════════════════╗");
    eprintln!("║  COMPARATIVO  plain vs encrypted                      ║");
    eprintln!("╠═══════════════════════════════════════════════════════╣");
    eprintln!(
        "║  plain     {:<10.0} writes/s  {:<10.0} writes/min ║",
        plain.writes_per_sec, plain.writes_per_min
    );
    eprintln!(
        "║  encrypted {:<10.0} writes/s  {:<10.0} writes/min ║",
        encrypted.writes_per_sec, encrypted.writes_per_min
    );
    eprintln!(
        "║  overhead de encriptação: {:.1}%                      ║",
        overhead_pct
    );
    eprintln!("╚═══════════════════════════════════════════════════════╝");
    eprintln!();

    assert_eq!(plain.total_errors, 0, "fase plain registou erros");
    assert_eq!(encrypted.total_errors, 0, "fase encrypted registou erros");
    assert!(plain.total_writes > 0, "fase plain não produziu escritas");
    assert!(
        encrypted.total_writes > 0,
        "fase encrypted não produziu escritas"
    );
}

// ---------------------------------------------------------------------------
// Fase 1: plain via SqliteWriteQueue
// ---------------------------------------------------------------------------

fn run_plain_phase() -> PhaseResult {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("plain-stress.db");
    let mut config = SqliteConfig::new(&db_path);
    config.write_queue_capacity = 8_192;
    config.write_batch_max_commands = 128;
    config.write_batch_max_delay_ms = 2;

    let queue = Arc::new(SqliteWriteQueue::start(config).unwrap());

    queue
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS stress_writes (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                writer   INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                payload  TEXT    NOT NULL
            );",
        )
        .unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(WRITER_THREADS + 1));
    let mut handles = Vec::new();

    for writer_id in 0..WRITER_THREADS {
        let queue = Arc::clone(&queue);
        let stop = Arc::clone(&stop);
        let writes = Arc::clone(&writes);
        let errors = Arc::clone(&errors);
        let barrier = Arc::clone(&barrier);

        handles.push(thread::spawn(move || {
            let mut revision = 0u64;
            barrier.wait();
            while !stop.load(Ordering::Relaxed) {
                let sql = format!(
                    "INSERT INTO stress_writes (writer, revision, payload) VALUES ({writer_id}, {revision}, '{PAYLOAD_BYTES}');"
                );
                match queue.execute_batch(&sql) {
                    Ok(()) => {
                        writes.fetch_add(1, Ordering::Relaxed);
                        revision = revision.wrapping_add(1);
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    barrier.wait();
    let result = collect_phase_results("plain SQLite — SqliteWriteQueue", &stop, &writes, &errors);

    for handle in handles {
        handle.join().unwrap();
    }
    let q = Arc::try_unwrap(queue).ok().unwrap();
    q.shutdown().unwrap();

    result
}

// ---------------------------------------------------------------------------
// Fase 2: encrypted via DbManager pool
// ---------------------------------------------------------------------------

fn run_encrypted_phase() -> PhaseResult {
    let dir = tempdir().unwrap();
    let manager = Arc::new(
        DbManager::init(
            dir.path(),
            StorageMode::Encrypted {
                key: "stress-test-key-dev-32bytes!!!!!".to_owned(),
            },
        )
        .unwrap(),
    );

    manager
        .execute_secure_batch(
            "CREATE TABLE IF NOT EXISTS stress_writes (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                writer   INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                payload  TEXT    NOT NULL
            );",
        )
        .unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(WRITER_THREADS + 1));
    let mut handles = Vec::new();

    for writer_id in 0..WRITER_THREADS {
        let manager = Arc::clone(&manager);
        let stop = Arc::clone(&stop);
        let writes = Arc::clone(&writes);
        let errors = Arc::clone(&errors);
        let barrier = Arc::clone(&barrier);

        handles.push(thread::spawn(move || {
            let mut revision = 0u64;
            barrier.wait();
            while !stop.load(Ordering::Relaxed) {
                let sql = format!(
                    "INSERT INTO stress_writes (writer, revision, payload) VALUES ({writer_id}, {revision}, '{PAYLOAD_BYTES}');"
                );
                match manager.execute_secure_batch(&sql) {
                    Ok(()) => {
                        writes.fetch_add(1, Ordering::Relaxed);
                        revision = revision.wrapping_add(1);
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    barrier.wait();
    let result = collect_phase_results(
        "encrypted SQLCipher — DbManager pool",
        &stop,
        &writes,
        &errors,
    );

    for handle in handles {
        handle.join().unwrap();
    }

    result
}

// ---------------------------------------------------------------------------
// Loop de monitorização e recolha de resultados
// ---------------------------------------------------------------------------

fn collect_phase_results(
    label: &'static str,
    stop: &Arc<AtomicBool>,
    writes: &Arc<AtomicU64>,
    errors: &Arc<AtomicU64>,
) -> PhaseResult {
    let started_at = Instant::now();
    let mut last_writes = 0u64;
    let mut peak_per_sec = 0.0f64;
    let mut next_report = REPORT_INTERVAL;

    while started_at.elapsed() < PHASE_DURATION {
        thread::sleep(Duration::from_secs(1));
        let elapsed = started_at.elapsed();
        let current_writes = writes.load(Ordering::Relaxed);
        let delta = current_writes - last_writes;
        last_writes = current_writes;
        if delta as f64 > peak_per_sec {
            peak_per_sec = delta as f64;
        }
        if elapsed >= next_report {
            eprintln!(
                "  [{label}] {:.0}s — writes={} erros={} ({:.0}/s)",
                elapsed.as_secs_f64(),
                current_writes,
                errors.load(Ordering::Relaxed),
                current_writes as f64 / elapsed.as_secs_f64(),
            );
            next_report += REPORT_INTERVAL;
        }
    }

    stop.store(true, Ordering::Relaxed);

    let duration = started_at.elapsed();
    let total_writes = writes.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let writes_per_sec = total_writes as f64 / duration.as_secs_f64();

    PhaseResult {
        label,
        duration,
        total_writes,
        total_errors,
        writes_per_sec,
        writes_per_min: writes_per_sec * 60.0,
        peak_writes_per_sec: peak_per_sec,
    }
}

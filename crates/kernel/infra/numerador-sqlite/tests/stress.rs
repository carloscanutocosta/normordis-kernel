//! Stress test: NNS concurrent writes + reads over SQLite (WAL mode).
//!
//! Prova que o NNS garante:
//!   1. Unicidade — dois threads nunca obtêm o mesmo sequence_value.
//!   2. Sem perdas — COUNT(DB) == atribuições reportadas pelos writers.
//!   3. Leituras concorrentes funcionam sem erros enquanto há escrita.
//!
//! Executar:
//!   cargo test -p domain-numerador-sqlite --test stress \
//!       -- stress_nns_concurrent_writes_and_reads --ignored --nocapture
//!
//! Duração: 300 s (5 min). Alterar com variável de ambiente:
//!   STRESS_DURATION_SECS=30 cargo test ... --ignored --nocapture

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::{NaiveDate, Utc};
use domain_numerador::{
    ActorRef, AssignNumberRequest, AssignmentFilter, FormatPart, NumberFormat, NumberingKind,
    NumberingSequence, NumberingSequenceRepository, NumberingStore, ResetPolicy, TargetRef,
};
use numerador_sqlite::NumeradorDb;
use rusqlite::Connection;

// ─── Configuração ────────────────────────────────────────────────────────────

const WRITER_THREADS: usize = 4;
const READER_THREADS: usize = 2;
const DEFAULT_DURATION_SECS: u64 = 300;
const SEQ_ID: &str = "stress-seq";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open_wal(path: &std::path::Path) -> NumeradorDb {
    let conn = Connection::open(path).expect("abrir ligação");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=10000;",
    )
    .expect("configurar WAL");
    NumeradorDb::from_connection(conn).expect("inicializar NumeradorDb")
}

fn test_sequence() -> NumberingSequence {
    NumberingSequence {
        sequence_id: SEQ_ID.into(),
        kind: NumberingKind::Document,
        document_type: Some("stress_doc".into()),
        procedure_type: None,
        entity_id: "stress-entity".into(),
        org_unit_id: None,
        padding: 8,
        reset_policy: ResetPolicy::Never,
        format: NumberFormat {
            separator: String::new(),
            parts: vec![FormatPart::Literal("STR".into()), FormatPart::Sequence],
        },
        valid_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        valid_to: None,
    }
}

fn assign_request(writer_id: usize, seq: u64) -> AssignNumberRequest {
    AssignNumberRequest {
        kind: NumberingKind::Document,
        target: TargetRef {
            id: format!("w{writer_id}-{seq}"),
            target_type: "document".into(),
        },
        document_type: Some("stress_doc".into()),
        procedure_type: None,
        entity_id: "stress-entity".into(),
        org_unit_id: None,
        actor: ActorRef {
            id: format!("writer-{writer_id}"),
            name: None,
        },
        requested_at: None,
        correlation_id: None,
        metadata: Default::default(),
    }
}

// ─── Teste ───────────────────────────────────────────────────────────────────

#[test]
#[ignore]
fn stress_nns_concurrent_writes_and_reads() {
    let duration_secs: u64 = std::env::var("STRESS_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_DURATION_SECS);

    let dir = tempfile::tempdir().expect("criar directório temporário");
    let db_path = dir.path().join("stress.db");

    // Inicializar DB e registar sequência
    {
        let db = open_wal(&db_path);
        db.upsert(&test_sequence()).expect("upsert sequência");
    }

    eprintln!(
        "\n=== NNS Stress Test ===  [{WRITER_THREADS} writers | {READER_THREADS} readers | {duration_secs}s]"
    );

    let stop = Arc::new(AtomicBool::new(false));
    let total_assigned = Arc::new(AtomicU64::new(0));
    let total_read_ops = Arc::new(AtomicU64::new(0));
    let total_write_errors = Arc::new(AtomicU64::new(0));
    let total_read_errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut handles = Vec::new();

    // ── Writer threads ────────────────────────────────────────────────────────

    for writer_id in 0..WRITER_THREADS {
        let stop = Arc::clone(&stop);
        let total_assigned = Arc::clone(&total_assigned);
        let total_write_errors = Arc::clone(&total_write_errors);
        let path = db_path.clone();

        handles.push(thread::spawn(move || {
            let mut db = open_wal(&path);
            let mut local_seq = 0u64;

            while !stop.load(Ordering::Relaxed) {
                local_seq += 1;
                let req = assign_request(writer_id, local_seq);
                let ref_id = format!("ref-w{writer_id}-{local_seq}");
                match db.assign(&req, Utc::now(), &ref_id) {
                    Ok(_) => {
                        total_assigned.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        total_write_errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[writer-{writer_id}] erro: {e}");
                    }
                }
            }
        }));
    }

    // ── Reader threads ────────────────────────────────────────────────────────

    for reader_id in 0..READER_THREADS {
        let stop = Arc::clone(&stop);
        let total_read_ops = Arc::clone(&total_read_ops);
        let total_read_errors = Arc::clone(&total_read_errors);
        let path = db_path.clone();

        handles.push(thread::spawn(move || {
            let db = open_wal(&path);
            let filter = AssignmentFilter {
                sequence_id: Some(SEQ_ID.into()),
                ..Default::default()
            };
            let mut tick = 0u64;

            while !stop.load(Ordering::Relaxed) {
                tick += 1;
                match db.count_assignments(&filter) {
                    Ok(_) => {
                        total_read_ops.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        total_read_errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[reader-{reader_id}] count erro: {e}");
                    }
                }
                // list a cada 20 ticks
                if tick % 20 == 0 {
                    match db.list_assignments(&filter, 100) {
                        Ok(_) => {
                            total_read_ops.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            total_read_errors.fetch_add(1, Ordering::Relaxed);
                            eprintln!("[reader-{reader_id}] list erro: {e}");
                        }
                    }
                }
            }
        }));
    }

    // ── Progress reporter ─────────────────────────────────────────────────────

    {
        let stop = Arc::clone(&stop);
        let assigned = Arc::clone(&total_assigned);
        let reads = Arc::clone(&total_read_ops);
        let write_errors = Arc::clone(&total_write_errors);
        handles.push(thread::spawn(move || {
            let mut last = 0u64;
            let mut t = 0u64;
            while !stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(10));
                t += 10;
                let cur = assigned.load(Ordering::Relaxed);
                let delta = cur - last;
                let werr = write_errors.load(Ordering::Relaxed);
                let rops = reads.load(Ordering::Relaxed);
                eprintln!(
                    "  [{t:>3}s] total={cur:>8}  +{delta:>6}/10s  read_ops={rops:>8}  write_err={werr}"
                );
                last = cur;
            }
        }));
    }

    // ── Esperar duração configurada ───────────────────────────────────────────

    thread::sleep(Duration::from_secs(duration_secs));
    stop.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().expect("thread entrou em pânico");
    }

    let elapsed = start.elapsed();
    let assigned = total_assigned.load(Ordering::Relaxed);
    let read_ops = total_read_ops.load(Ordering::Relaxed);
    let write_errs = total_write_errors.load(Ordering::Relaxed);
    let read_errs = total_read_errors.load(Ordering::Relaxed);

    // ── Verificação de integridade via SQL directo ────────────────────────────
    //
    // Abrimos uma ligação raw para obter COUNT, COUNT(DISTINCT) e MAX em uma
    // só query — mais eficiente do que carregar todas as linhas para memória.

    let conn = Connection::open(&db_path).expect("abrir DB para verificação");
    let (db_count, distinct_count, max_seq_val): (i64, i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COUNT(DISTINCT SequenceValue), COALESCE(MAX(SequenceValue), 0)
             FROM nns_assignment
             WHERE SequenceId = ?1",
            rusqlite::params![SEQ_ID],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query de verificação");

    let duplicates = db_count - distinct_count;

    eprintln!("\n=== Resultados ===");
    eprintln!("  Duração:          {:.1}s", elapsed.as_secs_f64());
    eprintln!("  Atribuições OK:   {assigned}");
    eprintln!("  Erros de escrita: {write_errs}");
    eprintln!(
        "  Throughput:       {:.1} allocs/s",
        assigned as f64 / elapsed.as_secs_f64()
    );
    eprintln!("  Operações leitura: {read_ops}");
    eprintln!("  Erros de leitura:  {read_errs}");
    eprintln!("  --- Integridade ---");
    eprintln!("  Linhas na BD:     {db_count}");
    eprintln!("  Valores distintos: {distinct_count}");
    eprintln!("  MAX(SequenceValue): {max_seq_val}");
    eprintln!("  Duplicados:        {duplicates}");

    if duplicates == 0 && db_count == assigned as i64 && max_seq_val == db_count {
        eprintln!("\n  ✓ Atomicidade confirmada — sem duplicados, sem lacunas.");
    }

    // ── Asserções ─────────────────────────────────────────────────────────────

    assert_eq!(
        write_errs, 0,
        "{write_errs} erros de escrita — ver log acima"
    );
    assert_eq!(
        read_errs, 0,
        "{read_errs} erros de leitura — leituras concorrentes falharam"
    );
    assert_eq!(
        duplicates, 0,
        "{duplicates} sequence_values duplicados — atomicidade violada!"
    );
    assert_eq!(
        db_count, assigned as i64,
        "BD tem {db_count} linhas mas foram reportadas {assigned} atribuições — perda de dados"
    );
    assert_eq!(
        max_seq_val, db_count,
        "MAX(SequenceValue)={max_seq_val} != COUNT={db_count} — há lacunas no contador"
    );
}

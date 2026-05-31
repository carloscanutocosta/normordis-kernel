/// Stress tests para o NNS (Numbering Service).
///
/// Dois testes:
///
/// 1. `correctness` — corre sempre (CI-friendly, < 1 s):
///    N threads × M alocações. Verifica que todos os `sequence_value`s são
///    únicos, sem lacunas e que não há erros de SQLITE_BUSY.
///
/// 2. `throughput` — `#[ignore]`, execução explícita:
///    8 writers durante 2 minutos. Relata alocações/s e erros a cada 10 s.
///    Comando: cargo test -p domain-numerador-sqlite --test nns_stress_tests -- --ignored --nocapture
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use adapter_sqlite::SqliteRelationalConfig;
use chrono::{NaiveDate, Utc};
use domain_numerador::{
    ActorRef, AssignNumberRequest, AssignmentMetadata, FormatPart, NumberFormat, NumberingKind,
    NumberingSequence, NumberingSequenceRepository, NumberingStore, ResetPolicy, TargetRef,
};
use numerador_sqlite::NumeradorDb;
use tempfile::tempdir;

// ─── Fixtures ─────────────────────────────────────────────────────────────────

/// Série sem reset (ResetPolicy::Never) — um único row em nns_counter partilhado
/// por todos os threads. Pior caso de contenção: maximiza conflitos de escrita.
fn oficio_sequence() -> NumberingSequence {
    NumberingSequence {
        sequence_id: "seq-oficio-stress".into(),
        kind: NumberingKind::Document,
        document_type: Some("oficio".into()),
        procedure_type: None,
        entity_id: "sf-setubal".into(),
        org_unit_id: None,
        padding: 6,
        reset_policy: ResetPolicy::Never,
        format: NumberFormat {
            separator: "/".into(),
            parts: vec![FormatPart::Literal("OF".into()), FormatPart::Sequence],
        },
        valid_from: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        valid_to: None,
    }
}

fn assign_request(target_id: &str) -> AssignNumberRequest {
    AssignNumberRequest {
        kind: NumberingKind::Document,
        target: TargetRef {
            id: target_id.into(),
            target_type: "document".into(),
        },
        document_type: Some("oficio".into()),
        procedure_type: None,
        entity_id: "sf-setubal".into(),
        org_unit_id: None,
        actor: ActorRef {
            id: "u-stress".into(),
            name: None,
        },
        requested_at: None,
        correlation_id: None,
        metadata: AssignmentMetadata::default(),
    }
}

fn open_db(path: &std::path::Path) -> NumeradorDb {
    NumeradorDb::open(&SqliteRelationalConfig::read_write_create(path)).unwrap()
}

// ─── Correctness test (CI) ────────────────────────────────────────────────────

/// Verifica que N threads a alocar números simultaneamente produzem
/// `sequence_value`s únicos, sem lacunas e sem erros de SQLITE_BUSY.
///
/// ResetPolicy::Never garante um único row de contador partilhado (pior caso).
#[test]
fn concurrent_allocation_produces_unique_contiguous_sequence_values() {
    const THREADS: usize = 4;
    const ALLOCS_PER_THREAD: usize = 50;
    const EXPECTED_TOTAL: u64 = (THREADS * ALLOCS_PER_THREAD) as u64;

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("nns-correctness.db");

    open_db(&db_path).upsert(&oficio_sequence()).unwrap();

    let db_path = Arc::new(db_path);
    let barrier = Arc::new(Barrier::new(THREADS));

    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let path = Arc::clone(&db_path);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut db = open_db(&path);
                barrier.wait(); // todos os threads arrancam ao mesmo tempo

                (0..ALLOCS_PER_THREAD)
                    .map(|i| {
                        let target = format!("doc-{t}-{i}");
                        let ref_id = format!("ref-{t}-{i}");
                        db.assign(&assign_request(&target), Utc::now(), &ref_id)
                            .unwrap_or_else(|e| panic!("thread {t} alocação {i} falhou: {e}"))
                            .sequence_value
                    })
                    .collect::<Vec<u64>>()
            })
        })
        .collect();

    let all: Vec<u64> = handles
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();

    assert_eq!(
        all.len(),
        (THREADS * ALLOCS_PER_THREAD),
        "contagem total incorrecta"
    );

    let unique: HashSet<u64> = all.iter().copied().collect();

    assert_eq!(
        unique.len(),
        all.len(),
        "{} valores duplicados detectados",
        all.len() - unique.len()
    );

    assert_eq!(
        *unique.iter().min().unwrap(),
        1,
        "sequence_value mínimo deveria ser 1"
    );
    assert_eq!(
        *unique.iter().max().unwrap(),
        EXPECTED_TOTAL,
        "sequence_value máximo deveria ser {EXPECTED_TOTAL}"
    );
}

// ─── Throughput test (ignore) ─────────────────────────────────────────────────

#[test]
#[ignore = "stress test de 2 minutos; correr com: cargo test -p domain-numerador-sqlite --test nns_stress_tests -- --ignored --nocapture"]
fn throughput_eight_writers_two_minutes() {
    const WRITER_THREADS: usize = 8;
    const DURATION: Duration = Duration::from_secs(2 * 60);
    const REPORT_INTERVAL: Duration = Duration::from_secs(10);

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("nns-throughput.db");

    open_db(&db_path).upsert(&oficio_sequence()).unwrap();

    let db_path = Arc::new(db_path);
    let barrier = Arc::new(Barrier::new(WRITER_THREADS + 1));
    let stop = Arc::new(AtomicBool::new(false));
    let total_allocs = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..WRITER_THREADS)
        .map(|t| {
            let path = Arc::clone(&db_path);
            let barrier = Arc::clone(&barrier);
            let stop = Arc::clone(&stop);
            let allocs = Arc::clone(&total_allocs);
            let errors = Arc::clone(&total_errors);

            thread::spawn(move || {
                let mut db = open_db(&path);
                let mut local_seq = 0u64;
                barrier.wait();

                while !stop.load(Ordering::Relaxed) {
                    let target = format!("doc-{t}-{local_seq}");
                    let ref_id = format!("ref-{t}-{local_seq}");
                    match db.assign(&assign_request(&target), Utc::now(), &ref_id) {
                        Ok(_) => {
                            allocs.fetch_add(1, Ordering::Relaxed);
                            local_seq += 1;
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::Relaxed);
                            eprintln!("[thread {t}] erro: {e}");
                        }
                    }
                }
            })
        })
        .collect();

    // Coordena arranque e supervisiona progresso
    barrier.wait();
    let started = Instant::now();
    let mut last_report = Instant::now();

    while started.elapsed() < DURATION {
        thread::sleep(Duration::from_millis(200));
        if last_report.elapsed() >= REPORT_INTERVAL {
            let elapsed = started.elapsed();
            let a = total_allocs.load(Ordering::Relaxed);
            let e = total_errors.load(Ordering::Relaxed);
            let rate = a as f64 / elapsed.as_secs_f64();
            eprintln!(
                "nns stress  elapsed={:>4.0}s  allocs={:>7}  errors={:>4}  rate={:>6.0}/s",
                elapsed.as_secs_f64(),
                a,
                e,
                rate,
            );
            last_report = Instant::now();
        }
    }
    stop.store(true, Ordering::Relaxed);

    for h in handles {
        h.join().unwrap();
    }

    let final_allocs = total_allocs.load(Ordering::Relaxed);
    let final_errors = total_errors.load(Ordering::Relaxed);
    let rate = final_allocs as f64 / DURATION.as_secs_f64();

    eprintln!(
        "\nnns stress final  allocs={final_allocs}  errors={final_errors}  rate={rate:.0}/s  threads={WRITER_THREADS}"
    );

    assert!(final_allocs > 0, "nenhuma alocação realizada");
    assert_eq!(
        final_errors, 0,
        "{final_errors} erros durante o stress test (possível SQLITE_BUSY sem retry)"
    );
}

use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use support_logging::{FileLogger, LogEvent, LogLevel, LoggingConfig, TechnicalLogger};
use tempfile::tempdir;

const STRESS_DURATION: Duration = Duration::from_secs(60);
const WRITER_THREADS: usize = 8;

#[test]
#[ignore = "stress test de 1 minuto; correr explicitamente com cargo test -p support-logging --test stress_tests -- --ignored --nocapture"]
fn concurrent_jsonl_logging_for_one_minute() {
    let dir = tempdir().unwrap();
    let mut config = LoggingConfig::new(dir.path(), "app.log");
    config.min_level = LogLevel::Trace;
    config.max_file_size_mb = 1;
    config.max_files = 5;
    config.max_message_chars = 512;
    config.max_details_bytes = 1024;
    config.flush_each_event = true;

    let logger = Arc::new(FileLogger::new(config).unwrap());
    let start = Arc::new(Barrier::new(WRITER_THREADS + 1));
    let stop = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for writer_id in 0..WRITER_THREADS {
        handles.push(spawn_writer(
            Arc::clone(&logger),
            Arc::clone(&start),
            Arc::clone(&stop),
            Arc::clone(&writes),
            Arc::clone(&errors),
            writer_id,
        ));
    }

    start.wait();
    let started_at = Instant::now();
    while started_at.elapsed() < STRESS_DURATION {
        thread::sleep(Duration::from_secs(5));
        eprintln!(
            "support-logging stress: elapsed={:?} writes={} errors={}",
            started_at.elapsed(),
            writes.load(Ordering::Relaxed),
            errors.load(Ordering::Relaxed),
        );
    }
    stop.store(true, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    let final_writes = writes.load(Ordering::Relaxed);
    let final_errors = errors.load(Ordering::Relaxed);
    let (line_count, rotated_count) = validate_log_files(dir.path());

    eprintln!(
        "support-logging stress final: writes={final_writes} errors={final_errors} lines={line_count} rotated_files={rotated_count}"
    );

    assert!(final_writes > 0, "stress test did not write log events");
    assert_eq!(final_errors, 0, "stress test observed logging errors");
    assert!(line_count > 0, "stress test produced no JSONL lines");
    assert!(rotated_count <= 4, "max_files total was not respected");
}

fn spawn_writer(
    logger: Arc<FileLogger>,
    start: Arc<Barrier>,
    stop: Arc<AtomicBool>,
    writes: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    writer_id: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut revision = 0_u64;
        start.wait();

        while !stop.load(Ordering::Relaxed) {
            let event = LogEvent::new(
                LogLevel::Info,
                "support-logging-stress",
                format!("writer-{writer_id} revision-{revision}\nwith newline"),
            )
            .with_details(serde_json::json!({
                "writer": writer_id,
                "revision": revision,
                "password": "must-not-appear",
                "payload": "must-not-appear",
                "safe": "visible"
            }));

            match logger.log(event) {
                Ok(()) => {
                    writes.fetch_add(1, Ordering::Relaxed);
                    revision = revision.wrapping_add(1);
                }
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    })
}

fn validate_log_files(log_dir: &std::path::Path) -> (usize, usize) {
    let mut lines = 0_usize;
    let mut rotated = 0_usize;

    for entry in fs::read_dir(log_dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy();
        if name != "app.log" && name.starts_with("app.") && name.ends_with(".log") {
            rotated += 1;
        }
        if name != "app.log" && !(name.starts_with("app.") && name.ends_with(".log")) {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("must-not-appear"));
        for line in content.lines() {
            let value: Value = serde_json::from_str(line).unwrap();
            assert!(value["message"].as_str().unwrap().contains("writer-"));
            assert!(!value["message"].as_str().unwrap().contains('\n'));
            assert_eq!(value["details"]["password"], "[REDACTED]");
            assert_eq!(value["details"]["payload"], "[REDACTED]");
            lines += 1;
        }
    }

    (lines, rotated)
}

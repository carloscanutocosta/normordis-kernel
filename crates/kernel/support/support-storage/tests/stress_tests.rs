use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;
use support_crypto::{SecretKey, StaticKeyProvider};
use support_storage::{
    CryptoStorageProtector, JsonStorageCodec, MemoryStorage, ProtectedStorage, Storage, StorageKey,
    StorageNamespace,
};

const STRESS_DURATION: Duration = Duration::from_secs(5 * 60);
const WRITER_THREADS: usize = 4;
const READER_THREADS: usize = 8;
const KEY_COUNT: usize = 128;

#[test]
#[ignore = "stress test de 5 minutos; correr explicitamente com cargo test -p support-storage --test stress_tests -- --ignored --nocapture"]
fn concurrent_reads_and_writes_for_five_minutes() {
    let storage = Arc::new(build_storage());
    let namespace = StorageNamespace::new("stress.concurrent").unwrap();

    for key_index in 0..KEY_COUNT {
        let key = storage_key(key_index);
        storage
            .put_json(
                &namespace,
                &key,
                &json!({
                    "key": key_index,
                    "writer": "seed",
                    "revision": 0,
                    "payload": {"items": [1, 2, 3], "active": true}
                }),
            )
            .unwrap();
    }

    let start = Arc::new(Barrier::new(WRITER_THREADS + READER_THREADS + 1));
    let stop = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicU64::new(0));
    let reads = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for writer_id in 0..WRITER_THREADS {
        handles.push(spawn_writer(
            Arc::clone(&storage),
            namespace.clone(),
            Arc::clone(&start),
            Arc::clone(&stop),
            Arc::clone(&writes),
            Arc::clone(&errors),
            writer_id,
        ));
    }

    for reader_id in 0..READER_THREADS {
        handles.push(spawn_reader(
            Arc::clone(&storage),
            namespace.clone(),
            Arc::clone(&start),
            Arc::clone(&stop),
            Arc::clone(&reads),
            Arc::clone(&errors),
            reader_id,
        ));
    }

    start.wait();
    let started_at = Instant::now();
    while started_at.elapsed() < STRESS_DURATION {
        thread::sleep(Duration::from_secs(5));
        eprintln!(
            "support-storage stress: elapsed={:?} reads={} writes={} errors={}",
            started_at.elapsed(),
            reads.load(Ordering::Relaxed),
            writes.load(Ordering::Relaxed),
            errors.load(Ordering::Relaxed),
        );
    }
    stop.store(true, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    let final_reads = reads.load(Ordering::Relaxed);
    let final_writes = writes.load(Ordering::Relaxed);
    let final_errors = errors.load(Ordering::Relaxed);

    eprintln!(
        "support-storage stress final: reads={final_reads} writes={final_writes} errors={final_errors}"
    );

    assert!(final_reads > 0, "stress test did not perform reads");
    assert!(final_writes > 0, "stress test did not perform writes");
    assert_eq!(final_errors, 0, "stress test observed storage errors");
}

fn build_storage(
) -> ProtectedStorage<MemoryStorage, JsonStorageCodec, CryptoStorageProtector<StaticKeyProvider>> {
    let keys = StaticKeyProvider::new(SecretKey::new([31; support_crypto::KEY_LENGTH_BYTES]), None);

    ProtectedStorage::new(
        MemoryStorage::default(),
        JsonStorageCodec,
        CryptoStorageProtector::new(keys),
    )
}

fn spawn_writer(
    storage: Arc<
        ProtectedStorage<
            MemoryStorage,
            JsonStorageCodec,
            CryptoStorageProtector<StaticKeyProvider>,
        >,
    >,
    namespace: StorageNamespace,
    start: Arc<Barrier>,
    stop: Arc<AtomicBool>,
    writes: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    writer_id: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = seed_for(writer_id as u64, 0xA11C_E551);
        let mut revision = 0_u64;
        start.wait();

        while !stop.load(Ordering::Relaxed) {
            let key_index = next_index(&mut state);
            let key = storage_key(key_index);
            let value = json!({
                "key": key_index,
                "writer": writer_id,
                "revision": revision,
                "payload": {
                    "checksum_hint": state,
                    "items": [revision, revision + 1, revision + 2],
                    "active": revision.is_multiple_of(2)
                }
            });

            match storage.put_json(&namespace, &key, &value) {
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

fn spawn_reader(
    storage: Arc<
        ProtectedStorage<
            MemoryStorage,
            JsonStorageCodec,
            CryptoStorageProtector<StaticKeyProvider>,
        >,
    >,
    namespace: StorageNamespace,
    start: Arc<Barrier>,
    stop: Arc<AtomicBool>,
    reads: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    reader_id: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = seed_for(reader_id as u64, 0x5EED_BAAD);
        start.wait();

        while !stop.load(Ordering::Relaxed) {
            let key_index = next_index(&mut state);
            let key = storage_key(key_index);

            match storage.get_json(&namespace, &key) {
                Ok(Some(value)) => {
                    if value.get("key").and_then(|value| value.as_u64()) == Some(key_index as u64) {
                        reads.fetch_add(1, Ordering::Relaxed);
                    } else {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Ok(None) | Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    })
}

fn storage_key(index: usize) -> StorageKey {
    StorageKey::new(format!("key-{index}")).unwrap()
}

fn next_index(state: &mut u64) -> usize {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    ((*state >> 32) as usize) % KEY_COUNT
}

fn seed_for(id: u64, salt: u64) -> u64 {
    salt ^ (id + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

use std::path::Path;

use infra_backup::{
    MaintenanceConfig, MaintenanceRepository, MaintenanceService, MaintenanceStatus,
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_test_db(path: &Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch("CREATE TABLE docs (id INTEGER PRIMARY KEY, body TEXT NOT NULL);")
        .unwrap();
    for i in 0..10 {
        conn.execute("INSERT INTO docs (body) VALUES (?1)", [format!("row-{i}")])
            .unwrap();
    }
}

fn test_config(dir: &TempDir, db_paths: Vec<String>) -> MaintenanceConfig {
    MaintenanceConfig {
        schedule_time: "16:00".into(),
        destination_path: dir.path().join("backups").to_string_lossy().into_owned(),
        keep_last: 3,
        db_paths,
        control_db_path: dir.path().join("control.db").to_string_lossy().into_owned(),
        backup_passphrase: "passphrase-de-teste-segura".into(),
    }
}

/// Insere registos de runs bem-sucedidos diretamente no control.db para testes de rotação.
fn insert_fake_successful_runs(control_db: &Path, n: usize) {
    let conn = rusqlite::Connection::open(control_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS maintenance_log (
            id TEXT PRIMARY KEY, run_date TEXT NOT NULL UNIQUE,
            started_at TEXT NOT NULL, finished_at TEXT,
            triggered_by TEXT NOT NULL DEFAULT 'test',
            status TEXT NOT NULL, backup_path TEXT,
            backup_size INTEGER, checksum TEXT, purged_at TEXT
        );",
    )
    .unwrap();
    for i in 0..n {
        let date = format!("2026-01-{:02}", i + 1);
        let id = uuid::Uuid::new_v4().to_string();
        // O ficheiro não existe — rotation.rs tolera NotFound
        let fake_path = format!("/tmp/fake_backup_{date}.mbak");
        conn.execute(
            "INSERT INTO maintenance_log \
             (id, run_date, started_at, finished_at, triggered_by, status, backup_path, backup_size, checksum) \
             VALUES (?1,?2,'2026-01-01T00:00:00Z','2026-01-01T00:01:00Z','test','success',?3,1024,'abc')",
            rusqlite::params![id, date, fake_path],
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// Config validation
// ---------------------------------------------------------------------------

#[test]
fn config_rejects_empty_passphrase() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_config(&dir, vec!["app.db".into()]);
    config.backup_passphrase = "".into();
    assert!(MaintenanceService::new(config).is_err());
}

#[test]
fn config_rejects_empty_db_paths() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_config(&dir, vec![]);
    config.db_paths = vec![];
    assert!(MaintenanceService::new(config).is_err());
}

#[test]
fn config_rejects_zero_keep_last() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = test_config(&dir, vec!["app.db".into()]);
    config.keep_last = 0;
    assert!(MaintenanceService::new(config).is_err());
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lock_is_acquired_once_per_day() {
    let dir = tempfile::tempdir().unwrap();
    let repo =
        MaintenanceRepository::new(dir.path().join("control.db").to_string_lossy().into_owned());

    let first = repo.try_acquire_lock("test").await.unwrap();
    assert!(first.is_some(), "primeira aquisição deve ter sucesso");

    let second = repo.try_acquire_lock("test").await.unwrap();
    assert!(
        second.is_none(),
        "segunda aquisição no mesmo dia deve ser ignorada"
    );
}

#[tokio::test]
async fn finalize_run_persists_status_and_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let repo =
        MaintenanceRepository::new(dir.path().join("control.db").to_string_lossy().into_owned());

    let run = repo.try_acquire_lock("test").await.unwrap().unwrap();
    repo.finalize_run(
        run.id,
        MaintenanceStatus::Success,
        Some("/backups/backup_2026-05-21.mbak"),
        Some(4096),
        Some("deadbeef"),
    )
    .await
    .unwrap();

    let runs = repo.list_all_runs(10).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, MaintenanceStatus::Success);
    assert_eq!(
        runs[0].backup_path.as_deref(),
        Some("/backups/backup_2026-05-21.mbak")
    );
    assert_eq!(runs[0].backup_size, Some(4096));
    assert_eq!(runs[0].checksum.as_deref(), Some("deadbeef"));
    assert!(runs[0].finished_at.is_some());
}

#[tokio::test]
async fn list_successful_excludes_purged_and_failed() {
    let dir = tempfile::tempdir().unwrap();
    let control_db = dir.path().join("control.db");

    insert_fake_successful_runs(&control_db, 3);

    let repo = MaintenanceRepository::new(control_db.to_string_lossy().into_owned());
    let runs = repo.list_successful_backups().await.unwrap();
    assert_eq!(runs.len(), 3);

    // Purgar o mais antigo
    let oldest = runs.last().unwrap();
    repo.mark_purged(oldest.id).await.unwrap();

    let after_purge = repo.list_successful_backups().await.unwrap();
    assert_eq!(after_purge.len(), 2, "purged entry should not appear");
}

// ---------------------------------------------------------------------------
// Full maintenance run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_run_creates_encrypted_backup() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("app.db");
    make_test_db(&db_path);

    let config = test_config(&dir, vec![db_path.to_string_lossy().into_owned()]);
    let service = MaintenanceService::new(config).unwrap();

    service.run("test").await.unwrap();

    let runs = service.repository().list_all_runs(10).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, MaintenanceStatus::Success);

    let backup_path = runs[0].backup_path.as_ref().unwrap();
    assert!(
        std::path::Path::new(backup_path).exists(),
        "backup file must exist on disk"
    );

    // Ficheiro cifrado não deve conter texto SQLite em claro
    let bytes = std::fs::read(backup_path).unwrap();
    assert!(
        !bytes.windows(6).any(|w| w == b"SQLite"),
        "backup must be encrypted (no plaintext SQLite magic bytes)"
    );
}

#[tokio::test]
async fn second_run_same_day_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("app.db");
    make_test_db(&db_path);

    let config = test_config(&dir, vec![db_path.to_string_lossy().into_owned()]);
    let service = MaintenanceService::new(config).unwrap();

    service.run("test").await.unwrap();
    service.run("test").await.unwrap(); // deve retornar Ok silenciosamente

    let runs = service.repository().list_all_runs(10).await.unwrap();
    assert_eq!(runs.len(), 1, "only one run should exist");
}

#[tokio::test]
async fn partial_run_when_one_db_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let good_db = dir.path().join("good.db");
    let bad_db = dir.path().join("bad.db");

    make_test_db(&good_db);
    std::fs::write(&bad_db, b"isto nao e um ficheiro sqlite valido").unwrap();

    let config = test_config(
        &dir,
        vec![
            good_db.to_string_lossy().into_owned(),
            bad_db.to_string_lossy().into_owned(),
        ],
    );
    let service = MaintenanceService::new(config).unwrap();

    service.run("test").await.unwrap();

    let runs = service.repository().list_all_runs(10).await.unwrap();
    assert_eq!(runs[0].status, MaintenanceStatus::Partial);

    let details = service
        .repository()
        .get_run_details(runs[0].id)
        .await
        .unwrap();
    assert_eq!(details.len(), 2);

    let good = details.iter().find(|d| d.db_name == "good.db").unwrap();
    assert!(good.backup_included);

    let bad = details.iter().find(|d| d.db_name == "bad.db").unwrap();
    assert!(!bad.backup_included);
}

// ---------------------------------------------------------------------------
// Restore
// ---------------------------------------------------------------------------

#[tokio::test]
async fn restore_recovers_original_db_data() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("app.db");
    make_test_db(&db_path);

    let config = test_config(&dir, vec![db_path.to_string_lossy().into_owned()]);
    let service = MaintenanceService::new(config).unwrap();

    service.run("test").await.unwrap();

    let runs = service.repository().list_all_runs(1).await.unwrap();
    let run = &runs[0];

    let restore_dir = dir.path().join("restored");
    let restored = service.restore(run, &restore_dir).await.unwrap();

    assert!(
        !restored.is_empty(),
        "deve restaurar pelo menos um ficheiro"
    );

    let restored_db = restore_dir.join("app.db");
    assert!(
        restored_db.exists(),
        "app.db deve existir no diretório de restore"
    );

    let conn = rusqlite::Connection::open(&restored_db).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM docs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 10, "restored DB must have original row count");
}

#[tokio::test]
async fn restore_fails_on_checksum_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("app.db");
    make_test_db(&db_path);

    let config = test_config(&dir, vec![db_path.to_string_lossy().into_owned()]);
    let service = MaintenanceService::new(config).unwrap();

    service.run("test").await.unwrap();

    let mut runs = service.repository().list_all_runs(1).await.unwrap();
    // Adulterar o checksum para simular arquivo corrompido
    runs[0].checksum = Some("checksum-falso-adulterado".into());

    let restore_dir = dir.path().join("restored");
    let result = service.restore(&runs[0], &restore_dir).await;
    assert!(result.is_err(), "restore with tampered checksum must fail");
}

// ---------------------------------------------------------------------------
// Rotation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rotation_purges_oldest_beyond_keep_last() {
    let dir = tempfile::tempdir().unwrap();
    let control_db = dir.path().join("control.db");

    insert_fake_successful_runs(&control_db, 5);

    let repo = MaintenanceRepository::new(control_db.to_string_lossy().into_owned());

    let before = repo.list_successful_backups().await.unwrap();
    assert_eq!(before.len(), 5);

    // Rodar rotação com keep_last=3
    infra_backup::rotate_backups(&repo, 3).await.unwrap();

    let after = repo.list_successful_backups().await.unwrap();
    assert_eq!(after.len(), 3, "apenas 3 backups devem permanecer activos");

    // Os mais recentes (2026-01-05, 04, 03) ficam; 02 e 01 são purgados
    let dates: Vec<_> = after.iter().map(|r| r.run_date.to_string()).collect();
    assert!(dates.contains(&"2026-01-05".to_string()));
    assert!(dates.contains(&"2026-01-04".to_string()));
    assert!(dates.contains(&"2026-01-03".to_string()));
}

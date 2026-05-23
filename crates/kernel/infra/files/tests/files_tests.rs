use core_config::PathsConfig;
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use files::{
    ensure_directories, generate_technical_filename, prune_stale_temp_files, resolve_layout,
};

#[test]
fn technical_filename_is_sanitized() {
    let name = generate_technical_filename("oficio final", "abc/123", ".pdf");
    assert_eq!(name, "oficio_final_abc_123.pdf");
}

#[test]
fn ensure_directories_creates_layout() {
    let base = std::env::temp_dir().join("support_files_test_layout");
    let _ = fs::remove_dir_all(&base);

    let layout = resolve_layout(&base, &PathsConfig::default());
    ensure_directories(&layout).unwrap();

    assert!(PathBuf::from(&layout.data_dir).exists());
    assert!(PathBuf::from(&layout.artifacts_dir).exists());

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn prune_stale_temp_files_removes_old_entries() {
    let base = std::env::temp_dir().join("support_files_test_tmp_cleanup");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let old_file = base.join("old.txt");
    fs::write(&old_file, "old").unwrap();
    sleep(Duration::from_secs(2));

    let recent_file = base.join("recent.txt");
    fs::write(&recent_file, "recent").unwrap();

    prune_stale_temp_files(&base, Duration::from_secs(1)).unwrap();

    assert!(!old_file.exists());
    assert!(recent_file.exists());

    let _ = fs::remove_dir_all(&base);
}

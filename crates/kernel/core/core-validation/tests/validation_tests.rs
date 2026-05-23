use core_validation::validators::{email, iban, json, nif, string, uuid};
use core_validation::{
    manifest_file, sha256_bytes, sha256_file, ManifestEntry, ValidationError, ValidationIssue,
    ValidationReport, ValidationSeverity, EMAIL_FORMAT,
};
use serde_json::json;
use std::fs;
use std::io::Write;

#[test]
fn report_ok_is_valid() {
    assert!(ValidationReport::ok().is_valid());
}

#[test]
fn report_with_error_is_invalid() {
    let report =
        ValidationReport::with_issue(ValidationIssue::error(EMAIL_FORMAT, "email", "invalid"));

    assert!(!report.is_valid());
}

#[test]
fn report_with_warning_is_valid() {
    let report =
        ValidationReport::with_issue(ValidationIssue::warning(EMAIL_FORMAT, "email", "warning"));

    assert!(report.is_valid());
}

#[test]
fn required_accepts_non_empty_text() {
    assert!(string::required("name", "Alice").is_valid());
}

#[test]
fn required_rejects_empty_text() {
    assert!(!string::required("name", " \t ").is_valid());
}

#[test]
fn max_length_rejects_excess() {
    assert!(!string::max_length("name", "abcd", 3).is_valid());
}

#[test]
fn email_valid() {
    assert!(email::validate_email("email", "user@example.com").is_valid());
}

#[test]
fn email_with_spaces_is_invalid() {
    assert!(!email::validate_email("email", " user@example.com").is_valid());
    assert!(!email::validate_email("email", "user @example.com").is_valid());
}

#[test]
fn uuid_valid() {
    assert!(uuid::validate_uuid("id", "550e8400-e29b-41d4-a716-446655440000").is_valid());
}

#[test]
fn uuid_invalid() {
    assert!(!uuid::validate_uuid("id", "not-a-uuid").is_valid());
}

#[test]
fn nif_valid() {
    assert!(nif::validate_nif("nif", "501964843").is_valid());
}

#[test]
fn nif_with_invalid_checksum_is_invalid() {
    assert!(!nif::validate_nif("nif", "501964844").is_valid());
}

#[test]
fn nif_with_letters_is_invalid() {
    assert!(!nif::validate_nif("nif", "50196484A").is_valid());
}

#[test]
fn iban_valid() {
    assert!(iban::validate_iban("iban", "PT50 0002 0123 1234 5678 9015 4").is_valid());
}

#[test]
fn iban_mod97_invalid() {
    assert!(!iban::validate_iban("iban", "PT50 0002 0123 1234 5678 9015 5").is_valid());
}

#[test]
fn json_object_valid() {
    assert!(json::require_object("payload", &json!({ "name": "Alice" })).is_valid());
}

#[test]
fn json_non_object_invalid() {
    assert!(!json::require_object("payload", &json!(["Alice"])).is_valid());
}

#[test]
fn require_field_finds_field() {
    assert!(json::require_field("payload", &json!({ "name": "Alice" }), "name").is_valid());
}

#[test]
fn require_field_fails_when_missing() {
    assert!(!json::require_field("payload", &json!({ "name": "Alice" }), "age").is_valid());
}

#[test]
fn validation_error_converts_to_mini_error() {
    let mini_error = ValidationError::InvalidInput.to_mini_error();

    assert_eq!(mini_error.code.as_str(), "MINI.VALIDATION.INVALID_INPUT");
    assert_eq!(mini_error.component.as_str(), "core-validation");
}

#[test]
fn sha256_bytes_is_deterministic() {
    assert_eq!(sha256_bytes(b"abc"), sha256_bytes(b"abc"));
}

#[test]
fn sha256_bytes_returns_lowercase_hex() {
    let hash = sha256_bytes(b"ABC");

    assert_eq!(hash.len(), 64);
    assert!(hash
        .chars()
        .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch)));
}

#[test]
fn sha256_bytes_matches_known_value() {
    assert_eq!(
        sha256_bytes(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn sha256_file_reads_small_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.txt");
    fs::write(&path, b"abc").unwrap();

    assert_eq!(
        sha256_file(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn sha256_file_rejects_directory() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        sha256_file(dir.path()).unwrap_err(),
        ValidationError::NotRegularFile
    );
}

#[test]
fn sha256_file_fails_when_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        sha256_file(dir.path().join("missing.txt")).unwrap_err(),
        ValidationError::FileNotFound
    );
}

#[test]
fn manifest_file_generates_path_size_and_sha256() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.txt");
    fs::write(&path, b"abc").unwrap();

    let manifest = manifest_file(&path).unwrap();

    assert_eq!(manifest.path, path.to_string_lossy().replace('\\', "/"));
    assert_eq!(manifest.size, 3);
    assert_eq!(
        manifest.sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn manifest_file_is_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.txt");
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(b"abc").unwrap();
    file.sync_all().unwrap();

    assert_eq!(manifest_file(&path).unwrap(), manifest_file(&path).unwrap());
}

#[test]
fn manifest_file_rejects_directory() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        manifest_file(dir.path()).unwrap_err(),
        ValidationError::NotRegularFile
    );
}

#[test]
fn integrity_error_converts_to_mini_error() {
    let mini_error = ValidationError::FileNotFound.to_mini_error();

    assert_eq!(mini_error.code.as_str(), "MINI.VALIDATION.FILE_NOT_FOUND");
    assert_eq!(mini_error.component.as_str(), "core-validation");
}

#[test]
fn manifest_entry_is_public() {
    let entry = ManifestEntry {
        path: "sample.txt".to_string(),
        size: 3,
        sha256: sha256_bytes(b"abc"),
    };

    assert_eq!(entry.size, 3);
}

#[test]
fn crate_does_not_depend_on_sqlite() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("rusqlite"));
    assert!(!manifest.contains("sqlite"));
    assert!(!manifest.contains("adapter-sqlite"));
}

#[test]
fn crate_does_not_depend_on_tauri() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("tauri"));
}

#[test]
fn crate_does_not_depend_on_core_audit() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("core-audit"));
}

#[test]
fn merge_keeps_warning_only_report_valid() {
    let mut report = ValidationReport::ok();
    report.push(ValidationIssue::new(
        "validation.example",
        Some("field".to_string()),
        ValidationSeverity::Warning,
        "warning",
    ));

    assert!(report.is_valid());
}

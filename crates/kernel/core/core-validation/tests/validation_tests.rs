use core_validation::validators::{
    cc, coherence, cp, email, hash_format, iban, json, mime, nif, niss, phone, range, semver,
    string, uuid,
};
use core_validation::{
    manifest_file, sha256_bytes, sha256_file, ManifestEntry, ManifestList, RuleOutcome,
    ValidationContext, ValidationError, ValidationIssue, ValidationReport, ValidationResult,
    ValidationSeverity, ValidationStatus, EMAIL_FORMAT,
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

// ── ValidationStatus ──────────────────────────────────────────────────────────

#[test]
fn status_failed_is_blocking() {
    assert!(ValidationStatus::Failed.is_blocking());
    assert!(!ValidationStatus::Passed.is_blocking());
    assert!(!ValidationStatus::Warning.is_blocking());
}

#[test]
fn status_allows_progression() {
    assert!(ValidationStatus::Passed.allows_progression());
    assert!(ValidationStatus::Warning.allows_progression());
    assert!(ValidationStatus::Skipped.allows_progression());
    assert!(ValidationStatus::NotApplicable.allows_progression());
    assert!(ValidationStatus::Overridden.allows_progression());
    assert!(!ValidationStatus::Failed.allows_progression());
    assert!(!ValidationStatus::ExecutionError.allows_progression());
}

// ── ValidationContext ─────────────────────────────────────────────────────────

#[test]
fn context_builder_sets_fields() {
    let ctx = ValidationContext::new("2026-06-03T14:00:00Z")
        .with_actor("user_abc")
        .with_scope("uo_001")
        .with_engine_version("0.3.0");

    assert_eq!(ctx.actor_id.as_deref(), Some("user_abc"));
    assert_eq!(ctx.scope.as_deref(), Some("uo_001"));
    assert_eq!(ctx.timestamp_rfc3339, "2026-06-03T14:00:00Z");
    assert_eq!(ctx.engine_version.as_deref(), Some("0.3.0"));
}

#[test]
fn context_minimal_has_no_optional_fields() {
    let ctx = ValidationContext::new("2026-06-03T00:00:00Z");
    assert!(ctx.actor_id.is_none());
    assert!(ctx.scope.is_none());
    assert!(ctx.engine_version.is_none());
}

// ── ValidationResult ──────────────────────────────────────────────────────────

#[test]
fn result_new_requires_explicit_status() {
    let result = ValidationResult::new("val_000", "Artefacto", "art_x", ValidationStatus::Failed);
    assert_eq!(result.overall_status, ValidationStatus::Failed);
    assert!(result.outcomes.is_empty());
}

#[test]
fn result_from_clean_report_is_passed() {
    let report = ValidationReport::ok();
    let result =
        ValidationResult::from_report("val_001", "DocumentInstance", "doc_abc", None, &report);

    assert_eq!(result.overall_status, ValidationStatus::Passed);
    assert!(result.outcomes.is_empty());
    assert!(result.allows_progression());
}

#[test]
fn result_from_report_with_error_is_failed() {
    let report = ValidationReport::with_issue(ValidationIssue::error(
        "DOC.HASH.REQUIRED",
        "payload_hash",
        "hash is missing",
    ));
    let result =
        ValidationResult::from_report("val_002", "DocumentInstance", "doc_xyz", None, &report);

    assert_eq!(result.overall_status, ValidationStatus::Failed);
    assert!(result.is_blocking());
    assert_eq!(result.outcomes[0].status, ValidationStatus::Failed);
}

#[test]
fn result_from_report_with_warning_only_is_warning() {
    let mut report = ValidationReport::ok();
    report.push(ValidationIssue::warning(
        "DOC.FIELD.OPTIONAL",
        "notes",
        "optional field missing",
    ));
    let result =
        ValidationResult::from_report("val_003", "DocumentInstance", "doc_w", None, &report);

    assert_eq!(result.overall_status, ValidationStatus::Warning);
    assert!(result.allows_progression());
}

#[test]
fn result_with_context_stores_it() {
    let ctx = ValidationContext::new("2026-06-03T10:00:00Z").with_actor("svc_ingest");
    let result = ValidationResult::new(
        "val_004",
        "IngestPackage",
        "pkg_001",
        ValidationStatus::Passed,
    )
    .with_context(ctx);

    assert!(result.context.is_some());
    assert_eq!(
        result.context.unwrap().actor_id.as_deref(),
        Some("svc_ingest")
    );
}

#[test]
fn result_from_report_with_checked_records_passed_rules() {
    let report = ValidationReport::ok(); // nif passou — nenhum issue
    let result = ValidationResult::from_report_with_checked(
        "val_010",
        "Pessoa",
        "p_abc",
        None,
        &report,
        &["validation.nif.format", "validation.nif.checksum"],
    );
    assert_eq!(result.overall_status, ValidationStatus::Passed);
    assert_eq!(result.outcomes.len(), 2);
    assert!(result
        .outcomes
        .iter()
        .all(|o| o.status == ValidationStatus::Passed));
}

#[test]
fn result_from_report_with_checked_does_not_duplicate_failed_rules() {
    let report = ValidationReport::with_issue(ValidationIssue::error(
        "validation.nif.checksum",
        "nif",
        "checksum inválido",
    ));
    let result = ValidationResult::from_report_with_checked(
        "val_011",
        "Pessoa",
        "p_xyz",
        None,
        &report,
        &["validation.nif.format", "validation.nif.checksum"],
    );
    // nif.checksum aparece como Failed (do report); nif.format aparece como Passed (de checked)
    assert_eq!(result.outcomes.len(), 2);
    let failed = result
        .outcomes
        .iter()
        .find(|o| o.rule_id == "validation.nif.checksum")
        .unwrap();
    let passed = result
        .outcomes
        .iter()
        .find(|o| o.rule_id == "validation.nif.format")
        .unwrap();
    assert_eq!(failed.status, ValidationStatus::Failed);
    assert_eq!(passed.status, ValidationStatus::Passed);
}

#[test]
fn rule_outcome_constructors() {
    assert_eq!(RuleOutcome::passed("R1").status, ValidationStatus::Passed);
    assert_eq!(
        RuleOutcome::failed("R2", "err").status,
        ValidationStatus::Failed
    );
    assert_eq!(RuleOutcome::skipped("R3").status, ValidationStatus::Skipped);
    assert_eq!(
        RuleOutcome::not_applicable("R4").status,
        ValidationStatus::NotApplicable
    );
    assert_eq!(
        RuleOutcome::overridden("R5", "justified by supervisor").status,
        ValidationStatus::Overridden
    );
}

// ── NISS ─────────────────────────────────────────────────────────────────────
//
// NISS de teste gerado manualmente:
//   Base: "112345678" — categoria 1, dígitos 2..9 = "12345678"
//   Pesos: [29,23,19,17,13,11,7,5,3]
//   Soma: 1*29+1*23+2*19+3*17+4*13+5*11+6*7+7*5+8*3 = 349
//   controlo = 9 - ((349-1) % 9) = 9 - (348 % 9) = 9 - 6 = 3
//   NISS completo: "11234567803"
//
// O dígito de posição 10 é sempre '0'; posição 11 é o controlo (3).

#[test]
fn niss_valid() {
    assert!(niss::validate_niss("niss", "11234567803").is_valid());
}

#[test]
fn niss_valid_with_spaces() {
    assert!(niss::validate_niss("niss", "1 1234567803").is_valid());
}

#[test]
fn niss_wrong_checksum_is_invalid() {
    assert!(!niss::validate_niss("niss", "11234567804").is_valid());
}

#[test]
fn niss_wrong_length_is_invalid() {
    assert!(!niss::validate_niss("niss", "1123456780").is_valid()); // 10 digits
    assert!(!niss::validate_niss("niss", "112345678031").is_valid()); // 12 digits
}

#[test]
fn niss_invalid_category_is_invalid() {
    // Category digit 4 is not valid
    assert!(!niss::validate_niss("niss", "41234567800").is_valid());
}

#[test]
fn niss_letters_are_invalid() {
    assert!(!niss::validate_niss("niss", "1123456780A").is_valid());
}

#[test]
fn niss_auxiliary_digit_nonzero_is_invalid() {
    // Position 10 (index 9) must be '0' — here it's '1'
    assert!(!niss::validate_niss("niss", "11234567813").is_valid());
}

// ── Coherence ─────────────────────────────────────────────────────────────────

#[test]
fn date_range_valid_equal_dates() {
    assert!(coherence::validate_date_range("period", "2026-01-01", "2026-01-01").is_valid());
}

#[test]
fn date_range_valid_start_before_end() {
    assert!(coherence::validate_date_range("period", "2026-01-01", "2026-12-31").is_valid());
}

#[test]
fn date_range_invalid_start_after_end() {
    assert!(!coherence::validate_date_range("period", "2026-12-31", "2026-01-01").is_valid());
}

#[test]
fn date_range_rejects_datetime_strings() {
    // validate_date_range só aceita YYYY-MM-DD — datetimes devem usar validate_datetime_range
    assert!(!coherence::validate_date_range(
        "interval",
        "2026-06-01T08:00:00Z",
        "2026-06-01T18:00:00Z"
    )
    .is_valid());
}

#[test]
fn date_range_rejects_mixed_formats() {
    assert!(
        !coherence::validate_date_range("interval", "2026-06-01", "2026-06-01T18:00:00Z")
            .is_valid()
    );
}

#[test]
fn datetime_range_valid() {
    assert!(coherence::validate_datetime_range(
        "interval",
        "2026-06-01T08:00:00Z",
        "2026-06-01T18:00:00Z"
    )
    .is_valid());
}

#[test]
fn datetime_range_invalid_start_after_end() {
    assert!(!coherence::validate_datetime_range(
        "interval",
        "2026-06-01T18:00:00Z",
        "2026-06-01T08:00:00Z"
    )
    .is_valid());
}

#[test]
fn datetime_range_rejects_date_only_strings() {
    assert!(!coherence::validate_datetime_range("interval", "2026-06-01", "2026-06-02").is_valid());
}

#[test]
fn datetime_range_rejects_mixed_formats() {
    assert!(
        !coherence::validate_datetime_range("interval", "2026-06-01T08:00:00Z", "2026-06-02")
            .is_valid()
    );
}

// ── Cross-offset com comparação UTC correcta ──────────────────────────────────

#[test]
fn datetime_range_cross_offset_same_instant_is_valid() {
    // "2026-01-01T10:00:00+01:00" = 09:00 UTC = "2026-01-01T09:00:00Z" → mesmo instante → válido
    assert!(coherence::validate_datetime_range(
        "t",
        "2026-01-01T10:00:00+01:00",
        "2026-01-01T09:00:00Z",
    )
    .is_valid());
}

#[test]
fn datetime_range_cross_offset_start_before_end_in_utc_is_valid() {
    // "2026-01-01T08:00:00+01:00" = 07:00 UTC < "2026-01-01T09:00:00Z" = 09:00 UTC → válido
    assert!(coherence::validate_datetime_range(
        "t",
        "2026-01-01T08:00:00+01:00",
        "2026-01-01T09:00:00Z",
    )
    .is_valid());
}

#[test]
fn datetime_range_cross_offset_start_after_end_in_utc_is_invalid() {
    // "2026-01-01T18:00:00+01:00" = 17:00 UTC > "2026-01-01T08:00:00Z" = 08:00 UTC → inválido
    assert!(!coherence::validate_datetime_range(
        "t",
        "2026-01-01T18:00:00+01:00",
        "2026-01-01T08:00:00Z",
    )
    .is_valid());
}

#[test]
fn datetime_range_z_and_plus_zero_same_instant() {
    // Z e +00:00 representam o mesmo instante → válido, sem issues
    let report = coherence::validate_datetime_range(
        "t",
        "2026-01-01T08:00:00Z",
        "2026-01-01T18:00:00+00:00",
    );
    assert!(report.is_valid());
    assert!(report.issues.is_empty());
}

#[test]
fn datetime_range_fractional_seconds_accepted() {
    assert!(coherence::validate_datetime_range(
        "t",
        "2026-01-01T08:00:00.123Z",
        "2026-01-01T18:00:00.456Z",
    )
    .is_valid());
}

// ── Validação semântica de datas e datetimes ──────────────────────────────────

#[test]
fn date_range_rejects_month_out_of_range() {
    assert!(!coherence::validate_date_range("d", "2026-00-01", "2026-12-31").is_valid()); // mês 0
    assert!(!coherence::validate_date_range("d", "2026-01-01", "2026-13-31").is_valid()); // mês 13
    assert!(!coherence::validate_date_range("d", "2026-99-01", "2026-12-31").is_valid());
    // mês 99
}

#[test]
fn date_range_rejects_day_out_of_range() {
    assert!(!coherence::validate_date_range("d", "2026-01-00", "2026-12-31").is_valid()); // dia 0
    assert!(!coherence::validate_date_range("d", "2026-01-01", "2026-12-32").is_valid()); // dia 32
    assert!(!coherence::validate_date_range("d", "2026-01-99", "2026-12-31").is_valid());
    // dia 99
}

#[test]
fn datetime_range_rejects_hour_out_of_range() {
    assert!(!coherence::validate_datetime_range(
        "t",
        "2026-01-01T24:00:00Z",
        "2026-01-02T00:00:00Z"
    )
    .is_valid());
    assert!(!coherence::validate_datetime_range(
        "t",
        "2026-01-01T25:00:00Z",
        "2026-01-02T00:00:00Z"
    )
    .is_valid());
}

#[test]
fn datetime_range_rejects_minute_out_of_range() {
    assert!(!coherence::validate_datetime_range(
        "t",
        "2026-01-01T08:60:00Z",
        "2026-01-01T09:00:00Z"
    )
    .is_valid());
}

#[test]
fn datetime_range_accepts_leap_second() {
    // Segundo 60 é válido em RFC 3339 (leap second) — chrono aceita-o correctamente
    assert!(coherence::validate_datetime_range(
        "t",
        "2026-01-01T08:00:60Z",
        "2026-01-01T09:00:00Z"
    )
    .is_valid());
}

#[test]
fn date_range_accepts_boundary_values() {
    // mês 12, dia 31 — válidos
    assert!(coherence::validate_date_range("d", "2026-01-01", "2026-12-31").is_valid());
    // mês 01, dia 01 — válidos
    assert!(coherence::validate_date_range("d", "2026-01-01", "2026-01-01").is_valid());
}

#[test]
fn datetime_range_accepts_boundary_values() {
    // hora 23, minuto 59, segundo 59 — válidos
    assert!(coherence::validate_datetime_range(
        "t",
        "2026-01-01T00:00:00Z",
        "2026-12-31T23:59:59Z"
    )
    .is_valid());
}

#[test]
fn state_transition_allowed() {
    let allowed = [("rascunho", "em_revisao"), ("em_revisao", "aprovado")];
    assert!(
        coherence::validate_state_transition("estado", "rascunho", "em_revisao", &allowed)
            .is_valid()
    );
}

#[test]
fn state_transition_not_allowed() {
    let allowed = [("rascunho", "em_revisao"), ("em_revisao", "aprovado")];
    assert!(
        !coherence::validate_state_transition("estado", "rascunho", "aprovado", &allowed)
            .is_valid()
    );
}

#[test]
fn state_transition_empty_allowed_always_fails() {
    assert!(!coherence::validate_state_transition("estado", "a", "b", &[]).is_valid());
}

// ── Hash format ───────────────────────────────────────────────────────────────

#[test]
fn sha256_hex_valid() {
    assert!(hash_format::validate_sha256_hex(
        "payload_hash",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )
    .is_valid());
}

#[test]
fn sha256_hex_uppercase_is_invalid() {
    assert!(!hash_format::validate_sha256_hex(
        "payload_hash",
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
    )
    .is_valid());
}

#[test]
fn sha256_hex_wrong_length_is_invalid() {
    assert!(!hash_format::validate_sha256_hex("h", "ba7816bf").is_valid());
}

#[test]
fn sha256_hex_non_hex_chars_are_invalid() {
    let bad = "gg7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(!hash_format::validate_sha256_hex("h", bad).is_valid());
}

#[test]
fn sha256_hex_matches_sha256_bytes_output() {
    let hash = sha256_bytes(b"abc");
    assert!(hash_format::validate_sha256_hex("h", &hash).is_valid());
}

// ── ValidationIssue::info ────────────────────────────────────────────────────

#[test]
fn issue_info_is_valid_in_report() {
    let report = ValidationReport::with_issue(ValidationIssue::info(
        "validation.example",
        "field",
        "info message",
    ));
    assert!(report.is_valid());
    assert_eq!(report.issues[0].severity, ValidationSeverity::Info);
}

// ── CC (Cartão de Cidadão) ────────────────────────────────────────────────────
//
// CC de teste gerado manualmente:
//   Base: "12345678A" — 8 dígitos + série A
//   Algoritmo Luhn alfanumérico (pesos alternados, índices pares × 2):
//   i=0 (par):  '1'=1, 1*2=2,  reduce=2
//   i=1 (ímpar): '2'=2,        2
//   i=2 (par):  '3'=3, 3*2=6,  reduce=6
//   i=3 (ímpar): '4'=4,        4
//   i=4 (par):  '5'=5, 5*2=10, reduce=1+0=1
//   i=5 (ímpar): '6'=6,        6
//   i=6 (par):  '7'=7, 7*2=14, reduce=1+4=5
//   i=7 (ímpar): '8'=8,        8
//   i=8 (par):  'A'=10, 10*2=20, reduce=2+0=2
//   Soma = 2+2+6+4+1+6+5+8+2 = 36
//   controlo = (10 - (36 % 10)) % 10 = (10 - 6) % 10 = 4
//   CC completo: "12345678A4"

#[test]
fn cc_valid() {
    assert!(cc::validate_cc("cc", "12345678A4").is_valid());
}

#[test]
fn cc_valid_with_spaces() {
    assert!(cc::validate_cc("cc", "12345678 A 4").is_valid());
}

#[test]
fn cc_valid_with_hyphens() {
    assert!(cc::validate_cc("cc", "12345678-A-4").is_valid());
}

#[test]
fn cc_wrong_checksum_is_invalid() {
    assert!(!cc::validate_cc("cc", "12345678A5").is_valid());
    assert!(!cc::validate_cc("cc", "12345678A0").is_valid());
}

#[test]
fn cc_wrong_length_is_invalid() {
    assert!(!cc::validate_cc("cc", "1234567A4").is_valid()); // 9 chars
    assert!(!cc::validate_cc("cc", "123456789A4").is_valid()); // 11 chars
}

#[test]
fn cc_non_digit_in_first_8_is_invalid() {
    assert!(!cc::validate_cc("cc", "1234567XA4").is_valid());
}

#[test]
fn cc_lowercase_series_is_normalized_and_valid() {
    assert!(cc::validate_cc("cc", "12345678a4").is_valid());
}

#[test]
fn cc_digit_in_series_position_is_invalid() {
    // Position 9 must be a letter, not a digit
    assert!(!cc::validate_cc("cc", "1234567814").is_valid());
}

#[test]
fn cc_letter_in_check_position_is_invalid() {
    assert!(!cc::validate_cc("cc", "12345678AA").is_valid());
}

#[test]
fn cc_normalize_removes_spaces_and_hyphens() {
    let n = cc::normalize_cc("12345678 A 4");
    assert_eq!(n.normalized, "12345678A4");
    assert_eq!(n.original, "12345678 A 4");
}

// ── ManifestList ──────────────────────────────────────────────────────────────

#[test]
fn manifest_list_from_entries_sorts_by_path() {
    let a = ManifestEntry {
        path: "b/file.txt".into(),
        size: 1,
        sha256: sha256_bytes(b"b"),
    };
    let b = ManifestEntry {
        path: "a/file.txt".into(),
        size: 1,
        sha256: sha256_bytes(b"a"),
    };
    let list = ManifestList::from_entries(vec![a, b]);
    assert_eq!(list.entries[0].path, "a/file.txt");
    assert_eq!(list.entries[1].path, "b/file.txt");
}

#[test]
fn manifest_list_hash_is_deterministic() {
    let entry = ManifestEntry {
        path: "file.txt".into(),
        size: 3,
        sha256: sha256_bytes(b"abc"),
    };
    let list1 = ManifestList::from_entries(vec![entry.clone()]);
    let list2 = ManifestList::from_entries(vec![entry]);
    assert_eq!(list1.list_hash, list2.list_hash);
}

#[test]
fn manifest_list_hash_changes_with_different_entries() {
    let e1 = ManifestEntry {
        path: "a.txt".into(),
        size: 1,
        sha256: sha256_bytes(b"a"),
    };
    let e2 = ManifestEntry {
        path: "b.txt".into(),
        size: 1,
        sha256: sha256_bytes(b"b"),
    };
    let list1 = ManifestList::from_entries(vec![e1]);
    let list2 = ManifestList::from_entries(vec![e2]);
    assert_ne!(list1.list_hash, list2.list_hash);
}

#[test]
fn manifest_list_from_paths_produces_valid_list() {
    let dir = tempfile::tempdir().unwrap();
    let path_a = dir.path().join("a.txt");
    let path_b = dir.path().join("b.txt");
    std::fs::write(&path_a, b"hello").unwrap();
    std::fs::write(&path_b, b"world").unwrap();

    let list = ManifestList::from_paths([&path_a, &path_b]).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list.total_size(), 10);
    assert_eq!(list.list_hash.len(), 64); // SHA-256 hex
}

#[test]
fn manifest_list_empty_is_valid() {
    let list = ManifestList::from_entries(vec![]);
    assert!(list.is_empty());
    assert_eq!(list.total_size(), 0);
}

#[test]
fn crate_depends_on_chrono_for_datetime_comparison() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("chrono"));
}

// ── min_length ────────────────────────────────────────────────────────────────

#[test]
fn min_length_accepts_exact() {
    assert!(string::min_length("f", "abc", 3).is_valid());
}

#[test]
fn min_length_accepts_above() {
    assert!(string::min_length("f", "abcd", 3).is_valid());
}

#[test]
fn min_length_rejects_below() {
    assert!(!string::min_length("f", "ab", 3).is_valid());
}

#[test]
fn min_length_counts_unicode_chars_not_bytes() {
    // "ção" tem 3 chars mas 5 bytes em UTF-8
    assert!(string::min_length("f", "ção", 3).is_valid());
    assert!(!string::min_length("f", "ção", 4).is_valid());
}

// ── semver ────────────────────────────────────────────────────────────────────

#[test]
fn semver_valid_simple() {
    assert!(semver::validate_semver("v", "1.0.0").is_valid());
    assert!(semver::validate_semver("v", "0.3.0").is_valid());
    assert!(semver::validate_semver("v", "10.20.30").is_valid());
}

#[test]
fn semver_valid_with_prerelease() {
    assert!(semver::validate_semver("v", "1.0.0-rc.1").is_valid());
    assert!(semver::validate_semver("v", "2.0.0-alpha.1").is_valid());
}

#[test]
fn semver_valid_with_build() {
    assert!(semver::validate_semver("v", "1.0.0+build.42").is_valid());
}

#[test]
fn semver_valid_with_prerelease_and_build() {
    assert!(semver::validate_semver("v", "2.0.0-alpha.1+build.001").is_valid());
}

#[test]
fn semver_rejects_missing_patch() {
    assert!(!semver::validate_semver("v", "1.0").is_valid());
}

#[test]
fn semver_rejects_trailing_separator() {
    assert!(!semver::validate_semver("v", "1.0.0-").is_valid());
    assert!(!semver::validate_semver("v", "1.0.0+").is_valid());
}

#[test]
fn semver_rejects_non_numeric_core() {
    assert!(!semver::validate_semver("v", "1.a.0").is_valid());
}

#[test]
fn semver_rejects_empty() {
    assert!(!semver::validate_semver("v", "").is_valid());
}

// ── código postal PT ──────────────────────────────────────────────────────────

#[test]
fn cp_valid() {
    assert!(cp::validate_cp("cp", "1000-001").is_valid());
    assert!(cp::validate_cp("cp", "4000-007").is_valid());
}

#[test]
fn cp_valid_with_space_separator() {
    assert!(cp::validate_cp("cp", "1000 001").is_valid());
}

#[test]
fn cp_normalize_space_to_hyphen() {
    let n = cp::normalize_cp("1000 001");
    assert_eq!(n.normalized, "1000-001");
}

#[test]
fn cp_rejects_missing_hyphen() {
    assert!(!cp::validate_cp("cp", "1000001").is_valid());
}

#[test]
fn cp_rejects_wrong_length() {
    assert!(!cp::validate_cp("cp", "100-001").is_valid()); // 4 dígitos insuficientes
    assert!(!cp::validate_cp("cp", "10000-001").is_valid()); // 5 dígitos a mais
}

#[test]
fn cp_rejects_letters() {
    assert!(!cp::validate_cp("cp", "ABCD-001").is_valid());
}

// ── telefone PT ───────────────────────────────────────────────────────────────

#[test]
fn phone_pt_valid_mobile() {
    assert!(phone::validate_phone_pt("t", "912345678").is_valid()); // Vodafone
    assert!(phone::validate_phone_pt("t", "963456789").is_valid()); // MEO
}

#[test]
fn phone_pt_valid_fixed() {
    assert!(phone::validate_phone_pt("t", "213456789").is_valid()); // Lisboa
    assert!(phone::validate_phone_pt("t", "225678901").is_valid()); // Porto
}

#[test]
fn phone_pt_valid_with_international_prefix() {
    assert!(phone::validate_phone_pt("t", "+351912345678").is_valid());
    assert!(phone::validate_phone_pt("t", "00351912345678").is_valid());
}

#[test]
fn phone_pt_valid_with_spaces_and_hyphens() {
    assert!(phone::validate_phone_pt("t", "91 234 5678").is_valid());
    assert!(phone::validate_phone_pt("t", "21-345-6789").is_valid());
}

#[test]
fn phone_pt_normalize_strips_formatting() {
    let n = phone::normalize_phone_pt("+351 91 234 5678");
    assert_eq!(n.normalized, "912345678");
}

#[test]
fn phone_pt_rejects_invalid_prefix() {
    assert!(!phone::validate_phone_pt("t", "112345678").is_valid()); // 1xx — emergência
    assert!(!phone::validate_phone_pt("t", "012345678").is_valid()); // 0xx — operador
}

#[test]
fn phone_pt_rejects_wrong_length() {
    assert!(!phone::validate_phone_pt("t", "91234567").is_valid()); // 8 dígitos
    assert!(!phone::validate_phone_pt("t", "9123456789").is_valid()); // 10 dígitos
}

// ── MIME type ─────────────────────────────────────────────────────────────────

#[test]
fn mime_valid() {
    assert!(mime::validate_mime("m", "application/pdf").is_valid());
    assert!(mime::validate_mime("m", "text/plain").is_valid());
    assert!(mime::validate_mime("m", "image/svg+xml").is_valid());
    assert!(mime::validate_mime(
        "m",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )
    .is_valid());
}

#[test]
fn mime_rejects_missing_slash() {
    assert!(!mime::validate_mime("m", "applicationpdf").is_valid());
}

#[test]
fn mime_rejects_empty_type_or_subtype() {
    assert!(!mime::validate_mime("m", "/pdf").is_valid());
    assert!(!mime::validate_mime("m", "application/").is_valid());
}

#[test]
fn mime_rejects_spaces() {
    assert!(!mime::validate_mime("m", "application/ pdf").is_valid());
    assert!(!mime::validate_mime("m", "text/plain; charset=utf-8").is_valid());
}

#[test]
fn mime_rejects_invalid_chars_in_type() {
    assert!(!mime::validate_mime("m", "app_lication/pdf").is_valid());
}

// ── numeric range ─────────────────────────────────────────────────────────────

#[test]
fn range_valid_within_bounds() {
    assert!(range::validate_in_range("v", 50.0, 0.0, 100.0).is_valid());
    assert!(range::validate_in_range("v", 0.0, 0.0, 100.0).is_valid()); // limite inferior
    assert!(range::validate_in_range("v", 100.0, 0.0, 100.0).is_valid()); // limite superior
}

#[test]
fn range_rejects_below_min() {
    assert!(!range::validate_in_range("v", -0.1, 0.0, 100.0).is_valid());
}

#[test]
fn range_rejects_above_max() {
    assert!(!range::validate_in_range("v", 100.1, 0.0, 100.0).is_valid());
}

#[test]
fn range_rejects_nan() {
    assert!(!range::validate_in_range("v", f64::NAN, 0.0, 100.0).is_valid());
}

#[test]
fn range_rejects_infinity() {
    assert!(!range::validate_in_range("v", f64::INFINITY, 0.0, 100.0).is_valid());
    assert!(!range::validate_in_range("v", f64::NEG_INFINITY, 0.0, 100.0).is_valid());
}

#[test]
fn range_works_for_normalized_score() {
    assert!(range::validate_in_range("score", 0.75, 0.0, 1.0).is_valid());
    assert!(!range::validate_in_range("score", 1.1, 0.0, 1.0).is_valid());
}

#[test]
fn range_works_for_integers() {
    assert!(range::validate_in_range("n", 5.0, 1.0, 10.0).is_valid());
    assert!(!range::validate_in_range("n", 11.0, 1.0, 10.0).is_valid());
}

// ── Validação de dia por mês (com ano bissexto) ───────────────────────────────

#[test]
fn date_range_rejects_february_31() {
    assert!(!coherence::validate_date_range("d", "2026-02-31", "2026-03-01").is_valid());
}

#[test]
fn date_range_rejects_april_31() {
    assert!(!coherence::validate_date_range("d", "2026-04-31", "2026-05-01").is_valid());
}

#[test]
fn date_range_accepts_february_28_in_common_year() {
    assert!(coherence::validate_date_range("d", "2026-02-01", "2026-02-28").is_valid());
}

#[test]
fn date_range_rejects_february_29_in_common_year() {
    assert!(!coherence::validate_date_range("d", "2026-02-01", "2026-02-29").is_valid());
}

#[test]
fn date_range_accepts_february_29_in_leap_year() {
    assert!(coherence::validate_date_range("d", "2024-02-01", "2024-02-29").is_valid());
}

#[test]
fn date_range_rejects_february_30_in_leap_year() {
    assert!(!coherence::validate_date_range("d", "2024-02-01", "2024-02-30").is_valid());
}

// ── Semver zeros à esquerda e sufixo ─────────────────────────────────────────

#[test]
fn semver_rejects_leading_zeros_in_major() {
    assert!(!semver::validate_semver("v", "01.0.0").is_valid());
}

#[test]
fn semver_rejects_leading_zeros_in_minor() {
    assert!(!semver::validate_semver("v", "1.01.0").is_valid());
}

#[test]
fn semver_rejects_leading_zeros_in_patch() {
    assert!(!semver::validate_semver("v", "1.0.01").is_valid());
}

#[test]
fn semver_accepts_zero_components() {
    assert!(semver::validate_semver("v", "0.0.0").is_valid());
}

#[test]
fn semver_rejects_invalid_chars_in_prerelease() {
    assert!(!semver::validate_semver("v", "1.0.0-rc!1").is_valid());
}

#[test]
fn semver_rejects_empty_prerelease_segment() {
    assert!(!semver::validate_semver("v", "1.0.0-rc..1").is_valid());
}

#[test]
fn semver_accepts_hyphen_in_prerelease_identifier() {
    assert!(semver::validate_semver("v", "1.0.0-rc-final.1").is_valid());
}

#[test]
fn semver_rejects_numeric_prerelease_with_leading_zero() {
    // SemVer 2.0 §9: identificadores numéricos de prerelease não podem ter zeros à esquerda
    assert!(!semver::validate_semver("v", "1.0.0-01").is_valid());
    assert!(!semver::validate_semver("v", "1.0.0-rc.01").is_valid());
    assert!(!semver::validate_semver("v", "1.0.0-00").is_valid());
}

#[test]
fn semver_accepts_numeric_prerelease_zero() {
    // "0" é válido (único zero permitido)
    assert!(semver::validate_semver("v", "1.0.0-0").is_valid());
    assert!(semver::validate_semver("v", "1.0.0-rc.0").is_valid());
}

#[test]
fn semver_accepts_build_metadata_with_leading_zeros() {
    // Build metadata permite zeros à esquerda (SemVer 2.0 §10 não restringe)
    assert!(semver::validate_semver("v", "1.0.0+001").is_valid());
    assert!(semver::validate_semver("v", "2.0.0-alpha.1+build.001").is_valid());
}

#[test]
fn semver_accepts_alphanumeric_prerelease_with_leading_zero_char() {
    // "01a" não é puramente numérico — não se aplica a restrição de zeros
    assert!(semver::validate_semver("v", "1.0.0-01a").is_valid());
}

// ── validate_in_range bounds em release ──────────────────────────────────────

#[test]
fn range_rejects_nan_min() {
    assert!(!range::validate_in_range("v", 5.0, f64::NAN, 10.0).is_valid());
}

#[test]
fn range_rejects_infinite_max() {
    assert!(!range::validate_in_range("v", 5.0, 0.0, f64::INFINITY).is_valid());
}

#[test]
fn range_rejects_inverted_bounds() {
    assert!(!range::validate_in_range("v", 5.0, 10.0, 0.0).is_valid());
}

// ── UUID aceita qualquer versão ───────────────────────────────────────────────

#[test]
fn uuid_accepts_v4() {
    assert!(uuid::validate_uuid("id", "550e8400-e29b-41d4-a716-446655440000").is_valid());
}

#[test]
fn uuid_accepts_nil() {
    assert!(uuid::validate_uuid("id", "00000000-0000-0000-0000-000000000000").is_valid());
}

#[test]
fn uuid_accepts_v7_format() {
    // UUID v7 — time-ordered; aceito porque validate_uuid é agnostico de versão
    assert!(uuid::validate_uuid("id", "018f4a1c-0000-7000-8000-000000000000").is_valid());
}

use core_config::{
    AuditProfile, ConfigError, CryptoProfile, Environment, LoggingProfile, MiniKernelProfile,
    RuntimeProfile, StorageBackend, StoragePurpose, DUPLICATE_STORAGE_PROFILE,
    INCONSISTENT_PROFILE, INVALID_APP_PROFILE, INVALID_AUDIT_PROFILE, INVALID_CRYPTO_PROFILE,
    INVALID_LOGGING_PROFILE, INVALID_RUNTIME_PROFILE, INVALID_STORAGE_PROFILE, MALFORMED_JSON,
    MISSING_STORAGE_PROFILE,
};
use std::path::PathBuf;
use support_errors::MiniError;

fn valid_profile() -> MiniKernelProfile {
    MiniKernelProfile::dev_default(PathBuf::from("target/core-config-test"))
}

#[test]
fn dev_default_is_valid() {
    let profile = valid_profile();

    profile.validate().unwrap();
    assert_eq!(profile.storage.profiles.len(), 4);
    assert!(profile.storage.profile("main").is_some());
    assert!(profile.storage.profile("audit").is_some());
    assert!(profile.storage.profile("documents").is_some());
    assert!(profile.storage.profile("cache").is_some());
}

#[test]
fn test_memory_is_valid() {
    MiniKernelProfile::test_memory().validate().unwrap();
}

#[test]
fn empty_app_id_fails() {
    let mut profile = valid_profile();
    profile.app.app_id.clear();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn blank_display_name_fails() {
    let mut profile = valid_profile();
    profile.app.display_name = "   ".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn duplicate_storage_profile_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[1].name = "main".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::DuplicateStorageProfile { .. })
    ));
}

#[test]
fn missing_default_profile_fails() {
    let mut profile = valid_profile();
    profile.storage.default_profile = "missing".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::MissingStorageProfile { .. })
    ));
}

#[test]
fn sqlite_without_database_path_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].database_path = None;

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn sqlite_with_empty_database_path_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].database_path = Some(PathBuf::new());

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn memory_with_database_path_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.storage.profiles[0].database_path = Some(PathBuf::from("main.sqlite"));

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn encrypted_storage_without_crypto_fails() {
    let mut profile = valid_profile();
    profile.crypto = CryptoProfile {
        enabled: false,
        key_id: None,
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InconsistentProfile { .. })
    ));
}

#[test]
fn crypto_enabled_without_key_id_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: None,
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidCryptoProfile { .. })
    ));
}

#[test]
fn crypto_enabled_with_blank_key_id_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: Some("   ".to_owned()),
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidCryptoProfile { .. })
    ));
}

#[test]
fn missing_audit_storage_profile_fails() {
    let mut profile = valid_profile();
    profile.audit.storage_profile = "missing".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::MissingStorageProfile { .. })
    ));
}

#[test]
fn logging_enabled_without_log_dir_fails() {
    let mut profile = valid_profile();
    profile.logging = LoggingProfile {
        enabled: true,
        log_dir: None,
        ..LoggingProfile::default()
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn logging_enabled_with_empty_log_dir_fails() {
    let mut profile = valid_profile();
    profile.logging = LoggingProfile {
        enabled: true,
        log_dir: Some(PathBuf::new()),
        ..LoggingProfile::default()
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn logging_file_name_with_forward_slash_fails() {
    let mut profile = valid_profile();
    profile.logging.file_name = "logs/app.jsonl".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn logging_blank_file_name_fails() {
    let mut profile = valid_profile();
    profile.logging.file_name = "   ".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn namespace_with_colon_fails() {
    let mut profile = valid_profile();
    profile.audit = AuditProfile {
        namespace: "audit:events".to_owned(),
        ..AuditProfile::default()
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn audit_storage_profile_with_colon_fails() {
    let mut profile = valid_profile();
    profile.audit.storage_profile = "audit:events".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn other_storage_purpose_blank_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].purpose = StoragePurpose::Other("   ".to_owned());

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn config_error_converts_to_mini_error() {
    let err = ConfigError::InvalidAppProfile {
        reason: "app_id is required".to_owned(),
    };

    let mini: MiniError = err.clone().into();

    assert_eq!(mini.code.as_str(), err.code());
    assert_eq!(mini.component.as_str(), "core-config");
    assert_eq!(mini.message, err.public_message());
}

#[test]
fn crate_does_not_depend_on_sqlite() {
    let manifest = include_str!("../Cargo.toml").to_lowercase();

    assert!(!manifest.contains("rusqlite"));
    assert!(!manifest.contains("adapter-sqlite"));
}

// ── AppProfile ────────────────────────────────────────────────────────────────

#[test]
fn app_id_with_invalid_characters_fails() {
    let mut profile = valid_profile();
    profile.app.app_id = "invalid app id".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn app_id_with_slash_fails() {
    let mut profile = valid_profile();
    profile.app.app_id = "app/id".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn app_id_with_allowed_characters_passes() {
    let mut profile = valid_profile();
    profile.app.app_id = "normordis.miniapps-v1_dev".to_owned();

    assert!(profile.validate().is_ok());
}

#[test]
fn dev_default_environment_is_dev() {
    let profile = valid_profile();

    assert_eq!(profile.app.environment, Environment::Dev);
}

#[test]
fn test_memory_environment_is_test() {
    let profile = MiniKernelProfile::test_memory();

    assert_eq!(profile.app.environment, Environment::Test);
}

#[test]
fn prod_factory_is_valid() {
    let profile = MiniKernelProfile::prod(
        PathBuf::from("target/core-config-prod-test"),
        "prod-secret-key-2026",
    );

    profile.validate().unwrap();
}

#[test]
fn prod_environment_is_prod() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-key");

    assert_eq!(profile.app.environment, Environment::Prod);
}

#[test]
fn prod_is_online() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-key");

    assert!(!profile.runtime.offline_mode);
}

#[test]
fn prod_has_crypto_enabled_with_provided_key() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-prod-key");

    assert!(profile.crypto.enabled);
    assert_eq!(profile.crypto.key_id.as_deref(), Some("my-prod-key"));
}

#[test]
fn prod_all_storage_encrypted() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-key");

    assert!(profile.storage.profiles.iter().all(|p| p.encrypted));
}

#[test]
fn prod_has_logging_enabled() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-key");

    assert!(profile.logging.enabled);
    assert!(profile.logging.log_dir.is_some());
}

#[test]
fn prod_has_four_storage_profiles() {
    let profile = MiniKernelProfile::prod(PathBuf::from("target/test"), "my-key");

    assert_eq!(profile.storage.profiles.len(), 4);
    assert!(profile.storage.profile("main").is_some());
    assert!(profile.storage.profile("audit").is_some());
    assert!(profile.storage.profile("documents").is_some());
    assert!(profile.storage.profile("cache").is_some());
}

// ── RuntimeProfile ────────────────────────────────────────────────────────────

#[test]
fn runtime_profile_name_blank_fails() {
    let mut profile = valid_profile();
    profile.runtime = RuntimeProfile {
        profile_name: "   ".to_owned(),
        offline_mode: true,
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidRuntimeProfile { .. })
    ));
}

#[test]
fn runtime_profile_name_with_space_fails() {
    let mut profile = valid_profile();
    profile.runtime = RuntimeProfile {
        profile_name: "dev profile".to_owned(),
        offline_mode: true,
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidRuntimeProfile { .. })
    ));
}

// ── StorageProfiles ───────────────────────────────────────────────────────────

#[test]
fn storage_profiles_empty_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles.clear();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::MissingStorageProfile { .. })
    ));
}

#[test]
fn blank_default_profile_name_fails() {
    let mut profile = valid_profile();
    profile.storage.default_profile = "   ".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::MissingStorageProfile { .. })
    ));
}

#[test]
fn storage_profile_name_blank_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].name = "".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn storage_profile_name_with_space_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].name = "main db".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn storage_profile_name_with_colon_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].name = "main:db".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn other_storage_purpose_non_blank_is_valid() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].purpose = StoragePurpose::Other("analytics".to_owned());

    assert!(profile.validate().is_ok());
}

// ── CryptoProfile ─────────────────────────────────────────────────────────────

#[test]
fn crypto_key_id_with_colon_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: Some("key:id".to_owned()),
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidCryptoProfile { .. })
    ));
}

#[test]
fn crypto_key_id_with_space_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: Some("key id".to_owned()),
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidCryptoProfile { .. })
    ));
}

// ── LoggingProfile ────────────────────────────────────────────────────────────

#[test]
fn logging_max_file_size_zero_fails() {
    let mut profile = valid_profile();
    profile.logging.max_file_size_mb = 0;

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn logging_max_files_zero_fails() {
    let mut profile = valid_profile();
    profile.logging.max_files = 0;

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

#[test]
fn logging_retention_days_zero_fails() {
    let mut profile = valid_profile();
    profile.logging.retention_days = 0;

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidLoggingProfile { .. })
    ));
}

// ── AuditProfile ──────────────────────────────────────────────────────────────

#[test]
fn audit_namespace_blank_fails() {
    let mut profile = valid_profile();
    profile.audit = AuditProfile {
        namespace: "   ".to_owned(),
        ..AuditProfile::default()
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn audit_namespace_with_space_fails() {
    let mut profile = valid_profile();
    profile.audit = AuditProfile {
        namespace: "audit events".to_owned(),
        ..AuditProfile::default()
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn audit_storage_profile_blank_fails() {
    let mut profile = valid_profile();
    profile.audit.storage_profile = "   ".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn audit_disabled_skips_storage_profile_check() {
    let mut profile = valid_profile();
    profile.audit = AuditProfile {
        enabled: false,
        storage_profile: "non-existent-profile".to_owned(),
        ..AuditProfile::default()
    };

    assert!(profile.validate().is_ok());
}

// ── MiniKernelProfile::load_validated_from_json_str ──────────────────────────

#[test]
fn load_validated_profile_roundtrip() {
    let original = valid_profile();
    let json = serde_json::to_string(&original).unwrap();
    let loaded = MiniKernelProfile::load_validated_from_json_str(&json).unwrap();

    assert_eq!(loaded, original);
}

#[test]
fn load_validated_profile_rejects_invalid_json() {
    let err = MiniKernelProfile::load_validated_from_json_str("{bad json}").unwrap_err();

    assert!(matches!(err, ConfigError::MalformedJson { .. }));
}

#[test]
fn load_validated_profile_rejects_invalid_profile() {
    let mut profile = valid_profile();
    profile.app.app_id = String::new();
    let json = serde_json::to_string(&profile).unwrap();

    let err = MiniKernelProfile::load_validated_from_json_str(&json).unwrap_err();

    assert!(matches!(err, ConfigError::InvalidAppProfile { .. }));
}

// ── string length limits ──────────────────────────────────────────────────────

#[test]
fn app_id_at_max_length_passes() {
    let mut profile = valid_profile();
    profile.app.app_id = "a".repeat(128);

    assert!(profile.validate().is_ok());
}

#[test]
fn app_id_over_max_length_fails() {
    let mut profile = valid_profile();
    profile.app.app_id = "a".repeat(129);

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn display_name_over_max_length_fails() {
    let mut profile = valid_profile();
    profile.app.display_name = "a".repeat(256);

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn profile_name_over_max_length_fails() {
    let mut profile = valid_profile();
    profile.runtime = RuntimeProfile {
        profile_name: "a".repeat(65),
        offline_mode: true,
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidRuntimeProfile { .. })
    ));
}

#[test]
fn key_id_over_max_length_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: Some("k".repeat(129)),
    };

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidCryptoProfile { .. })
    ));
}

#[test]
fn storage_profile_name_over_max_length_fails() {
    let mut profile = valid_profile();
    profile.storage.profiles[0].name = "a".repeat(65);

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

#[test]
fn audit_namespace_over_max_length_fails() {
    let mut profile = valid_profile();
    profile.audit.namespace = "a".repeat(129);

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

// ── audit storage purpose invariant ──────────────────────────────────────────

#[test]
fn audit_storage_profile_purpose_must_be_audit() {
    let mut profile = valid_profile();
    // Point audit at "main" which has StoragePurpose::Main, not Audit
    profile.audit.storage_profile = "main".to_owned();

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidAuditProfile { .. })
    ));
}

#[test]
fn audit_storage_profile_with_correct_purpose_passes() {
    let profile = valid_profile();
    // dev_default() puts audit on a profile with StoragePurpose::Audit
    assert!(profile.validate().is_ok());
}

// ── logging disabled skips field validation ───────────────────────────────────

#[test]
fn logging_disabled_skips_field_validation() {
    let mut profile = valid_profile();
    profile.logging = LoggingProfile {
        enabled: false,
        log_dir: None,
        file_name: "   ".to_owned(),
        max_file_size_mb: 0,
        max_files: 0,
        retention_days: 0,
    };

    assert!(profile.validate().is_ok());
}

// ── memory + encrypted ────────────────────────────────────────────────────────

#[test]
fn memory_storage_encrypted_fails() {
    let mut profile = MiniKernelProfile::test_memory();
    profile.crypto = CryptoProfile {
        enabled: true,
        key_id: Some("test-key".to_owned()),
    };
    profile.storage.profiles[0].encrypted = true;

    assert!(matches!(
        profile.validate(),
        Err(ConfigError::InvalidStorageProfile { .. })
    ));
}

// ── ConfigError ───────────────────────────────────────────────────────────────

#[test]
fn all_error_codes_are_unique() {
    let codes = [
        INVALID_APP_PROFILE,
        INVALID_RUNTIME_PROFILE,
        INVALID_STORAGE_PROFILE,
        DUPLICATE_STORAGE_PROFILE,
        MISSING_STORAGE_PROFILE,
        INVALID_CRYPTO_PROFILE,
        INVALID_LOGGING_PROFILE,
        INVALID_AUDIT_PROFILE,
        INCONSISTENT_PROFILE,
        MALFORMED_JSON,
    ];

    let mut seen = std::collections::HashSet::new();
    for code in &codes {
        assert!(seen.insert(*code), "duplicate error code: {code}");
    }
}

#[test]
fn config_error_code_matches_variant() {
    let cases: &[(ConfigError, &str)] = &[
        (ConfigError::InvalidAppProfile { reason: "x".into() }, INVALID_APP_PROFILE),
        (ConfigError::InvalidRuntimeProfile { reason: "x".into() }, INVALID_RUNTIME_PROFILE),
        (ConfigError::InvalidStorageProfile { reason: "x".into() }, INVALID_STORAGE_PROFILE),
        (ConfigError::DuplicateStorageProfile { name: "x".into() }, DUPLICATE_STORAGE_PROFILE),
        (ConfigError::MissingStorageProfile { name: "x".into() }, MISSING_STORAGE_PROFILE),
        (ConfigError::InvalidCryptoProfile { reason: "x".into() }, INVALID_CRYPTO_PROFILE),
        (ConfigError::InvalidLoggingProfile { reason: "x".into() }, INVALID_LOGGING_PROFILE),
        (ConfigError::InvalidAuditProfile { reason: "x".into() }, INVALID_AUDIT_PROFILE),
        (ConfigError::InconsistentProfile { reason: "x".into() }, INCONSISTENT_PROFILE),
        (ConfigError::MalformedJson { reason: "x".into() }, MALFORMED_JSON),
    ];

    for (err, expected_code) in cases {
        assert_eq!(err.code(), *expected_code, "wrong code for {:?}", err);
    }
}

#[test]
fn every_error_has_non_empty_public_message() {
    let errors = [
        ConfigError::InvalidAppProfile { reason: "x".into() },
        ConfigError::InvalidRuntimeProfile { reason: "x".into() },
        ConfigError::InvalidStorageProfile { reason: "x".into() },
        ConfigError::DuplicateStorageProfile { name: "x".into() },
        ConfigError::MissingStorageProfile { name: "x".into() },
        ConfigError::InvalidCryptoProfile { reason: "x".into() },
        ConfigError::InvalidLoggingProfile { reason: "x".into() },
        ConfigError::InvalidAuditProfile { reason: "x".into() },
        ConfigError::InconsistentProfile { reason: "x".into() },
        ConfigError::MalformedJson { reason: "x".into() },
    ];

    for err in &errors {
        assert!(
            !err.public_message().is_empty(),
            "empty public_message for {:?}",
            err
        );
        let mini: MiniError = err.clone().into();
        assert_eq!(mini.message, err.public_message());
        assert_eq!(mini.code.as_str(), err.code());
    }
}

#[test]
fn crate_does_not_depend_on_tauri() {
    let manifest = include_str!("../Cargo.toml").to_lowercase();

    assert!(!manifest.contains("tauri"));
}

#[test]
fn storage_profile_helper_finds_by_name() {
    let profile = valid_profile();

    assert_eq!(
        profile.storage.profile("main").unwrap().backend,
        StorageBackend::Sqlite
    );
    assert!(profile.storage.profile("unknown").is_none());
}

#[test]
fn profile_serializes_and_deserializes_from_json() {
    let profile = valid_profile();
    let json = serde_json::to_string(&profile).unwrap();
    let decoded: MiniKernelProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, profile);
    decoded.validate().unwrap();
}

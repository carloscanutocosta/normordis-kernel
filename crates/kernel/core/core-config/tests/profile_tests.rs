use core_config::{
    AuditProfile, ConfigError, CryptoProfile, LoggingProfile, MiniKernelProfile, StorageBackend,
    StoragePurpose,
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

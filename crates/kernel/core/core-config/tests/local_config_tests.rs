use core_config::{
    app_config_to_json_string, load_app_config_from_json_str,
    load_validated_app_config_from_json_str, resolve_paths, validate_app_config, AppConfig,
    AppOptions, ConfigError, Environment, PathsConfig,
};
use std::path::PathBuf;

#[test]
fn default_app_config_is_valid() {
    let config = AppConfig::default();

    assert!(validate_app_config(&config).is_ok());
}

#[test]
fn resolves_paths_under_base_dir() {
    let config = AppConfig::default();
    let resolved = resolve_paths("/workspace", &config.paths);

    assert_eq!(
        resolved.database_dir,
        PathBuf::from("/workspace").join("database")
    );
    assert_eq!(
        resolved.data_dir,
        PathBuf::from("/workspace").join("assets")
    );
    assert_eq!(resolved.logs_dir, PathBuf::from("/workspace").join("logs"));
}

#[test]
fn serializes_and_loads_json_with_app_name() {
    let mut config = AppConfig::default();
    config.options.app_name = "miniapp-json".into();

    let json = app_config_to_json_string(&config).unwrap();
    let loaded = load_app_config_from_json_str(&json).unwrap();

    assert!(json.contains("\"app_name\": \"miniapp-json\""));
    assert_eq!(loaded, config);
}

// ── deny_unknown_fields ───────────────────────────────────────────────────────

#[test]
fn unknown_top_level_field_fails() {
    let json = r#"{"paths": {}, "options": {}, "extra_field": true}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

#[test]
fn unknown_paths_field_fails() {
    let json = r#"{"paths": {"database_dir": "db", "unknown_dir": "x"}}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

#[test]
fn unknown_options_field_fails() {
    let json = r#"{"options": {"app_name": "ok", "debug_mode": true}}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

#[test]
fn typo_in_path_field_fails() {
    // "data_directory" instead of "data_dir" — catches the most common config typo
    let json = r#"{"paths": {"data_directory": "assets"}}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

// ── load_app_config_from_json_str ─────────────────────────────────────────────

#[test]
fn invalid_json_fails() {
    let err = load_app_config_from_json_str("{not json}").unwrap_err();

    assert!(matches!(err, ConfigError::MalformedJson { .. }));
}

#[test]
fn empty_json_object_loads_defaults() {
    let config = load_app_config_from_json_str("{}").unwrap();

    assert_eq!(config, AppConfig::default());
}

#[test]
fn partial_json_loads_with_defaults() {
    let json = r#"{"options": {"app_name": "my-app"}}"#;
    let config = load_app_config_from_json_str(json).unwrap();

    assert_eq!(config.options.app_name, "my-app");
    assert_eq!(config.paths, PathsConfig::default());
}

// ── validate_app_config ───────────────────────────────────────────────────────

#[test]
fn empty_app_name_fails() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "".into(),
            ..AppOptions::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn blank_app_name_fails() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "   ".into(),
            ..AppOptions::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn empty_data_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            data_dir: PathBuf::new(),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn empty_database_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            database_dir: PathBuf::new(),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn empty_artifacts_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            artifacts_dir: PathBuf::new(),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn empty_temp_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            temp_dir: PathBuf::new(),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn empty_logs_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            logs_dir: PathBuf::new(),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

// ── AppOptions.environment ────────────────────────────────────────────────────

#[test]
fn default_options_environment_is_dev() {
    assert_eq!(AppOptions::default().environment, Environment::Dev);
}

#[test]
fn environment_serializes_as_lowercase() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "test".into(),
            environment: Environment::Prod,
        },
        ..AppConfig::default()
    };

    let json = app_config_to_json_string(&config).unwrap();

    assert!(json.contains("\"prod\""));
    assert!(!json.contains("\"Prod\""));
    assert!(!json.contains("\"local\""));
}

#[test]
fn environment_deserializes_from_lowercase() {
    let json = r#"{"options": {"app_name": "app", "environment": "test"}}"#;
    let config = load_app_config_from_json_str(json).unwrap();

    assert_eq!(config.options.environment, Environment::Test);
}

#[test]
fn environment_rejects_pascal_case_in_json() {
    let json = r#"{"options": {"app_name": "app", "environment": "Test"}}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

#[test]
fn invalid_environment_string_in_json_fails() {
    let json = r#"{"options": {"app_name": "app", "environment": "staging"}}"#;

    assert!(load_app_config_from_json_str(json).is_err());
}

// ── load_validated_app_config_from_json_str ───────────────────────────────────

#[test]
fn load_validated_accepts_valid_config() {
    let json = r#"{"options": {"app_name": "valid-app"}}"#;
    let config = load_validated_app_config_from_json_str(json).unwrap();

    assert_eq!(config.options.app_name, "valid-app");
}

#[test]
fn load_validated_rejects_invalid_json() {
    let err = load_validated_app_config_from_json_str("{bad}").unwrap_err();

    assert!(matches!(err, ConfigError::MalformedJson { .. }));
}

#[test]
fn load_validated_rejects_invalid_config() {
    let json = r#"{"options": {"app_name": ""}}"#;
    let err = load_validated_app_config_from_json_str(json).unwrap_err();

    assert!(matches!(err, ConfigError::InvalidAppProfile { .. }));
}

#[test]
fn load_validated_rejects_absolute_path_in_config() {
    let json = r#"{"paths": {"database_dir": "/etc/evil"}}"#;
    let err = load_validated_app_config_from_json_str(json).unwrap_err();

    assert!(matches!(err, ConfigError::InvalidAppProfile { .. }));
}

// ── PathsConfig defaults ──────────────────────────────────────────────────────

#[test]
fn artifacts_dir_default_differs_from_temp_dir() {
    let paths = PathsConfig::default();

    assert_ne!(paths.artifacts_dir, paths.temp_dir);
    assert_eq!(paths.artifacts_dir, PathBuf::from("artifacts"));
    assert_eq!(paths.temp_dir, PathBuf::from("tmp"));
}

// ── app_name length ───────────────────────────────────────────────────────────

#[test]
fn app_name_at_max_length_passes() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "a".repeat(128),
            ..AppOptions::default()
        },
        ..AppConfig::default()
    };

    assert!(validate_app_config(&config).is_ok());
}

#[test]
fn app_name_over_max_length_fails() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "a".repeat(129),
            ..AppOptions::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

// ── path traversal guards ─────────────────────────────────────────────────────

#[test]
fn dotdot_in_database_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            database_dir: PathBuf::from("../../etc"),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn dotdot_in_data_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            data_dir: PathBuf::from("subdir/../../../secret"),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[cfg(windows)]
#[test]
fn drive_relative_path_fails() {
    // "C:evil" has Component::Prefix but no RootDir — escapes has_root() on Windows
    let config = AppConfig {
        paths: PathsConfig {
            database_dir: std::path::PathBuf::from("C:evil"),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn absolute_database_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            database_dir: PathBuf::from("/etc/malicious"),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

#[test]
fn absolute_data_dir_fails() {
    let config = AppConfig {
        paths: PathsConfig {
            data_dir: PathBuf::from("/tmp/exfil"),
            ..PathsConfig::default()
        },
        ..AppConfig::default()
    };

    assert!(matches!(
        validate_app_config(&config),
        Err(ConfigError::InvalidAppProfile { .. })
    ));
}

// ── resolve_paths ─────────────────────────────────────────────────────────────

#[test]
fn resolve_paths_with_custom_subdirs() {
    let paths = PathsConfig {
        database_dir: PathBuf::from("db"),
        data_dir: PathBuf::from("files"),
        artifacts_dir: PathBuf::from("out"),
        temp_dir: PathBuf::from("tmp"),
        logs_dir: PathBuf::from("log"),
    };

    let resolved = resolve_paths("/base", &paths);

    assert_eq!(resolved.database_dir, PathBuf::from("/base/db"));
    assert_eq!(resolved.data_dir, PathBuf::from("/base/files"));
    assert_eq!(resolved.artifacts_dir, PathBuf::from("/base/out"));
    assert_eq!(resolved.temp_dir, PathBuf::from("/base/tmp"));
    assert_eq!(resolved.logs_dir, PathBuf::from("/base/log"));
}

#[test]
fn app_config_json_roundtrip_preserves_all_fields() {
    let config = AppConfig {
        options: AppOptions {
            app_name: "roundtrip-app".into(),
            environment: Environment::Prod,
        },
        paths: PathsConfig {
            database_dir: PathBuf::from("custom-db"),
            data_dir: PathBuf::from("custom-data"),
            artifacts_dir: PathBuf::from("custom-artifacts"),
            temp_dir: PathBuf::from("custom-tmp"),
            logs_dir: PathBuf::from("custom-logs"),
        },
    };

    let json = app_config_to_json_string(&config).unwrap();
    let loaded = load_app_config_from_json_str(&json).unwrap();

    assert_eq!(loaded, config);
}

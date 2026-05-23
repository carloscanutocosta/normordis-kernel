use core_config::{
    app_config_to_json_string, load_app_config_from_json_str, resolve_paths, validate_app_config,
    AppConfig,
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

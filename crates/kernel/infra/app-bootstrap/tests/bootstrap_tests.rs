use tempfile::tempdir;

use core_config::AppConfig;
use core_documental::{DocumentTypeCode, TemplateRepository};
use support_app_bootstrap::{
    bootstrap_local_app, load_from_json_file, save_to_json_file, BootstrapOptions,
};

#[test]
fn bootstrap_creates_directories_and_database() {
    let dir = tempdir().unwrap();
    let config = AppConfig::default();

    let runtime = bootstrap_local_app(
        dir.path(),
        config,
        BootstrapOptions {
            documental_db_file_name: "documental.db".to_string(),
            rh_db_file_name: "rh.db".to_string(),
            config_file_name: "app-config.json".to_string(),
        },
    )
    .unwrap();

    assert!(runtime.layout.data_dir.exists());
    assert!(runtime.layout.database_dir.exists());
    assert!(runtime.layout.artifacts_dir.exists());
    assert!(runtime.layout.temp_dir.exists());
    assert!(runtime.layout.logs_dir.exists());

    assert!(runtime.config_path.ends_with("app-config.json"));
    assert!(runtime.config_path.exists());

    assert!(runtime.documental_db_path.ends_with("documental.db"));
    assert!(runtime
        .documental_db_path
        .starts_with(&runtime.layout.database_dir));

    assert!(runtime.rh_db_path.ends_with("rh.db"));
    assert!(runtime.rh_db_path.starts_with(&runtime.layout.database_dir));

    assert!(runtime.release_notes_path.ends_with("release-notes.json"));
    assert!(runtime.release_notes_path.exists());

    // Verifica que o documental_store está funcional e vazio.
    let doc_type = DocumentTypeCode::new("any-type").unwrap();
    let tpl = runtime
        .documental_store
        .get_active_for_type(&doc_type)
        .unwrap();
    assert!(tpl.is_none());

    assert!(runtime.users_store.list_users().unwrap().is_empty());

    assert_eq!(
        load_from_json_file(&runtime.config_path).unwrap(),
        runtime.config
    );
}

#[test]
fn bootstrap_rejects_empty_documental_db_file_name() {
    let dir = tempdir().unwrap();
    let config = AppConfig::default();

    let err = bootstrap_local_app(
        dir.path(),
        config,
        BootstrapOptions {
            documental_db_file_name: "   ".to_string(),
            rh_db_file_name: "rh.db".to_string(),
            config_file_name: "app-config.json".to_string(),
        },
    )
    .unwrap_err();

    assert!(err.to_string().contains("base de dados"));
}

#[test]
fn bootstrap_rejects_empty_rh_db_file_name() {
    let dir = tempdir().unwrap();
    let config = AppConfig::default();

    let err = bootstrap_local_app(
        dir.path(),
        config,
        BootstrapOptions {
            documental_db_file_name: "documental.db".to_string(),
            rh_db_file_name: "  ".to_string(),
            config_file_name: "app-config.json".to_string(),
        },
    )
    .unwrap_err();

    assert!(err.to_string().contains("base de dados"));
}

#[test]
fn bootstrap_uses_existing_json_config() {
    let dir = tempdir().unwrap();
    let mut config = AppConfig::default();
    config.options.app_name = "custom-app".into();
    config.paths.data_dir = "Dados".into();
    let config_path = dir.path().join("app-config.json");
    save_to_json_file(&config, &config_path).unwrap();

    let runtime = bootstrap_local_app(
        dir.path(),
        AppConfig::default(),
        BootstrapOptions {
            documental_db_file_name: "documental.db".to_string(),
            rh_db_file_name: "rh.db".to_string(),
            config_file_name: "app-config.json".to_string(),
        },
    )
    .unwrap();

    assert_eq!(runtime.config.options.app_name, "custom-app");
    assert_eq!(
        runtime.config.paths.database_dir,
        std::path::PathBuf::from("database")
    );
    assert!(runtime.layout.data_dir.ends_with("Dados"));
}

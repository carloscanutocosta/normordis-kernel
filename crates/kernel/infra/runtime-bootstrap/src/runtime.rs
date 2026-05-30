use std::path::PathBuf;

use core_config::{ConfigError, LoggingProfile, MiniKernelProfile, StorageBackend, StorageProfile};
use support_crypto::{KeyProvider, KeyResolver};
use support_errors::MiniError;
use support_logging::{FileLogger, LoggingConfig, TechnicalLogger};

use crate::audit::{AuditDbConfig, AuditDbRuntime, AuditDbService};
use crate::error::{RuntimeError, RUNTIME_COMPONENT};

pub struct KernelRuntime {
    audit: AuditDbRuntime,
    logger: Option<FileLogger>,
}

impl KernelRuntime {
    pub fn open<P: KeyProvider + KeyResolver + Send + Sync>(
        profile: &MiniKernelProfile,
        keys: P,
    ) -> Result<Self, MiniError> {
        profile.validate().map_err(runtime_error_from_config)?;

        let logger = open_logger(&profile.logging)?;
        if let Some(logger) = &logger {
            logger.info(RUNTIME_COMPONENT, "opening kernel runtime");
        }

        let audit_storage_profile = resolve_audit_storage_profile(profile)?;
        let db_path = resolve_audit_db_path(audit_storage_profile)?;
        let audit_config = AuditDbConfig::new(db_path);
        let audit = AuditDbRuntime::open(audit_config, keys)
            .map_err(|_| RuntimeError::AuditRuntimeFailed)?;

        if let Some(logger) = &logger {
            logger.info(RUNTIME_COMPONENT, "kernel runtime opened");
        }

        Ok(Self { audit, logger })
    }

    pub fn audit(&self) -> &AuditDbService {
        self.audit.service()
    }

    pub fn logger(&self) -> Option<&dyn TechnicalLogger> {
        self.logger
            .as_ref()
            .map(|logger| logger as &dyn TechnicalLogger)
    }

    pub fn shutdown(&self) -> Result<(), MiniError> {
        if let Some(logger) = &self.logger {
            logger.info(RUNTIME_COMPONENT, "shutting down kernel runtime");
        }
        self.audit.shutdown()
    }
}

fn runtime_error_from_config(error: ConfigError) -> RuntimeError {
    match error {
        ConfigError::InvalidStorageProfile { .. }
        | ConfigError::MissingStorageProfile { .. }
        | ConfigError::InvalidAuditProfile { .. } => RuntimeError::InvalidStorageProfile,
        ConfigError::InvalidLoggingProfile { .. } => RuntimeError::LoggingRuntimeFailed,
        _ => RuntimeError::RuntimeOpenFailed,
    }
}

fn resolve_audit_storage_profile(
    profile: &MiniKernelProfile,
) -> Result<&StorageProfile, RuntimeError> {
    profile
        .storage
        .profile(&profile.audit.storage_profile)
        .ok_or(RuntimeError::InvalidStorageProfile)
}

fn resolve_audit_db_path(storage: &StorageProfile) -> Result<PathBuf, RuntimeError> {
    match storage.backend {
        StorageBackend::Sqlite => storage
            .database_path
            .clone()
            .ok_or(RuntimeError::InvalidStorageProfile),
        StorageBackend::Memory => Ok(PathBuf::from(":memory:")),
    }
}

fn open_logger(profile: &LoggingProfile) -> Result<Option<FileLogger>, RuntimeError> {
    if !profile.enabled {
        return Ok(None);
    }

    let log_dir = profile
        .log_dir
        .as_ref()
        .ok_or(RuntimeError::LoggingRuntimeFailed)?;
    let mut config = LoggingConfig::new(log_dir, &profile.file_name);
    config.max_file_size_mb = profile.max_file_size_mb;
    config.max_files = profile.max_files;
    config.retention_days = profile.retention_days;

    FileLogger::new(config)
        .map(Some)
        .map_err(|_| RuntimeError::LoggingRuntimeFailed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use core_audit::{AuditActor, AuditTarget};
    use core_config::{MiniKernelProfile, StorageBackend, StorageProfile, StoragePurpose};
    use serde_json::json;
    use support_crypto::{KeyId, SecretKey, StaticKeyProvider};
    use tempfile::tempdir;

    use super::*;
    use crate::{
        AUDIT_RUNTIME_FAILED, INVALID_STORAGE_PROFILE, LOGGING_RUNTIME_FAILED,
        UNSUPPORTED_STORAGE_BACKEND,
    };

    fn keys() -> StaticKeyProvider {
        StaticKeyProvider::new(
            SecretKey::new([41; support_crypto::KEY_LENGTH_BYTES]),
            Some(KeyId::new("runtime-test-key").unwrap()),
        )
    }

    fn record_one(runtime: &KernelRuntime) -> String {
        runtime
            .audit()
            .record_event(
                "runtime.opened",
                AuditActor::new("system").unwrap(),
                AuditTarget::new("runtime", "kernel").unwrap(),
                Some(json!({"ok": true})),
            )
            .unwrap()
            .event_id
    }

    #[test]
    fn kernel_runtime_opens_with_sqlite_profile() {
        let dir = tempdir().unwrap();
        let profile = MiniKernelProfile::dev_default(dir.path());

        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        record_one(&runtime);
        assert!(dir.path().join("audit.sqlite").exists());
        runtime.shutdown().unwrap();
    }

    #[test]
    fn kernel_runtime_opens_with_memory_profile() {
        let profile = MiniKernelProfile::test_memory();

        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        let event_id = record_one(&runtime);
        assert!(runtime.audit().get(&event_id).unwrap().is_some());
        runtime.shutdown().unwrap();
    }

    #[test]
    fn auditable_events_are_persisted_in_sqlite() {
        let dir = tempdir().unwrap();
        let profile = MiniKernelProfile::dev_default(dir.path());
        let event_id = {
            let runtime = KernelRuntime::open(&profile, keys()).unwrap();
            let event_id = record_one(&runtime);
            runtime.shutdown().unwrap();
            event_id
        };

        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        assert!(runtime.audit().get(&event_id).unwrap().is_some());
        runtime.shutdown().unwrap();
    }

    #[test]
    fn shutdown_is_idempotent_for_memory_runtime() {
        let profile = MiniKernelProfile::test_memory();
        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        runtime.shutdown().unwrap();
        runtime.shutdown().unwrap();
    }

    #[test]
    fn shutdown_is_idempotent_for_sqlite_runtime() {
        let dir = tempdir().unwrap();
        let profile = MiniKernelProfile::dev_default(dir.path());
        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        record_one(&runtime);
        runtime.shutdown().unwrap();
        runtime.shutdown().unwrap();
    }

    #[test]
    fn sqlite_runtime_rejects_writes_after_shutdown() {
        let dir = tempdir().unwrap();
        let profile = MiniKernelProfile::dev_default(dir.path());
        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        runtime.shutdown().unwrap();
        let err = runtime
            .audit()
            .record_event(
                "runtime.after_shutdown",
                AuditActor::new("system").unwrap(),
                AuditTarget::new("runtime", "kernel").unwrap(),
                None,
            )
            .unwrap_err();

        assert_eq!(err, core_audit::AuditError::StoreFailed);
    }

    #[test]
    fn sqlite_open_failure_maps_to_audit_runtime_error() {
        let dir = tempdir().unwrap();
        let mut profile = MiniKernelProfile::dev_default(dir.path());
        fs::write(dir.path().join("not-a-dir"), "occupied").unwrap();
        profile
            .storage
            .profiles
            .iter_mut()
            .find(|storage| storage.name == "audit")
            .unwrap()
            .database_path = Some(dir.path().join("not-a-dir").join("audit.sqlite"));

        let err = match KernelRuntime::open(&profile, keys()) {
            Ok(_) => panic!("runtime open should fail"),
            Err(err) => err,
        };

        assert_eq!(err.to_public().code, AUDIT_RUNTIME_FAILED);
    }

    #[test]
    fn missing_storage_profile_fails_with_runtime_error() {
        let mut profile = MiniKernelProfile::test_memory();
        profile.audit.storage_profile = "missing".to_owned();

        let err = match KernelRuntime::open(&profile, keys()) {
            Ok(_) => panic!("runtime open should fail"),
            Err(err) => err,
        };

        assert_eq!(err.to_public().code, INVALID_STORAGE_PROFILE);
    }

    #[test]
    fn unsupported_storage_backend_error_is_publicly_registered() {
        let err = RuntimeError::UnsupportedStorageBackend.to_mini_error();

        assert_eq!(err.to_public().code, UNSUPPORTED_STORAGE_BACKEND);
    }

    #[test]
    fn invalid_logging_profile_fails_with_runtime_error() {
        let dir = tempdir().unwrap();
        let mut profile = MiniKernelProfile::dev_default(dir.path());
        profile.logging.file_name = "nested/app.log".to_owned();

        let err = match KernelRuntime::open(&profile, keys()) {
            Ok(_) => panic!("runtime open should fail"),
            Err(err) => err,
        };

        assert_eq!(err.to_public().code, LOGGING_RUNTIME_FAILED);
    }

    #[test]
    fn manifest_does_not_depend_on_tauri() {
        let manifest =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();

        assert!(!manifest.contains(&format!("tau{}", "ri")));
    }

    #[test]
    fn manifest_does_not_depend_on_ui_or_async_runtime() {
        let manifest =
            fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();

        assert!(!manifest.contains("ui"));
        assert!(!manifest.contains("tokio"));
        assert!(!manifest.contains("async-std"));
    }

    #[test]
    fn runtime_uses_configured_audit_storage_profile_among_many_profiles() {
        let dir = tempdir().unwrap();
        let mut profile = MiniKernelProfile::dev_default(dir.path());
        profile.storage.profiles.push(StorageProfile {
            name: "audit-alt".to_owned(),
            backend: StorageBackend::Sqlite,
            database_path: Some(dir.path().join("audit-alt.sqlite")),
            encrypted: true,
            purpose: StoragePurpose::Audit,
        });
        profile.audit.storage_profile = "audit-alt".to_owned();

        let runtime = KernelRuntime::open(&profile, keys()).unwrap();
        record_one(&runtime);
        runtime.shutdown().unwrap();

        assert!(dir.path().join("audit-alt.sqlite").exists());
        assert!(!dir.path().join("audit.sqlite").exists());
    }

    #[test]
    fn logger_is_exposed_when_logging_profile_is_enabled() {
        let dir = tempdir().unwrap();
        let profile = MiniKernelProfile::dev_default(dir.path());

        let runtime = KernelRuntime::open(&profile, keys()).unwrap();

        assert!(runtime.logger().is_some());
        runtime.shutdown().unwrap();
    }
}

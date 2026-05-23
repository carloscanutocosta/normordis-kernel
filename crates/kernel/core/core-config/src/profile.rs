use crate::{
    AppProfile, AuditProfile, ConfigError, CryptoProfile, Environment, LoggingProfile,
    RuntimeProfile, StorageBackend, StorageProfile, StorageProfiles, StoragePurpose,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiniKernelProfile {
    pub app: AppProfile,
    pub runtime: RuntimeProfile,
    pub storage: StorageProfiles,
    pub crypto: CryptoProfile,
    pub logging: LoggingProfile,
    pub audit: AuditProfile,
}

impl MiniKernelProfile {
    pub fn validate(&self) -> Result<(), ConfigError> {
        crate::validate::validate_profile(self)
    }

    pub fn dev_default(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        let sqlite_profile = |name: &str, file_name: &str, purpose| StorageProfile {
            name: name.to_owned(),
            backend: StorageBackend::Sqlite,
            database_path: Some(base_dir.join(file_name)),
            encrypted: true,
            purpose,
        };

        Self {
            app: AppProfile {
                app_id: "normordis.miniapps.dev".to_owned(),
                display_name: "Normordis Miniapps".to_owned(),
                environment: Environment::Dev,
            },
            runtime: RuntimeProfile {
                profile_name: "dev".to_owned(),
                offline_mode: true,
            },
            storage: StorageProfiles {
                default_profile: "main".to_owned(),
                profiles: vec![
                    sqlite_profile("main", "main.sqlite", StoragePurpose::Main),
                    sqlite_profile("audit", "audit.sqlite", StoragePurpose::Audit),
                    sqlite_profile("documents", "documents.sqlite", StoragePurpose::Documents),
                    sqlite_profile("cache", "cache.sqlite", StoragePurpose::Cache),
                ],
            },
            crypto: CryptoProfile {
                enabled: true,
                key_id: Some("dev-local-key".to_owned()),
            },
            logging: LoggingProfile {
                enabled: true,
                log_dir: Some(base_dir.join("logs")),
                ..LoggingProfile::default()
            },
            audit: AuditProfile::default(),
        }
    }

    pub fn test_memory() -> Self {
        Self {
            app: AppProfile {
                app_id: "normordis.miniapps.test".to_owned(),
                display_name: "Normordis Miniapps Test".to_owned(),
                environment: Environment::Test,
            },
            runtime: RuntimeProfile {
                profile_name: "test".to_owned(),
                offline_mode: true,
            },
            storage: StorageProfiles {
                default_profile: "main".to_owned(),
                profiles: vec![
                    StorageProfile {
                        name: "main".to_owned(),
                        backend: StorageBackend::Memory,
                        database_path: None,
                        encrypted: false,
                        purpose: StoragePurpose::Main,
                    },
                    StorageProfile {
                        name: "audit".to_owned(),
                        backend: StorageBackend::Memory,
                        database_path: None,
                        encrypted: false,
                        purpose: StoragePurpose::Audit,
                    },
                ],
            },
            crypto: CryptoProfile {
                enabled: false,
                key_id: None,
            },
            logging: LoggingProfile::default(),
            audit: AuditProfile::default(),
        }
    }
}

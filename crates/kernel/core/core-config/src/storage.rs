use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProfiles {
    pub default_profile: String,
    pub profiles: Vec<StorageProfile>,
}

impl StorageProfiles {
    pub fn profile(&self, name: &str) -> Option<&StorageProfile> {
        self.profiles.iter().find(|profile| profile.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProfile {
    pub name: String,
    pub backend: StorageBackend,
    pub database_path: Option<PathBuf>,
    pub encrypted: bool,
    pub purpose: StoragePurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    Memory,
    Sqlite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum StoragePurpose {
    Main,
    Audit,
    Documents,
    Cache,
    Temp,
    Other(String),
}

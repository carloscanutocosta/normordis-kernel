use serde::{Deserialize, Serialize};

pub const DEFAULT_AUDIT_NAMESPACE: &str = "audit.events";
pub const DEFAULT_AUDIT_STORAGE_PROFILE: &str = "audit";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditProfile {
    pub enabled: bool,
    pub namespace: String,
    pub storage_profile: String,
}

impl Default for AuditProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            namespace: DEFAULT_AUDIT_NAMESPACE.to_owned(),
            storage_profile: DEFAULT_AUDIT_STORAGE_PROFILE.to_owned(),
        }
    }
}

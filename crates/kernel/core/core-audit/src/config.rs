use support_storage::StorageNamespace;

pub const DEFAULT_AUDIT_EVENTS_NAMESPACE: &str = "audit.events";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditStoreConfig {
    pub namespace: StorageNamespace,
}

impl AuditStoreConfig {
    pub fn new(namespace: StorageNamespace) -> Self {
        Self { namespace }
    }
}

impl Default for AuditStoreConfig {
    fn default() -> Self {
        Self {
            namespace: StorageNamespace::new(DEFAULT_AUDIT_EVENTS_NAMESPACE)
                .expect("default audit namespace must be valid"),
        }
    }
}

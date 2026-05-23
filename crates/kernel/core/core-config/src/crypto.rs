use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoProfile {
    pub enabled: bool,
    pub key_id: Option<String>,
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TechnicalId(String);

impl TechnicalId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn new_id() -> TechnicalId {
    TechnicalId(Uuid::new_v4().to_string())
}

pub fn new_id_string() -> String {
    new_id().0
}

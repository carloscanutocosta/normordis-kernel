use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::namespace::validate_segment;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageKey(String);

impl StorageKey {
    pub const MAX_LEN: usize = 256;

    pub fn new(value: impl Into<String>) -> Result<Self, StorageError> {
        let value = value.into();
        validate_segment(&value, Self::MAX_LEN, true).map_err(|_| StorageError::InvalidKey)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_key() {
        let key = StorageKey::new("doc-1.value").unwrap();

        assert_eq!(key.as_str(), "doc-1.value");
    }

    #[test]
    fn rejects_invalid_key() {
        for value in ["", "doc 1", "doc/1", "doc\\1", "../doc", "doc:1"] {
            assert_eq!(
                StorageKey::new(value).unwrap_err(),
                StorageError::InvalidKey
            );
        }
    }
}

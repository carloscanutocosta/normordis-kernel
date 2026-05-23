use serde::{Deserialize, Serialize};

use crate::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageNamespace(String);

impl StorageNamespace {
    pub const MAX_LEN: usize = 128;

    pub fn new(value: impl Into<String>) -> Result<Self, StorageError> {
        let value = value.into();
        validate_segment(&value, Self::MAX_LEN, false)
            .map_err(|_| StorageError::InvalidNamespace)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn validate_segment(
    value: &str,
    max_len: usize,
    reject_path_tokens: bool,
) -> Result<(), ()> {
    if value.is_empty()
        || value.chars().count() > max_len
        || value.chars().any(char::is_whitespace)
        || value.contains(':')
    {
        return Err(());
    }

    if reject_path_tokens && (value.contains('/') || value.contains('\\') || value.contains("..")) {
        return Err(());
    }

    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_namespace() {
        let namespace = StorageNamespace::new("core.documents-v1").unwrap();

        assert_eq!(namespace.as_str(), "core.documents-v1");
    }

    #[test]
    fn rejects_invalid_namespace() {
        assert_eq!(
            StorageNamespace::new("").unwrap_err(),
            StorageError::InvalidNamespace
        );
        assert_eq!(
            StorageNamespace::new("core documents").unwrap_err(),
            StorageError::InvalidNamespace
        );
        assert_eq!(
            StorageNamespace::new("core:documents").unwrap_err(),
            StorageError::InvalidNamespace
        );
        assert_eq!(
            StorageNamespace::new("core/documents").unwrap_err(),
            StorageError::InvalidNamespace
        );
    }
}

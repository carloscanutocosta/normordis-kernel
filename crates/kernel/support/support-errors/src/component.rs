use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Component(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ComponentError {
    #[error("component cannot be empty")]
    Empty,
    #[error("component cannot contain spaces")]
    ContainsSpaces,
}

impl Component {
    pub fn new(value: impl Into<String>) -> Result<Self, ComponentError> {
        let value = value.into();

        if value.is_empty() {
            return Err(ComponentError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(ComponentError::ContainsSpaces);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_component() {
        let component = Component::new("adapter-sqlite").unwrap();

        assert_eq!(component.as_str(), "adapter-sqlite");
    }

    #[test]
    fn rejects_invalid_deserialized_component() {
        let err = serde_json::from_str::<Component>(r#""adapter sqlite""#).unwrap_err();

        assert!(err.to_string().contains("spaces"));
    }
}

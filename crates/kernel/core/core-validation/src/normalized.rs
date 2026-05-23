use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Normalized<T> {
    pub original: String,
    pub normalized: T,
}

impl<T> Normalized<T> {
    pub fn new(original: impl Into<String>, normalized: T) -> Self {
        Self {
            original: original.into(),
            normalized,
        }
    }
}

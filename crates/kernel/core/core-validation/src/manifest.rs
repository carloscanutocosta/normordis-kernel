use crate::{sha256_file, ValidationError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

pub fn manifest_file(path: impl AsRef<Path>) -> Result<ManifestEntry, ValidationError> {
    let path = path.as_ref();
    let metadata = path.metadata().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ValidationError::FileNotFound
        } else {
            ValidationError::ManifestFailed
        }
    })?;

    if !metadata.is_file() {
        return Err(ValidationError::NotRegularFile);
    }

    Ok(ManifestEntry {
        path: normalize_manifest_path(path),
        size: metadata.len(),
        sha256: sha256_file(path).map_err(|err| match err {
            ValidationError::FileNotFound => ValidationError::FileNotFound,
            ValidationError::NotRegularFile => ValidationError::NotRegularFile,
            ValidationError::FileReadFailed => ValidationError::FileReadFailed,
            _ => ValidationError::ManifestFailed,
        })?,
    })
}

fn normalize_manifest_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

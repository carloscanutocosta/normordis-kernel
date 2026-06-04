use crate::{sha256_bytes, sha256_file, ValidationError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// Manifesto de um pacote multi-ficheiro.
///
/// Agrega as entradas individuais e calcula um hash de lista (`list_hash`)
/// sobre a representação canónica JSON do array de entradas, ordenadas por `path`.
/// Isto garante que o manifesto é determinístico independentemente da ordem de inserção.
///
/// O `list_hash` pode ser usado como identificador de integridade do pacote completo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestList {
    pub entries: Vec<ManifestEntry>,
    /// SHA-256 da representação JSON canónica das entradas ordenadas por path.
    pub list_hash: String,
}

impl ManifestList {
    /// Constrói um `ManifestList` a partir de entradas já calculadas.
    ///
    /// Ordena as entradas por `path` antes de calcular o `list_hash`.
    pub fn from_entries(mut entries: Vec<ManifestEntry>) -> Self {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let list_hash = compute_list_hash(&entries);
        Self { entries, list_hash }
    }

    /// Constrói um `ManifestList` a partir de paths de ficheiros.
    pub fn from_paths<P: AsRef<Path>>(
        paths: impl IntoIterator<Item = P>,
    ) -> Result<Self, ValidationError> {
        let entries: Result<Vec<ManifestEntry>, ValidationError> =
            paths.into_iter().map(|p| manifest_file(p)).collect();
        Ok(Self::from_entries(entries?))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }
}

pub fn manifest_file(path: impl AsRef<Path>) -> Result<ManifestEntry, ValidationError> {
    let path = path.as_ref();
    let metadata = path.symlink_metadata().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ValidationError::FileNotFound
        } else {
            ValidationError::ManifestFailed
        }
    })?;

    if metadata_is_unsafe(&metadata) {
        return Err(ValidationError::UnsafeFileType);
    }

    if !metadata.is_file() {
        return Err(ValidationError::NotRegularFile);
    }

    Ok(ManifestEntry {
        path: normalize_manifest_path(path)?,
        size: metadata.len(),
        sha256: sha256_file(path).map_err(|err| match err {
            ValidationError::FileNotFound => ValidationError::FileNotFound,
            ValidationError::NotRegularFile => ValidationError::NotRegularFile,
            ValidationError::UnsafeFileType => ValidationError::UnsafeFileType,
            ValidationError::InvalidPathEncoding => ValidationError::InvalidPathEncoding,
            ValidationError::FileReadFailed => ValidationError::FileReadFailed,
            _ => ValidationError::ManifestFailed,
        })?,
    })
}

fn normalize_manifest_path(path: &Path) -> Result<String, ValidationError> {
    path.to_str()
        .ok_or(ValidationError::InvalidPathEncoding)
        .map(|path| path.replace('\\', "/"))
}

fn compute_list_hash(entries: &[ManifestEntry]) -> String {
    let canonical = serde_json::to_vec(entries)
        .expect("ManifestEntry only contains String and u64 — always JSON-serializable");
    sha256_bytes(&canonical)
}

fn metadata_is_unsafe(metadata: &std::fs::Metadata) -> bool {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    false
}

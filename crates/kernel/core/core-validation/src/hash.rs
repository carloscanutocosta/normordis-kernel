use crate::ValidationError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn sha256_bytes(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String, ValidationError> {
    let path = path.as_ref();
    let metadata = path.symlink_metadata().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ValidationError::FileNotFound
        } else {
            ValidationError::FileReadFailed
        }
    })?;

    if metadata_is_unsafe(&metadata) {
        return Err(ValidationError::UnsafeFileType);
    }

    if !metadata.is_file() {
        return Err(ValidationError::NotRegularFile);
    }

    let file = File::open(path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ValidationError::FileNotFound
        } else {
            ValidationError::FileReadFailed
        }
    })?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0_u8; 8192];
    let mut hasher = Sha256::new();

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| ValidationError::FileReadFailed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
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

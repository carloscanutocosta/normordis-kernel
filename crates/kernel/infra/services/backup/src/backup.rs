use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use tar::{Archive, Builder};

use crate::error::BackupServiceError;

/// Cria uma cópia limpa do banco via VACUUM INTO no diretório de staging.
/// Retorna o caminho do ficheiro copiado.
pub async fn vacuum_into(db_path: &str, staging_dir: &Path) -> Result<PathBuf, BackupServiceError> {
    let db_path = db_path.to_string();
    let staging_dir = staging_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let src = Path::new(&db_path);
        let db_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| BackupServiceError::IoFailed("invalid db path".into()))?;

        let dest = staging_dir.join(db_name);
        let conn = rusqlite::Connection::open(src)
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

        conn.execute_batch(&format!(
            "VACUUM INTO '{}'",
            dest.to_string_lossy().replace('\'', "''")
        ))
        .map_err(|e| BackupServiceError::ArchiveFailed(format!("VACUUM INTO failed: {e}")))?;

        Ok(dest)
    })
    .await
    .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?
}

/// Cria um arquivo tar.gz cifrado a partir do diretório de staging.
/// Retorna (caminho_final, checksum_hex, tamanho_bytes).
pub async fn create_archive(
    staging_dir: &Path,
    destination_dir: &Path,
    date_str: &str,
    passphrase: &str,
) -> Result<(PathBuf, String, i64), BackupServiceError> {
    let staging_dir = staging_dir.to_path_buf();
    let destination_dir = destination_dir.to_path_buf();
    let date_str = date_str.to_string();
    let passphrase = passphrase.to_string();

    tokio::task::spawn_blocking(move || {
        // 1. Criar tar.gz em memória
        let mut tar_bytes: Vec<u8> = Vec::new();
        {
            let enc = GzEncoder::new(&mut tar_bytes, Compression::default());
            let mut tar = Builder::new(enc);
            tar.append_dir_all(".", &staging_dir)
                .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;
            tar.into_inner()
                .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?
                .finish()
                .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;
        }

        // 2. Cifrar o tar.gz com support-backup
        let encrypted = support_backup::pack(&tar_bytes, &passphrase)
            .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;

        // 3. Escrever no destino
        std::fs::create_dir_all(&destination_dir)
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

        let archive_name = format!("backup_{date_str}.mbak");
        let archive_path = destination_dir.join(&archive_name);
        std::fs::write(&archive_path, &encrypted)
            .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

        // 4. Calcular checksum SHA-256 do ficheiro cifrado
        let checksum = hex::encode(Sha256::digest(&encrypted));
        let size = encrypted.len() as i64;

        Ok((archive_path, checksum, size))
    })
    .await
    .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?
}

/// Decifra e extrai um arquivo `.mbak` para `dest_dir`.
/// Retorna os caminhos dos ficheiros extraídos.
pub fn extract_archive(
    data: &[u8],
    passphrase: &str,
    dest_dir: &Path,
) -> Result<Vec<PathBuf>, BackupServiceError> {
    let tar_gz = support_backup::unpack(data, passphrase)
        .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;

    std::fs::create_dir_all(dest_dir).map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;

    let mut archive = Archive::new(GzDecoder::new(tar_gz.as_slice()));
    let mut restored = Vec::new();

    for entry in archive
        .entries()
        .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;
        let rel = entry
            .path()
            .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?
            .to_path_buf();

        // Ignorar entradas que não sejam ficheiros (ex: diretório raiz ".")
        if entry.header().entry_type().is_dir() {
            continue;
        }

        let dest = dest_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| BackupServiceError::IoFailed(e.to_string()))?;
        }
        entry
            .unpack(&dest)
            .map_err(|e| BackupServiceError::ArchiveFailed(e.to_string()))?;
        restored.push(dest);
    }

    Ok(restored)
}

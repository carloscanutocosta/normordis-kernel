use crate::cipher::{decrypt, encrypt};
use crate::compress::{compress, decompress};
use crate::error::BackupError;

/// Comprime e cifra bytes de origem em formato de arquivo `.mbak`.
pub fn pack(source: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError> {
    let compressed = compress(source)?;
    encrypt(&compressed, passphrase)
}

/// Decifra e descomprime um arquivo `.mbak` produzido por [`pack`].
pub fn unpack(archive: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError> {
    let compressed = decrypt(archive, passphrase)?;
    decompress(&compressed)
}

use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::io::Read;

use crate::error::BackupError;

pub fn compress(data: &[u8]) -> Result<Vec<u8>, BackupError> {
    let mut encoder = GzEncoder::new(data, Compression::default());
    let mut out = Vec::new();
    encoder.read_to_end(&mut out).map_err(|_| BackupError::CompressFailed)?;
    Ok(out)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, BackupError> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|_| BackupError::DecompressFailed)?;
    Ok(out)
}

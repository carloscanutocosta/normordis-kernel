use support_backup::{pack, unpack, BackupError};

#[test]
fn pack_unpack_roundtrip() {
    let data = b"dados institucionais para backup - conteudo de teste";
    let packed = pack(data, "passphrase-segura-123").unwrap();
    let unpacked = unpack(&packed, "passphrase-segura-123").unwrap();
    assert_eq!(unpacked, data);
}

#[test]
fn wrong_passphrase_fails_to_unpack() {
    let data = b"conteudo confidencial";
    let packed = pack(data, "passphrase-correta").unwrap();
    let result = unpack(&packed, "passphrase-errada");
    assert!(matches!(result, Err(BackupError::DecryptFailed)));
}

#[test]
fn packed_bytes_differ_from_original() {
    let data = b"texto original";
    let packed = pack(data, "passphrase").unwrap();
    assert_ne!(packed.as_slice(), data.as_slice());
}

#[test]
fn large_data_is_compressed() {
    // Dados repetitivos comprimem bem — o arquivo cifrado deve ser menor que o original
    let data = vec![0xABu8; 512 * 1024]; // 512 KB de bytes repetidos
    let packed = pack(&data, "passphrase").unwrap();
    assert!(
        packed.len() < data.len(),
        "packed ({} bytes) should be smaller than original ({} bytes)",
        packed.len(),
        data.len()
    );
    let unpacked = unpack(&packed, "passphrase").unwrap();
    assert_eq!(unpacked, data);
}

#[test]
fn empty_data_roundtrip() {
    let packed = pack(b"", "passphrase").unwrap();
    let unpacked = unpack(&packed, "passphrase").unwrap();
    assert_eq!(unpacked, b"");
}

#[test]
fn corrupt_archive_fails_to_unpack() {
    let garbage = b"isto nao e um arquivo mbak valido";
    let result = unpack(garbage, "passphrase");
    assert!(result.is_err());
}

#[test]
fn two_packs_of_same_data_produce_different_ciphertexts() {
    // Garante que nonce é aleatório — dois packs nunca devem ser idênticos
    let data = b"mesmo conteudo";
    let packed1 = pack(data, "passphrase").unwrap();
    let packed2 = pack(data, "passphrase").unwrap();
    assert_ne!(packed1, packed2);
}

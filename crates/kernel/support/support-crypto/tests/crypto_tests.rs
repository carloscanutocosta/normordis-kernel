use support_crypto::{
    decrypt_bytes_with_key, decrypt_bytes_with_passphrase, decrypt_text_with_key,
    decrypt_text_with_passphrase, derive_key_from_passphrase, encrypt_bytes_with_key,
    encrypt_bytes_with_passphrase, encrypt_text_with_key, encrypt_text_with_passphrase,
    CryptoError, CryptoPolicy, EncryptedPayload, KdfConfig, KeyId, KeyProvider, KeyResolver,
    KeyResult, SecretKey, StaticKeyProvider, StorageAad, StorageEnvelope, CURRENT_ALGORITHM,
    CURRENT_CRYPTO_VERSION, CURRENT_KDF, DECRYPT_FAILED, EXTERNAL_KEY, INVALID_AAD,
    INVALID_PAYLOAD, POLICY_VIOLATION, STORAGE_AAD_PREFIX,
};
#[test]
fn roundtrips_text_with_passphrase() {
    let payload = encrypt_text_with_passphrase(
        "segredo muito importante",
        "passphrase-segura",
        Some(b"document:123"),
    )
    .unwrap();

    let plaintext =
        decrypt_text_with_passphrase(&payload, "passphrase-segura", Some(b"document:123")).unwrap();

    assert_eq!(plaintext, "segredo muito importante");
    assert_eq!(payload.version, CURRENT_CRYPTO_VERSION);
    assert_eq!(payload.algorithm, CURRENT_ALGORITHM);
    assert_eq!(payload.kdf.algorithm, CURRENT_KDF);
    assert_eq!(payload.key_id, None);
}

#[test]
fn rejects_wrong_passphrase() {
    let payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    let err = decrypt_bytes_with_passphrase(&payload, "errada", None).unwrap_err();
    assert_eq!(err, CryptoError::DecryptionFailed);
}

#[test]
fn rejects_wrong_aad() {
    let payload = encrypt_bytes_with_passphrase(b"dados", "correta", Some(b"ctx-a")).unwrap();
    let err = decrypt_bytes_with_passphrase(&payload, "correta", Some(b"ctx-b")).unwrap_err();
    assert_eq!(err, CryptoError::DecryptionFailed);
}

#[test]
fn rejects_tampered_ciphertext() {
    let mut payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    payload.ciphertext_b64.push('A');
    let err = decrypt_bytes_with_passphrase(&payload, "correta", None).unwrap_err();
    assert!(matches!(
        err,
        CryptoError::InvalidEncoding | CryptoError::DecryptionFailed
    ));
}

#[test]
fn derives_deterministic_key_for_same_inputs() {
    let config = KdfConfig {
        algorithm: CURRENT_KDF.to_string(),
        memory_kib: 19_456,
        iterations: 2,
        parallelism: 1,
        salt_b64: "c2FsdDEyMzQ1Njc4OTAxMg".to_string(),
    };

    let key1 = derive_key_from_passphrase("segredo", &config).unwrap();
    let key2 = derive_key_from_passphrase("segredo", &config).unwrap();

    assert_eq!(key1.0, key2.0);
}

#[test]
fn serializes_payload_to_json() {
    let payload = encrypt_text_with_passphrase("abc", "segredo", None).unwrap();
    let json = serde_json::to_string(&payload).unwrap();
    let roundtrip: EncryptedPayload = serde_json::from_str(&json).unwrap();

    assert_eq!(roundtrip.version, CURRENT_CRYPTO_VERSION);
    assert_eq!(roundtrip.algorithm, CURRENT_ALGORITHM);
}

#[test]
fn rejects_tampered_nonce() {
    let mut payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    payload.nonce_b64 = "invalid".to_string();

    let err = decrypt_bytes_with_passphrase(&payload, "correta", None).unwrap_err();

    assert!(matches!(
        err,
        CryptoError::InvalidEncoding | CryptoError::InvalidNonce
    ));
}

#[test]
fn rejects_unsupported_version() {
    let mut payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    payload.version = CURRENT_CRYPTO_VERSION + 1;

    let err = decrypt_bytes_with_passphrase(&payload, "correta", None).unwrap_err();

    assert_eq!(
        err,
        CryptoError::UnsupportedVersion(CURRENT_CRYPTO_VERSION + 1)
    );
}

#[test]
fn rejects_unsupported_algorithm() {
    let mut payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    payload.algorithm = "AES-256-GCM".to_string();

    let err = decrypt_bytes_with_passphrase(&payload, "correta", None).unwrap_err();

    assert_eq!(
        err,
        CryptoError::UnsupportedAlgorithm("AES-256-GCM".to_string())
    );
}

#[test]
fn rejects_unsupported_kdf() {
    let mut payload = encrypt_bytes_with_passphrase(b"dados", "correta", None).unwrap();
    payload.kdf.algorithm = "PBKDF2".to_string();

    let err = decrypt_bytes_with_passphrase(&payload, "correta", None).unwrap_err();

    assert_eq!(err, CryptoError::UnsupportedKdf("PBKDF2".to_string()));
}

#[test]
fn secret_key_debug_is_redacted() {
    let config = KdfConfig {
        algorithm: CURRENT_KDF.to_string(),
        memory_kib: 19_456,
        iterations: 2,
        parallelism: 1,
        salt_b64: "c2FsdDEyMzQ1Njc4OTAxMg".to_string(),
    };

    let key = derive_key_from_passphrase("segredo", &config).unwrap();

    assert_eq!(format!("{key:?}"), "SecretKey([REDACTED])");
}

#[test]
fn converts_crypto_error_to_safe_mini_error() {
    let err = CryptoError::DecryptionFailed.to_mini_error();
    let public = err.to_public();

    assert_eq!(public.code, DECRYPT_FAILED);
    assert_eq!(public.message, "failed to decrypt payload");
}

#[test]
fn invalid_payload_mini_error_does_not_expose_payload_values() {
    let err = CryptoError::UnsupportedAlgorithm("SECRET-ALG".to_string()).to_mini_error();
    let public = err.to_public();

    assert_eq!(public.code, INVALID_PAYLOAD);
    assert!(!public.message.contains("SECRET-ALG"));
}

#[test]
fn storage_aad_builds_canonical_context() {
    let aad = StorageAad::new("documents", "doc-1", "body").unwrap();

    assert_eq!(
        aad.canonical(),
        format!("{STORAGE_AAD_PREFIX}:documents:doc-1:body")
    );
    assert_eq!(aad.aad_bytes(), aad.canonical().into_bytes());
}

#[test]
fn storage_aad_rejects_empty_whitespace_or_colon_segments() {
    assert_eq!(
        StorageAad::new("", "record", "field").unwrap_err(),
        CryptoError::InvalidAad
    );
    assert_eq!(
        StorageAad::new("name space", "record", "field").unwrap_err(),
        CryptoError::InvalidAad
    );
    assert_eq!(
        StorageAad::new("namespace", "record:1", "field").unwrap_err(),
        CryptoError::InvalidAad
    );
}

#[test]
fn storage_aad_binds_ciphertext_to_record_context() {
    let aad = StorageAad::new("documents", "doc-1", "body").unwrap();
    let aad_bytes = aad.aad_bytes();
    let payload =
        encrypt_text_with_passphrase("conteudo", "passphrase-segura", Some(&aad_bytes)).unwrap();

    let wrong_aad = StorageAad::new("documents", "doc-2", "body")
        .unwrap()
        .aad_bytes();
    let err =
        decrypt_text_with_passphrase(&payload, "passphrase-segura", Some(&wrong_aad)).unwrap_err();

    assert_eq!(err, CryptoError::DecryptionFailed);
}

#[test]
fn invalid_aad_converts_to_safe_mini_error() {
    let err = CryptoError::InvalidAad.to_mini_error();
    let public = err.to_public();

    assert_eq!(public.code, INVALID_AAD);
    assert_eq!(public.message, "storage encryption context is invalid");
}

struct FixedKeyProvider {
    key: Option<SecretKey>,
}

impl KeyProvider for FixedKeyProvider {
    fn current_key(&self) -> KeyResult {
        self.key
            .as_ref()
            .map(|key| SecretKey(key.0))
            .ok_or_else(|| Box::new(CryptoError::InvalidKey.to_mini_error()))
    }
}

#[test]
fn key_provider_contract_returns_current_key() {
    let config = KdfConfig {
        algorithm: CURRENT_KDF.to_string(),
        memory_kib: 19_456,
        iterations: 2,
        parallelism: 1,
        salt_b64: "c2FsdDEyMzQ1Njc4OTAxMg".to_string(),
    };
    let key = derive_key_from_passphrase("segredo", &config).unwrap();
    let provider = FixedKeyProvider { key: Some(key) };

    let current = provider.current_key().unwrap();

    assert_eq!(current.0.len(), support_crypto::KEY_LENGTH_BYTES);
}

#[test]
fn encrypts_and_decrypts_with_external_key_and_key_id() {
    let key = SecretKey::new([7; support_crypto::KEY_LENGTH_BYTES]);
    let key_id = KeyId::new("local-main-v1").unwrap();
    let aad = StorageAad::new("documents", "doc-1", "body").unwrap();
    let aad_bytes = aad.aad_bytes();

    let payload = encrypt_text_with_key("conteudo", &key, Some(&aad_bytes), Some(&key_id)).unwrap();
    let plaintext = decrypt_text_with_key(&payload, &key, Some(&aad_bytes)).unwrap();

    assert_eq!(plaintext, "conteudo");
    assert_eq!(payload.key_id.as_deref(), Some("local-main-v1"));
    assert_eq!(payload.kdf.algorithm, EXTERNAL_KEY);
}

#[test]
fn rejects_invalid_key_id() {
    assert_eq!(
        KeyId::new("local key").unwrap_err(),
        CryptoError::InvalidKeyId
    );
    assert_eq!(
        KeyId::new("local:key").unwrap_err(),
        CryptoError::InvalidKeyId
    );
}

#[test]
fn storage_policy_requires_key_id_for_storage_payloads() {
    let payload = encrypt_text_with_passphrase("conteudo", "passphrase-segura", None).unwrap();
    let err = CryptoPolicy::storage_default()
        .validate_payload(&payload)
        .unwrap_err();

    assert_eq!(err, CryptoError::PolicyViolation);
}

#[test]
fn storage_policy_accepts_storage_envelope_with_aad_and_key_id() {
    let key = SecretKey::new([11; support_crypto::KEY_LENGTH_BYTES]);
    let key_id = KeyId::new("local-main-v1").unwrap();
    let aad = StorageAad::new("documents", "doc-1", "body").unwrap();
    let aad_bytes = aad.aad_bytes();
    let payload = encrypt_text_with_key("conteudo", &key, Some(&aad_bytes), Some(&key_id)).unwrap();
    let envelope = StorageEnvelope::new(aad, payload);

    CryptoPolicy::storage_default()
        .validate_storage_envelope(&envelope)
        .unwrap();
}

#[test]
fn static_key_provider_resolves_expected_key_id() {
    let key = SecretKey::new([13; support_crypto::KEY_LENGTH_BYTES]);
    let key_id = KeyId::new("local-main-v1").unwrap();
    let provider = StaticKeyProvider::new(key, Some(key_id));

    let current = provider.current_key().unwrap();
    let resolved = provider.key_for_id(Some("local-main-v1")).unwrap();

    assert_eq!(current.0, [13; support_crypto::KEY_LENGTH_BYTES]);
    assert_eq!(resolved.0, [13; support_crypto::KEY_LENGTH_BYTES]);
}

#[test]
fn static_key_provider_rejects_unexpected_key_id() {
    let key = SecretKey::new([13; support_crypto::KEY_LENGTH_BYTES]);
    let key_id = KeyId::new("local-main-v1").unwrap();
    let provider = StaticKeyProvider::new(key, Some(key_id));

    let err = provider.key_for_id(Some("other-key")).unwrap_err();

    assert_eq!(err.to_public().code, support_crypto::INVALID_KEY);
}

#[test]
fn static_key_provider_debug_redacts_key_material() {
    let key = SecretKey::new([21; support_crypto::KEY_LENGTH_BYTES]);
    let provider = StaticKeyProvider::new(key, Some(KeyId::new("local-main-v1").unwrap()));

    let debug = format!("{provider:?}");

    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("21"));
}

#[test]
fn policy_violation_converts_to_safe_mini_error() {
    let err = CryptoError::PolicyViolation.to_mini_error();
    let public = err.to_public();

    assert_eq!(public.code, POLICY_VIOLATION);
    assert_eq!(public.message, "crypto policy requirements were not met");
}

#[test]
fn validates_external_key_payload_shape() {
    let key = SecretKey::new([19; support_crypto::KEY_LENGTH_BYTES]);
    let payload = encrypt_bytes_with_key(b"dados", &key, None, None).unwrap();

    support_crypto::validate_encrypted_payload(&payload).unwrap();

    let decrypted = decrypt_bytes_with_key(&payload, &key, None).unwrap();
    assert_eq!(decrypted, b"dados");
}

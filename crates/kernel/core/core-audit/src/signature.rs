use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::chain::AuditExportManifest;
use crate::error::AuditError;

pub const AUDIT_SIGNATURE_ALGORITHM: &str = "Ed25519";

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AuditSigningKey {
    bytes: [u8; 32],
}

impl AuditSigningKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.bytes)
    }
}

impl std::fmt::Debug for AuditSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AuditSigningKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditManifestSignature {
    pub algorithm: String,
    pub key_id: Option<String>,
    pub public_key_b64: String,
    pub signature_b64: String,
    pub signed_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedAuditExportManifest {
    pub manifest: AuditExportManifest,
    pub signature: AuditManifestSignature,
}

pub fn sign_manifest(
    manifest: AuditExportManifest,
    signing_key: &AuditSigningKey,
    key_id: Option<String>,
) -> Result<SignedAuditExportManifest, AuditError> {
    let key = signing_key.signing_key();
    let public_key = key.verifying_key();
    let bytes = canonical_manifest_bytes(&manifest)?;
    let signature = key.sign(&bytes);

    Ok(SignedAuditExportManifest {
        manifest,
        signature: AuditManifestSignature {
            algorithm: AUDIT_SIGNATURE_ALGORITHM.to_string(),
            key_id,
            public_key_b64: STANDARD.encode(public_key.as_bytes()),
            signature_b64: STANDARD.encode(signature.to_bytes()),
            signed_at_utc: Utc::now(),
        },
    })
}

pub fn verify_signed_manifest(signed: &SignedAuditExportManifest) -> Result<(), AuditError> {
    if signed.signature.algorithm != AUDIT_SIGNATURE_ALGORITHM {
        return Err(AuditError::SignatureVerificationFailed);
    }

    let public_key_bytes = STANDARD
        .decode(&signed.signature.public_key_b64)
        .map_err(|_| AuditError::SignatureVerificationFailed)?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| AuditError::SignatureVerificationFailed)?;
    let public_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| AuditError::SignatureVerificationFailed)?;

    let signature_bytes = STANDARD
        .decode(&signed.signature.signature_b64)
        .map_err(|_| AuditError::SignatureVerificationFailed)?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AuditError::SignatureVerificationFailed)?;
    let signature = Signature::from_bytes(&signature_bytes);

    public_key
        .verify(&canonical_manifest_bytes(&signed.manifest)?, &signature)
        .map_err(|_| AuditError::SignatureVerificationFailed)
}

fn canonical_manifest_bytes(manifest: &AuditExportManifest) -> Result<Vec<u8>, AuditError> {
    serde_json::to_vec(manifest).map_err(|_| AuditError::SignFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> AuditExportManifest {
        AuditExportManifest {
            schema_version: 1,
            generated_at_utc: Utc::now(),
            events_count: 2,
            head_record_hash: Some("abc".to_string()),
            manifest_hash: "manifest-hash".to_string(),
        }
    }

    #[test]
    fn signs_and_verifies_manifest() {
        let key = AuditSigningKey::from_bytes([7; 32]);
        let signed = sign_manifest(manifest(), &key, Some("audit-key-1".to_string())).unwrap();

        assert_eq!(signed.signature.algorithm, AUDIT_SIGNATURE_ALGORITHM);
        assert_eq!(signed.signature.key_id.as_deref(), Some("audit-key-1"));
        verify_signed_manifest(&signed).unwrap();
    }

    #[test]
    fn rejects_tampered_manifest_signature() {
        let key = AuditSigningKey::from_bytes([7; 32]);
        let mut signed = sign_manifest(manifest(), &key, None).unwrap();
        signed.manifest.events_count += 1;

        assert_eq!(
            verify_signed_manifest(&signed).unwrap_err(),
            AuditError::SignatureVerificationFailed
        );
    }
}

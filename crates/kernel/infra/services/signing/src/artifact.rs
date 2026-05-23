use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{require_non_empty, Provider, Result, SignatureFormat, SigningError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetachedSignatureRequest {
    pub provider: Provider,
    pub format: SignatureFormat,
    pub profile: String,
    pub certificate_ref: Option<String>,
    pub trust_service_ref: Option<String>,
    pub bytes_to_sign: Vec<u8>,
}

impl DetachedSignatureRequest {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        if self.bytes_to_sign.is_empty() {
            return Err(SigningError::EmptyField("bytes_to_sign"));
        }
        if self.provider.requires_certificate_ref()
            && self
                .certificate_ref
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::EmptyField("certificate_ref"));
        }
        Ok(())
    }

    pub fn signing_hash_hex(&self) -> String {
        hex::encode(Sha256::digest(&self.bytes_to_sign))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetachedSignature {
    pub format: SignatureFormat,
    pub algorithm: String,
    pub signature_der: Vec<u8>,
    pub certificate_ref: Option<String>,
    pub signed_at: DateTime<Utc>,
    pub signing_hash_hex: String,
}

impl DetachedSignature {
    pub fn validate_for(&self, request: &DetachedSignatureRequest) -> Result<()> {
        require_non_empty("algorithm", &self.algorithm)?;
        if self.signature_der.is_empty() {
            return Err(SigningError::EmptyField("signature_der"));
        }
        if self.format != request.format {
            return Err(SigningError::InvalidValue {
                field: "format",
                reason: "assinatura não corresponde ao formato pedido",
            });
        }
        if self.signing_hash_hex != request.signing_hash_hex() {
            return Err(SigningError::InvalidValue {
                field: "signing_hash_hex",
                reason: "hash assinado não corresponde aos bytes pedidos",
            });
        }
        Ok(())
    }
}

pub trait ExternalSigner {
    fn sign_detached(&self, request: &DetachedSignatureRequest) -> Result<DetachedSignature>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningEvidence {
    pub provider: Provider,
    pub format: SignatureFormat,
    pub profile: String,
    pub certificate_ref: Option<String>,
    pub trust_service_ref: Option<String>,
    pub signing_hash_hex: String,
    pub signature_hash_hex: String,
    pub signed_at: DateTime<Utc>,
}

impl SigningEvidence {
    pub fn from_signature(
        request: &DetachedSignatureRequest,
        signature: &DetachedSignature,
    ) -> Result<Self> {
        signature.validate_for(request)?;
        Ok(Self {
            provider: request.provider,
            format: request.format,
            profile: request.profile.clone(),
            certificate_ref: request.certificate_ref.clone(),
            trust_service_ref: request.trust_service_ref.clone(),
            signing_hash_hex: signature.signing_hash_hex.clone(),
            signature_hash_hex: hex::encode(Sha256::digest(&signature.signature_der)),
            signed_at: signature.signed_at,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DetachedSigningService;

impl DetachedSigningService {
    pub fn sign<S: ExternalSigner>(
        &self,
        signer: &S,
        request: &DetachedSignatureRequest,
    ) -> Result<(DetachedSignature, SigningEvidence)> {
        request.validate()?;
        let signature = signer.sign_detached(request)?;
        let evidence = SigningEvidence::from_signature(request, &signature)?;
        Ok((signature, evidence))
    }
}

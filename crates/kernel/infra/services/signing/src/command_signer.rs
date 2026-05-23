use std::io::Write;
use std::process::{Command, Stdio};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    require_non_empty, DetachedSignature, DetachedSignatureRequest, ExternalSigner, Result,
    SigningError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSignerConfig {
    pub program: String,
    pub args: Vec<String>,
    pub algorithm: String,
}

impl CommandSignerConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("program", &self.program)?;
        require_non_empty("algorithm", &self.algorithm)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CommandExternalSigner {
    config: CommandSignerConfig,
}

impl CommandExternalSigner {
    pub fn new(config: CommandSignerConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }
}

impl ExternalSigner for CommandExternalSigner {
    fn sign_detached(&self, request: &DetachedSignatureRequest) -> Result<DetachedSignature> {
        request.validate()?;

        let mut child = Command::new(&self.config.program)
            .args(&self.config.args)
            .env("MINI_SIGNING_PROVIDER", request.provider.as_str())
            .env("MINI_SIGNING_FORMAT", request.format.as_str())
            .env("MINI_SIGNING_PROFILE", &request.profile)
            .env("MINI_SIGNING_HASH_HEX", request.signing_hash_hex())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            SigningError::ExternalSignerFailed("stdin indisponível no signer externo".into())
        })?;
        stdin
            .write_all(&request.bytes_to_sign)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::ExternalSignerFailed(
                stderr.trim().to_string(),
            ));
        }
        if output.stdout.is_empty() {
            return Err(SigningError::ExternalSignerFailed(
                "signer externo não devolveu assinatura DER".into(),
            ));
        }

        Ok(DetachedSignature {
            format: request.format,
            algorithm: self.config.algorithm.clone(),
            signature_der: output.stdout,
            certificate_ref: request.certificate_ref.clone(),
            signed_at: Utc::now(),
            signing_hash_hex: request.signing_hash_hex(),
        })
    }
}

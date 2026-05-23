use serde::{Deserialize, Serialize};

use crate::{
    require_non_empty, validate_secret_ref, Config, Operation, Provider,
    QualifiedCertificateConfig, Result, SignatureFormat, SigningError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CartaoCidadaoPtMode {
    Middleware,
    AutenticacaoGov,
}

impl CartaoCidadaoPtMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Middleware => "middleware",
            Self::AutenticacaoGov => "autenticacao-gov",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartaoCidadaoPtConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub certificate_ref: String,
    pub signature_pin_ref: String,
    pub mode: CartaoCidadaoPtMode,
    pub trust_service_ref: String,
    pub reader_ref: Option<String>,
    pub require_timestamp: bool,
}

impl CartaoCidadaoPtConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        validate_secret_ref("signature_pin_ref", &self.signature_pin_ref)?;
        require_non_empty("trust_service_ref", &self.trust_service_ref)?;
        if self.mode == CartaoCidadaoPtMode::Middleware
            && self
                .reader_ref
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::EmptyField("reader_ref"));
        }
        Ok(())
    }

    pub fn base_config(&self) -> Config {
        Config {
            provider: Provider::CartaoCidadaoPt,
            profile: self.profile.clone(),
            certificate_ref: Some(self.certificate_ref.clone()),
        }
    }

    pub fn qualified_equivalent_config(&self) -> QualifiedCertificateConfig {
        QualifiedCertificateConfig {
            profile: self.profile.clone(),
            format: self.format,
            certificate_ref: self.certificate_ref.clone(),
            trust_service_ref: self.trust_service_ref.clone(),
            require_qualified_device: true,
            require_timestamp: self.require_timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartaoCidadaoPtPlan {
    pub provider: Provider,
    pub mode: CartaoCidadaoPtMode,
    pub format: SignatureFormat,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CartaoCidadaoPtAdapter;

impl CartaoCidadaoPtAdapter {
    pub fn build_plan(&self, cfg: &CartaoCidadaoPtConfig) -> Result<CartaoCidadaoPtPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("load-citizen-card-certificate"),
            Operation::required("resolve-portuguese-qualified-trust-context"),
        ];
        match cfg.mode {
            CartaoCidadaoPtMode::Middleware => {
                operations.push(Operation::required("check-smartcard-middleware"));
                operations.push(Operation::required("check-card-reader-availability"));
                operations.push(Operation::required("request-signature-pin"));
            }
            CartaoCidadaoPtMode::AutenticacaoGov => {
                operations.push(Operation::required("start-autenticacao-gov-signing-flow"));
                operations.push(Operation::required("await-remote-user-confirmation"));
            }
        }
        operations.push(Operation::required("produce-qualified-signature-artifact"));
        if cfg.require_timestamp {
            operations.push(Operation::required("attach-qualified-timestamp"));
        }
        Ok(CartaoCidadaoPtPlan {
            provider: Provider::CartaoCidadaoPt,
            mode: cfg.mode,
            format: cfg.format,
            operations,
        })
    }
}

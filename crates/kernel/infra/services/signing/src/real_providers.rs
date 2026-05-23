use serde::{Deserialize, Serialize};

use crate::{
    require_non_empty, validate_secret_ref, Operation, Provider, Result, SignatureFormat,
    SigningError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MiddlewareKind {
    Pkcs11,
    WindowsCapi,
    MacosKeychain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub kind: MiddlewareKind,
    pub certificate_ref: String,
    pub pin_ref: Option<String>,
    pub module_path: Option<String>,
    pub token_ref: Option<String>,
    pub trust_service_ref: Option<String>,
}

impl MiddlewareConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        if let Some(pin_ref) = &self.pin_ref {
            validate_secret_ref("pin_ref", pin_ref)?;
        }
        if self.kind == MiddlewareKind::Pkcs11
            && self
                .module_path
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::EmptyField("module_path"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutenticacaoGovFlow {
    CitizenCard,
    ChaveMovelDigital,
    Safe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutenticacaoGovConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub flow: AutenticacaoGovFlow,
    pub service_endpoint: String,
    pub client_id: String,
    pub callback_url: Option<String>,
    pub require_professional_attributes: bool,
    pub trust_service_ref: Option<String>,
}

impl AutenticacaoGovConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("service_endpoint", &self.service_endpoint)?;
        require_non_empty("client_id", &self.client_id)?;
        if self.flow == AutenticacaoGovFlow::Safe
            && self
                .callback_url
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(SigningError::EmptyField("callback_url"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsaConfig {
    pub profile: String,
    pub endpoint: String,
    pub policy_oid: Option<String>,
    pub credentials_ref: Option<String>,
    pub require_nonce: bool,
}

impl TsaConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("endpoint", &self.endpoint)?;
        if let Some(credentials_ref) = &self.credentials_ref {
            validate_secret_ref("credentials_ref", credentials_ref)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HsmConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub module_path: String,
    pub token_ref: String,
    pub key_ref: String,
    pub pin_ref: String,
    pub certificate_ref: String,
    pub trust_service_ref: Option<String>,
    pub tsa: Option<TsaConfig>,
}

impl HsmConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("module_path", &self.module_path)?;
        require_non_empty("token_ref", &self.token_ref)?;
        require_non_empty("key_ref", &self.key_ref)?;
        validate_secret_ref("pin_ref", &self.pin_ref)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        if let Some(tsa) = &self.tsa {
            tsa.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CegerCardConfig {
    pub profile: String,
    pub format: SignatureFormat,
    pub certificate_ref: String,
    pub signature_pin_ref: String,
    pub reader_ref: String,
    pub middleware_vendor: String,
    pub middleware_ref: String,
    pub trust_service_ref: String,
    pub require_timestamp: bool,
}

impl CegerCardConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        validate_secret_ref("signature_pin_ref", &self.signature_pin_ref)?;
        require_non_empty("reader_ref", &self.reader_ref)?;
        require_non_empty("middleware_vendor", &self.middleware_vendor)?;
        require_non_empty("middleware_ref", &self.middleware_ref)?;
        require_non_empty("trust_service_ref", &self.trust_service_ref)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderPlan {
    pub provider: Provider,
    pub format: Option<SignatureFormat>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealProviderAdapter;

impl RealProviderAdapter {
    pub fn build_middleware_plan(&self, cfg: &MiddlewareConfig) -> Result<ProviderPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("load-local-signing-middleware"),
            Operation::required("resolve-certificate"),
            Operation::required("produce-detached-signature"),
        ];
        if cfg.kind == MiddlewareKind::Pkcs11 {
            operations.insert(1, Operation::required("load-pkcs11-module"));
        }
        Ok(ProviderPlan {
            provider: Provider::Middleware,
            format: Some(cfg.format),
            operations,
        })
    }

    pub fn build_autenticacao_gov_plan(&self, cfg: &AutenticacaoGovConfig) -> Result<ProviderPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("start-autenticacao-gov-flow"),
            Operation::required("submit-signing-hash"),
            Operation::required("await-user-confirmation"),
            Operation::required("retrieve-signature-artifact"),
        ];
        if cfg.require_professional_attributes {
            operations.push(Operation::required("request-scap-professional-attributes"));
        }
        Ok(ProviderPlan {
            provider: Provider::AutenticacaoGov,
            format: Some(cfg.format),
            operations,
        })
    }

    pub fn build_tsa_plan(&self, cfg: &TsaConfig) -> Result<ProviderPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("hash-signature-artifact"),
            Operation::required("request-rfc3161-timestamp-token"),
            Operation::required("verify-timestamp-token"),
        ];
        if cfg.require_nonce {
            operations.insert(1, Operation::required("generate-timestamp-nonce"));
        }
        Ok(ProviderPlan {
            provider: Provider::Tsa,
            format: None,
            operations,
        })
    }

    pub fn build_hsm_plan(&self, cfg: &HsmConfig) -> Result<ProviderPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("load-hsm-pkcs11-module"),
            Operation::required("open-hsm-token-session"),
            Operation::required("resolve-hsm-key"),
            Operation::required("produce-detached-signature"),
        ];
        if cfg.tsa.is_some() {
            operations.push(Operation::required("attach-timestamp-token"));
        }
        Ok(ProviderPlan {
            provider: Provider::Hsm,
            format: Some(cfg.format),
            operations,
        })
    }

    pub fn build_ceger_card_plan(&self, cfg: &CegerCardConfig) -> Result<ProviderPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required(format!(
                "load-{}-middleware",
                cfg.middleware_vendor.to_ascii_lowercase()
            )),
            Operation::required("check-ceger-card-reader"),
            Operation::required("load-ceger-qualified-certificate"),
            Operation::required("request-signature-pin"),
            Operation::required("produce-qualified-signature-artifact"),
        ];
        if cfg.require_timestamp {
            operations.push(Operation::required("attach-qualified-timestamp"));
        }
        Ok(ProviderPlan {
            provider: Provider::CegerCard,
            format: Some(cfg.format),
            operations,
        })
    }
}

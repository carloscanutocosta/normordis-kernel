use serde::{Deserialize, Serialize};

use crate::{require_non_empty, Operation, Provider, Result, SigningError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CitizenCardMiddleware {
    OfficialSdkCpp,
    OfficialSdkJava,
    Pkcs11,
    TlsClientCertificate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CitizenCardAuthMode {
    MutualTls,
    SignedChallenge,
    CertificateDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenCardAuthConfig {
    pub profile: String,
    pub middleware: CitizenCardMiddleware,
    pub mode: CitizenCardAuthMode,
    pub certificate_ref: String,
    pub trust_chain_ref: String,
    pub pkcs11_module_path: Option<String>,
    pub sdk_library_ref: Option<String>,
    pub require_card_present: bool,
    pub require_active_certificates: bool,
}

impl CitizenCardAuthConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("certificate_ref", &self.certificate_ref)?;
        require_non_empty("trust_chain_ref", &self.trust_chain_ref)?;
        match self.middleware {
            CitizenCardMiddleware::Pkcs11 => {
                if self
                    .pkcs11_module_path
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
                {
                    return Err(SigningError::EmptyField("pkcs11_module_path"));
                }
            }
            CitizenCardMiddleware::OfficialSdkCpp | CitizenCardMiddleware::OfficialSdkJava => {
                if self
                    .sdk_library_ref
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
                {
                    return Err(SigningError::EmptyField("sdk_library_ref"));
                }
            }
            CitizenCardMiddleware::TlsClientCertificate => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenCardAuthPlan {
    pub provider: Provider,
    pub mode: CitizenCardAuthMode,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CitizenCardAuthAdapter;

impl CitizenCardAuthAdapter {
    pub fn build_plan(&self, cfg: &CitizenCardAuthConfig) -> Result<CitizenCardAuthPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("install-or-detect-autenticacao-gov-middleware"),
            Operation::required("detect-smartcard-reader"),
        ];

        if cfg.require_card_present {
            operations.push(Operation::required("verify-citizen-card-present"));
        }
        if cfg.require_active_certificates {
            operations.push(Operation::required(
                "verify-active-citizen-card-certificates",
            ));
        }

        match cfg.middleware {
            CitizenCardMiddleware::OfficialSdkCpp => {
                operations.push(Operation::required("initialize-pteid-cpp-sdk"));
                operations.push(Operation::required(
                    "read-authentication-certificate-via-sdk",
                ));
            }
            CitizenCardMiddleware::OfficialSdkJava => {
                operations.push(Operation::required("load-pteidlibj-jni"));
                operations.push(Operation::required(
                    "read-authentication-certificate-via-sdk",
                ));
            }
            CitizenCardMiddleware::Pkcs11 => {
                operations.push(Operation::required("load-citizen-card-pkcs11-module"));
                operations.push(Operation::required(
                    "read-authentication-certificate-via-pkcs11",
                ));
            }
            CitizenCardMiddleware::TlsClientCertificate => {
                operations.push(Operation::required(
                    "delegate-client-certificate-selection-to-tls",
                ));
            }
        }

        operations.push(Operation::required("validate-certificate-chain"));
        match cfg.mode {
            CitizenCardAuthMode::MutualTls => {
                operations.push(Operation::required("complete-mutual-tls-handshake"));
            }
            CitizenCardAuthMode::SignedChallenge => {
                operations.push(Operation::required("sign-authentication-challenge"));
                operations.push(Operation::required(
                    "verify-authentication-challenge-signature",
                ));
            }
            CitizenCardAuthMode::CertificateDiscovery => {
                operations.push(Operation::required(
                    "return-authenticated-certificate-identity",
                ));
            }
        }

        Ok(CitizenCardAuthPlan {
            provider: Provider::CitizenCardAuth,
            mode: cfg.mode,
            operations,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenCardOfficialMiddlewareDefaults {
    pub documentation_url: String,
    pub sdk_cpp_url: String,
    pub sdk_java_url: String,
}

impl Default for CitizenCardOfficialMiddlewareDefaults {
    fn default() -> Self {
        Self {
            documentation_url: "https://amagovpt.github.io/docs.autenticacao.gov/".into(),
            sdk_cpp_url: "https://amagovpt.github.io/docs.autenticacao.gov/sdk/cpp/".into(),
            sdk_java_url: "https://amagovpt.github.io/docs.autenticacao.gov/sdk/java/".into(),
        }
    }
}

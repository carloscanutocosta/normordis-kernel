use serde::{Deserialize, Serialize};

use crate::{
    AutenticacaoGovConfig, AutenticacaoGovFlow, CegerCardConfig, ChaveMovelDigitalConfig,
    ChaveMovelDigitalUserConfirmation, CommandExternalSigner, CommandSignerConfig, HsmConfig,
    MiddlewareConfig, MiddlewareKind, Pkcs11Mechanism, Pkcs11SigningConfig, Result,
    SignatureFormat, TsaConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bit4IdMiddlewareDefaults {
    pub module_path: String,
    pub middleware_ref: String,
}

impl Bit4IdMiddlewareDefaults {
    pub fn for_current_os() -> Self {
        Self {
            module_path: bit4id_pkcs11_module_path().into(),
            middleware_ref: "ecce:https://www.ecce.gov.pt/suporte/middleware/software".into(),
        }
    }
}

pub fn bit4id_pkcs11_module_path() -> &'static str {
    if cfg!(target_os = "windows") {
        r"C:\Windows\System32\bit4xpki.dll"
    } else if cfg!(target_os = "macos") {
        "/System/Library/Bit4id/pkcs11/libbit4ipki.dylib"
    } else {
        "/usr/lib/bit4id/libbit4xpki.so"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenCardPkcs11Defaults {
    pub module_candidates: Vec<String>,
    pub documentation_url: String,
}

impl CitizenCardPkcs11Defaults {
    pub fn for_current_os() -> Self {
        Self {
            module_candidates: citizen_card_pkcs11_module_candidates()
                .into_iter()
                .map(str::to_string)
                .collect(),
            documentation_url: "https://amagovpt.github.io/docs.autenticacao.gov/".into(),
        }
    }
}

pub fn citizen_card_pkcs11_module_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "windows") {
        vec![
            r"C:\Windows\System32\pteidpkcs11.dll",
            r"C:\Windows\System32\libpteidpkcs11.dll",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/usr/local/lib/libpteidpkcs11.dylib",
            "/Library/Frameworks/pteidpkcs11.framework/pteidpkcs11",
        ]
    } else {
        vec![
            "/usr/local/lib/libpteidpkcs11.so",
            "/usr/lib/libpteidpkcs11.so",
            "/usr/lib/x86_64-linux-gnu/libpteidpkcs11.so",
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenCardPkcs11Toolchain {
    pub profile: String,
    pub format: SignatureFormat,
    pub module_path: Option<String>,
    pub slot_ref: Option<String>,
    pub token_ref: Option<String>,
    pub private_key_label: Option<String>,
    pub private_key_id_hex: Option<String>,
    pub certificate_ref: String,
    pub pin_ref: String,
    pub mechanism: Pkcs11Mechanism,
    pub trust_chain_ref: Option<String>,
}

impl CitizenCardPkcs11Toolchain {
    pub fn signing_config(&self) -> Pkcs11SigningConfig {
        let module_path = self.module_path.clone().unwrap_or_else(|| {
            citizen_card_pkcs11_module_candidates()
                .first()
                .expect("citizen card pkcs11 candidates must not be empty")
                .to_string()
        });
        Pkcs11SigningConfig {
            profile: self.profile.clone(),
            format: self.format,
            module_path,
            slot_ref: self.slot_ref.clone(),
            token_ref: self.token_ref.clone(),
            private_key_label: self.private_key_label.clone(),
            private_key_id_hex: self.private_key_id_hex.clone(),
            certificate_ref: self.certificate_ref.clone(),
            pin_ref: self.pin_ref.clone(),
            mechanism: self.mechanism,
            trust_chain_ref: self.trust_chain_ref.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcceCegerToolchain {
    pub profile: String,
    pub format: SignatureFormat,
    pub certificate_ref: String,
    pub signature_pin_ref: String,
    pub reader_ref: String,
    pub trust_service_ref: String,
    pub require_timestamp: bool,
}

impl EcceCegerToolchain {
    pub fn ceger_card_config(&self) -> CegerCardConfig {
        let defaults = Bit4IdMiddlewareDefaults::for_current_os();
        CegerCardConfig {
            profile: self.profile.clone(),
            format: self.format,
            certificate_ref: self.certificate_ref.clone(),
            signature_pin_ref: self.signature_pin_ref.clone(),
            reader_ref: self.reader_ref.clone(),
            middleware_vendor: "bit4id".into(),
            middleware_ref: defaults.middleware_ref,
            trust_service_ref: self.trust_service_ref.clone(),
            require_timestamp: self.require_timestamp,
        }
    }

    pub fn middleware_config(&self) -> MiddlewareConfig {
        let defaults = Bit4IdMiddlewareDefaults::for_current_os();
        MiddlewareConfig {
            profile: self.profile.clone(),
            format: self.format,
            kind: MiddlewareKind::Pkcs11,
            certificate_ref: self.certificate_ref.clone(),
            pin_ref: Some(self.signature_pin_ref.clone()),
            module_path: Some(defaults.module_path),
            token_ref: None,
            trust_service_ref: Some(self.trust_service_ref.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkcs11HsmToolchain {
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

impl Pkcs11HsmToolchain {
    pub fn hsm_config(&self) -> HsmConfig {
        HsmConfig {
            profile: self.profile.clone(),
            format: self.format,
            module_path: self.module_path.clone(),
            token_ref: self.token_ref.clone(),
            key_ref: self.key_ref.clone(),
            pin_ref: self.pin_ref.clone(),
            certificate_ref: self.certificate_ref.clone(),
            trust_service_ref: self.trust_service_ref.clone(),
            tsa: self.tsa.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutenticacaoGovToolchain {
    pub profile: String,
    pub format: SignatureFormat,
    pub flow: AutenticacaoGovFlow,
    pub service_endpoint: String,
    pub client_id: String,
    pub callback_url: Option<String>,
    pub require_professional_attributes: bool,
    pub trust_service_ref: Option<String>,
}

impl AutenticacaoGovToolchain {
    pub fn config(&self) -> AutenticacaoGovConfig {
        AutenticacaoGovConfig {
            profile: self.profile.clone(),
            format: self.format,
            flow: self.flow,
            service_endpoint: self.service_endpoint.clone(),
            client_id: self.client_id.clone(),
            callback_url: self.callback_url.clone(),
            require_professional_attributes: self.require_professional_attributes,
            trust_service_ref: self.trust_service_ref.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalToolchain {
    pub profile: String,
    pub format: SignatureFormat,
    pub service_endpoint: String,
    pub client_id: String,
    pub callback_url: Option<String>,
    pub trust_service_ref: Option<String>,
    pub require_professional_attributes: bool,
    pub require_timestamp: bool,
    pub user_confirmation: ChaveMovelDigitalUserConfirmation,
}

impl ChaveMovelDigitalToolchain {
    pub fn config(&self) -> ChaveMovelDigitalConfig {
        ChaveMovelDigitalConfig {
            profile: self.profile.clone(),
            format: self.format,
            service_endpoint: self.service_endpoint.clone(),
            client_id: self.client_id.clone(),
            callback_url: self.callback_url.clone(),
            trust_service_ref: self.trust_service_ref.clone(),
            require_professional_attributes: self.require_professional_attributes,
            require_timestamp: self.require_timestamp,
            user_confirmation: self.user_confirmation,
        }
    }

    pub fn autenticacao_gov_config(&self) -> AutenticacaoGovConfig {
        AutenticacaoGovConfig {
            profile: self.profile.clone(),
            format: self.format,
            flow: AutenticacaoGovFlow::ChaveMovelDigital,
            service_endpoint: self.service_endpoint.clone(),
            client_id: self.client_id.clone(),
            callback_url: self.callback_url.clone(),
            require_professional_attributes: self.require_professional_attributes,
            trust_service_ref: self.trust_service_ref.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSignerToolchain {
    pub program: String,
    pub args: Vec<String>,
    pub algorithm: String,
}

impl CommandSignerToolchain {
    pub fn signer(&self) -> Result<CommandExternalSigner> {
        CommandExternalSigner::new(CommandSignerConfig {
            program: self.program.clone(),
            args: self.args.clone(),
            algorithm: self.algorithm.clone(),
        })
    }
}

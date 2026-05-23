use serde::{Deserialize, Serialize};

use crate::{
    AutenticacaoGovConfig, CartaoCidadaoPtAdapter, CartaoCidadaoPtConfig, CegerCardConfig,
    CitizenCardAuthAdapter, CitizenCardAuthConfig, Config, HsmConfig, MiddlewareConfig, OtcAdapter,
    OtcConfig, Provider, QualifiedCertificateAdapter, QualifiedCertificateConfig,
    RealProviderAdapter, Result, SigningError, TsaConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBinding {
    pub default_provider: Provider,
    pub qualified_certificate: Option<QualifiedCertificateConfig>,
    pub otc: Option<OtcConfig>,
    pub cartao_cidadao_pt: Option<CartaoCidadaoPtConfig>,
    pub middleware: Option<MiddlewareConfig>,
    pub autenticacao_gov: Option<AutenticacaoGovConfig>,
    pub tsa: Option<TsaConfig>,
    pub hsm: Option<HsmConfig>,
    pub ceger_card: Option<CegerCardConfig>,
    pub citizen_card_auth: Option<CitizenCardAuthConfig>,
}

impl RuntimeBinding {
    pub fn validate(&self) -> Result<()> {
        match self.default_provider {
            Provider::QualifiedCertificate => self
                .qualified_certificate
                .as_ref()
                .ok_or(SigningError::EmptyField("qualified_certificate"))?
                .validate(),
            Provider::Otc => self
                .otc
                .as_ref()
                .ok_or(SigningError::EmptyField("otc"))?
                .validate(),
            Provider::CartaoCidadaoPt => self
                .cartao_cidadao_pt
                .as_ref()
                .ok_or(SigningError::EmptyField("cartao_cidadao_pt"))?
                .validate(),
            Provider::Middleware => self
                .middleware
                .as_ref()
                .ok_or(SigningError::EmptyField("middleware"))?
                .validate(),
            Provider::AutenticacaoGov => self
                .autenticacao_gov
                .as_ref()
                .ok_or(SigningError::EmptyField("autenticacao_gov"))?
                .validate(),
            Provider::Tsa => self
                .tsa
                .as_ref()
                .ok_or(SigningError::EmptyField("tsa"))?
                .validate(),
            Provider::Hsm => self
                .hsm
                .as_ref()
                .ok_or(SigningError::EmptyField("hsm"))?
                .validate(),
            Provider::CegerCard => self
                .ceger_card
                .as_ref()
                .ok_or(SigningError::EmptyField("ceger_card"))?
                .validate(),
            Provider::CitizenCardAuth => self
                .citizen_card_auth
                .as_ref()
                .ok_or(SigningError::EmptyField("citizen_card_auth"))?
                .validate(),
        }
    }

    pub fn selected_config(&self) -> Result<Config> {
        self.validate()?;
        let cfg = match self.default_provider {
            Provider::QualifiedCertificate => self
                .qualified_certificate
                .as_ref()
                .expect("validated qualified_certificate")
                .base_config(),
            Provider::Otc => self.otc.as_ref().expect("validated otc").base_config(),
            Provider::CartaoCidadaoPt => self
                .cartao_cidadao_pt
                .as_ref()
                .expect("validated cartao_cidadao_pt")
                .base_config(),
            Provider::Middleware => {
                let cfg = self.middleware.as_ref().expect("validated middleware");
                Config {
                    provider: Provider::Middleware,
                    profile: cfg.profile.clone(),
                    certificate_ref: Some(cfg.certificate_ref.clone()),
                }
            }
            Provider::AutenticacaoGov => {
                let cfg = self
                    .autenticacao_gov
                    .as_ref()
                    .expect("validated autenticacao_gov");
                Config {
                    provider: Provider::AutenticacaoGov,
                    profile: cfg.profile.clone(),
                    certificate_ref: None,
                }
            }
            Provider::Tsa => {
                let cfg = self.tsa.as_ref().expect("validated tsa");
                Config {
                    provider: Provider::Tsa,
                    profile: cfg.profile.clone(),
                    certificate_ref: None,
                }
            }
            Provider::Hsm => {
                let cfg = self.hsm.as_ref().expect("validated hsm");
                Config {
                    provider: Provider::Hsm,
                    profile: cfg.profile.clone(),
                    certificate_ref: Some(cfg.certificate_ref.clone()),
                }
            }
            Provider::CegerCard => {
                let cfg = self.ceger_card.as_ref().expect("validated ceger_card");
                Config {
                    provider: Provider::CegerCard,
                    profile: cfg.profile.clone(),
                    certificate_ref: Some(cfg.certificate_ref.clone()),
                }
            }
            Provider::CitizenCardAuth => {
                let cfg = self
                    .citizen_card_auth
                    .as_ref()
                    .expect("validated citizen_card_auth");
                Config {
                    provider: Provider::CitizenCardAuth,
                    profile: cfg.profile.clone(),
                    certificate_ref: Some(cfg.certificate_ref.clone()),
                }
            }
        };
        Ok(cfg)
    }

    pub fn selected_operations(&self) -> Result<Vec<String>> {
        self.validate()?;
        let operations = match self.default_provider {
            Provider::QualifiedCertificate => {
                QualifiedCertificateAdapter
                    .build_plan(
                        self.qualified_certificate
                            .as_ref()
                            .expect("validated qualified_certificate"),
                    )?
                    .operations
            }
            Provider::Otc => {
                OtcAdapter
                    .build_plan(self.otc.as_ref().expect("validated otc"))?
                    .operations
            }
            Provider::CartaoCidadaoPt => {
                CartaoCidadaoPtAdapter
                    .build_plan(
                        self.cartao_cidadao_pt
                            .as_ref()
                            .expect("validated cartao_cidadao_pt"),
                    )?
                    .operations
            }
            Provider::Middleware => {
                RealProviderAdapter
                    .build_middleware_plan(self.middleware.as_ref().expect("validated middleware"))?
                    .operations
            }
            Provider::AutenticacaoGov => {
                RealProviderAdapter
                    .build_autenticacao_gov_plan(
                        self.autenticacao_gov
                            .as_ref()
                            .expect("validated autenticacao_gov"),
                    )?
                    .operations
            }
            Provider::Tsa => {
                RealProviderAdapter
                    .build_tsa_plan(self.tsa.as_ref().expect("validated tsa"))?
                    .operations
            }
            Provider::Hsm => {
                RealProviderAdapter
                    .build_hsm_plan(self.hsm.as_ref().expect("validated hsm"))?
                    .operations
            }
            Provider::CegerCard => {
                RealProviderAdapter
                    .build_ceger_card_plan(self.ceger_card.as_ref().expect("validated ceger_card"))?
                    .operations
            }
            Provider::CitizenCardAuth => {
                CitizenCardAuthAdapter
                    .build_plan(
                        self.citizen_card_auth
                            .as_ref()
                            .expect("validated citizen_card_auth"),
                    )?
                    .operations
            }
        };
        Ok(operations.into_iter().map(|op| op.name).collect())
    }
}

use serde::{Deserialize, Serialize};

use crate::{Result, SigningError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    QualifiedCertificate,
    Otc,
    CartaoCidadaoPt,
    Middleware,
    AutenticacaoGov,
    Tsa,
    Hsm,
    CegerCard,
    CitizenCardAuth,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QualifiedCertificate => "qualified-certificate",
            Self::Otc => "otc",
            Self::CartaoCidadaoPt => "cartao-cidadao-pt",
            Self::Middleware => "middleware",
            Self::AutenticacaoGov => "autenticacao-gov",
            Self::Tsa => "tsa",
            Self::Hsm => "hsm",
            Self::CegerCard => "ceger-card",
            Self::CitizenCardAuth => "citizen-card-auth",
        }
    }

    pub fn requires_certificate_ref(self) -> bool {
        matches!(
            self,
            Self::QualifiedCertificate
                | Self::CartaoCidadaoPt
                | Self::Middleware
                | Self::Hsm
                | Self::CegerCard
                | Self::CitizenCardAuth
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureFormat {
    Pades,
    Xades,
    Cades,
}

impl SignatureFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pades => "pades",
            Self::Xades => "xades",
            Self::Cades => "cades",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub provider: Provider,
    pub profile: String,
    pub certificate_ref: Option<String>,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
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

    pub fn requires_certificate_ref(&self) -> bool {
        self.provider.requires_certificate_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub name: String,
    pub required: bool,
}

impl Operation {
    pub fn required(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required: true,
        }
    }
}

pub(crate) fn require_non_empty(field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(SigningError::EmptyField(field));
    }
    Ok(())
}

pub(crate) fn validate_secret_ref(field: &'static str, value: &str) -> Result<()> {
    require_non_empty(field, value)?;
    let Some((scheme, path)) = value.split_once(':') else {
        return Err(SigningError::InvalidSecretRef {
            field,
            reason: "deve usar o formato scheme:ref",
        });
    };
    if scheme.is_empty() || path.is_empty() {
        return Err(SigningError::InvalidSecretRef {
            field,
            reason: "scheme e ref são obrigatórios",
        });
    }
    if !scheme
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(SigningError::InvalidSecretRef {
            field,
            reason: "scheme deve conter apenas ASCII minúsculo, dígitos ou hífen",
        });
    }
    Ok(())
}

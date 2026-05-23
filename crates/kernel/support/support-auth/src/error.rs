use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

pub const COMPONENT: &str = "support-auth";

const TOKEN_INVALID: &str = "MINI.AUTH.TOKEN_INVALID";
const TOKEN_EXPIRED: &str = "MINI.AUTH.TOKEN_EXPIRED";
const CLAIMS_INVALID: &str = "MINI.AUTH.CLAIMS_INVALID";
const METADATA_UNAVAILABLE: &str = "MINI.AUTH.METADATA_UNAVAILABLE";
const JWKS_UNAVAILABLE: &str = "MINI.AUTH.JWKS_UNAVAILABLE";
const PROVIDER_UNSUPPORTED: &str = "MINI.AUTH.PROVIDER_UNSUPPORTED";
const REQUEST_INVALID: &str = "MINI.AUTH.REQUEST_INVALID";
const OTC_INVALID: &str = "MINI.AUTH.OTC_INVALID";
const OTC_EXPIRED: &str = "MINI.AUTH.OTC_EXPIRED";
const OTC_ATTEMPTS_EXCEEDED: &str = "MINI.AUTH.OTC_ATTEMPTS_EXCEEDED";
const STATE_UNAVAILABLE: &str = "MINI.AUTH.STATE_UNAVAILABLE";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("token JWT inválido: {0}")]
    TokenInvalid(String),
    #[error("token JWT expirado")]
    TokenExpired,
    #[error("claims inválidas: {0}")]
    ClaimsInvalid(String),
    #[error("metadata OIDC indisponível: {0}")]
    MetadataUnavailable(String),
    #[error("JWKS indisponível: {0}")]
    JwksUnavailable(String),
    #[error("provider não suportado: {0}")]
    ProviderUnsupported(String),
    #[error("pedido inválido: {0}")]
    RequestInvalid(String),
    #[error("código OTC inválido")]
    OtcInvalid,
    #[error("código OTC expirado")]
    OtcExpired,
    #[error("limite de tentativas OTC excedido")]
    OtcAttemptsExceeded,
    #[error("estado OTC não encontrado")]
    StateUnavailable,
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TokenInvalid(_) => TOKEN_INVALID,
            Self::TokenExpired => TOKEN_EXPIRED,
            Self::ClaimsInvalid(_) => CLAIMS_INVALID,
            Self::MetadataUnavailable(_) => METADATA_UNAVAILABLE,
            Self::JwksUnavailable(_) => JWKS_UNAVAILABLE,
            Self::ProviderUnsupported(_) => PROVIDER_UNSUPPORTED,
            Self::RequestInvalid(_) => REQUEST_INVALID,
            Self::OtcInvalid => OTC_INVALID,
            Self::OtcExpired => OTC_EXPIRED,
            Self::OtcAttemptsExceeded => OTC_ATTEMPTS_EXCEEDED,
            Self::StateUnavailable => STATE_UNAVAILABLE,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::TokenInvalid(_) => "token de autenticação inválido",
            Self::TokenExpired => "token de autenticação expirado",
            Self::ClaimsInvalid(_) => "claims de autenticação inválidas",
            Self::MetadataUnavailable(_) => "metadata do provider indisponível",
            Self::JwksUnavailable(_) => "chaves públicas do provider indisponíveis",
            Self::ProviderUnsupported(_) => "provider de autenticação não suportado",
            Self::RequestInvalid(_) => "pedido de autenticação inválido",
            Self::OtcInvalid => "código de verificação inválido",
            Self::OtcExpired => "código de verificação expirado",
            Self::OtcAttemptsExceeded => "limite de tentativas de verificação excedido",
            Self::StateUnavailable => "estado de verificação não encontrado",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("support-auth error codes must be valid"),
            Component::new(COMPONENT).expect("support-auth component must be valid"),
            self.public_message(),
        )
    }
}

impl From<AuthError> for MiniError {
    fn from(value: AuthError) -> Self {
        value.to_mini_error()
    }
}

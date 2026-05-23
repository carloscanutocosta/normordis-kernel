use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SigningError {
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(&'static str),
    #[error("provider de assinatura não suportado: {0}")]
    UnsupportedProvider(String),
    #[error("formato de assinatura inválido: {0}")]
    InvalidSignatureFormat(String),
    #[error("delivery OTC inválido: {0}")]
    InvalidOtcDelivery(String),
    #[error("modo Cartão de Cidadão inválido: {0}")]
    InvalidCartaoCidadaoPtMode(String),
    #[error("valor inválido em {field}: {reason}")]
    InvalidValue {
        field: &'static str,
        reason: &'static str,
    },
    #[error("referência de segredo inválida em {field}: {reason}")]
    InvalidSecretRef {
        field: &'static str,
        reason: &'static str,
    },
    #[error("falha ao gerar OTC")]
    OtcGenerationFailed,
    #[error("provider externo de assinatura falhou: {0}")]
    ExternalSignerFailed(String),
}

pub type Result<T> = std::result::Result<T, SigningError>;

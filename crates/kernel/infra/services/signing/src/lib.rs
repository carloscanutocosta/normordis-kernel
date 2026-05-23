//! Adapters de infraestrutura para serviços de assinatura digital.
//!
//! Este crate materializa configurações e planos técnicos para providers de
//! assinatura. As integrações nativas ficam atrás de features explícitas para
//! manter o núcleo testável sem exigir hardware em todos os ambientes.

mod artifact;
mod cartao_cidadao_pt;
mod chave_movel_digital;
mod citizen_card;
mod command_signer;
mod config;
mod error;
mod otc;
mod pkcs11_signer;
mod qualified_certificate;
mod real_providers;
mod runtime;
mod toolchain;

pub use artifact::{
    DetachedSignature, DetachedSignatureRequest, DetachedSigningService, ExternalSigner,
    SigningEvidence,
};
pub use cartao_cidadao_pt::{
    CartaoCidadaoPtAdapter, CartaoCidadaoPtConfig, CartaoCidadaoPtMode, CartaoCidadaoPtPlan,
};
pub use chave_movel_digital::{
    ChaveMovelDigitalAdapter, ChaveMovelDigitalArtifact, ChaveMovelDigitalConfig,
    ChaveMovelDigitalGateway, ChaveMovelDigitalGatewayStartRequest,
    ChaveMovelDigitalGatewayStartResponse, ChaveMovelDigitalGatewayStatus,
    ChaveMovelDigitalGatewayStatusResponse, ChaveMovelDigitalHttpTransport, ChaveMovelDigitalPlan,
    ChaveMovelDigitalService, ChaveMovelDigitalSession, ChaveMovelDigitalUserConfirmation,
    HttpChaveMovelDigitalGateway, HttpChaveMovelDigitalGatewayConfig, MockChaveMovelDigitalGateway,
};
pub use citizen_card::{
    CitizenCardAuthAdapter, CitizenCardAuthConfig, CitizenCardAuthMode, CitizenCardAuthPlan,
    CitizenCardMiddleware, CitizenCardOfficialMiddlewareDefaults,
};
pub use command_signer::{CommandExternalSigner, CommandSignerConfig};
pub use config::{Config, Operation, Provider, SignatureFormat};
pub use error::{Result, SigningError};
pub use otc::{
    CommandOtcDeliveryConfig, CommandOtcDeliveryGateway, IssuedOtc, IssuedOtcRecord,
    MemoryOtcRecordStore, MockOtcDeliveryGateway, OtcAdapter, OtcAttempt, OtcCodeGenerator,
    OtcConfig, OtcDelivery, OtcDeliveryGateway, OtcDeliveryRequest, OtcDeliveryResult,
    OtcFlowService, OtcIssueRequest, OtcIssueResponse, OtcIssuer, OtcPlan, OtcRecordStore,
    OtcVerificationResult, OtcVerifyRequest, OtcVerifyResponse, RandomNumericCodeGenerator,
};
#[cfg(feature = "native-pkcs11")]
pub use pkcs11_signer::{probe_pkcs11_module, NativePkcs11Signer};
pub use pkcs11_signer::{
    PinResolver, Pkcs11Mechanism, Pkcs11ModuleProbe, Pkcs11ObjectProbe, Pkcs11SigningAdapter,
    Pkcs11SigningConfig, Pkcs11SigningPlan, Pkcs11SlotProbe, Pkcs11TokenProbe,
};
pub use qualified_certificate::{
    QualifiedCertificateAdapter, QualifiedCertificateConfig, QualifiedPlan,
};
pub use real_providers::{
    AutenticacaoGovConfig, AutenticacaoGovFlow, CegerCardConfig, HsmConfig, MiddlewareConfig,
    MiddlewareKind, ProviderPlan, RealProviderAdapter, TsaConfig,
};
pub use runtime::RuntimeBinding;
pub use toolchain::{
    bit4id_pkcs11_module_path, citizen_card_pkcs11_module_candidates, AutenticacaoGovToolchain,
    Bit4IdMiddlewareDefaults, ChaveMovelDigitalToolchain, CitizenCardPkcs11Defaults,
    CitizenCardPkcs11Toolchain, CommandSignerToolchain, EcceCegerToolchain, Pkcs11HsmToolchain,
};

pub(crate) use config::{require_non_empty, validate_secret_ref};

#[cfg(test)]
mod tests;

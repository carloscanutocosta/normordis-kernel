use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    require_non_empty, DetachedSignature, DetachedSignatureRequest, Operation, Provider, Result,
    SignatureFormat, SigningError, SigningEvidence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChaveMovelDigitalUserConfirmation {
    SmsOtp,
    MobileApp,
    ProviderDefault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalConfig {
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

impl ChaveMovelDigitalConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        require_non_empty("service_endpoint", &self.service_endpoint)?;
        require_non_empty("client_id", &self.client_id)?;
        if self
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

    pub fn detached_request(&self, bytes_to_sign: Vec<u8>) -> Result<DetachedSignatureRequest> {
        self.validate()?;
        Ok(DetachedSignatureRequest {
            provider: Provider::AutenticacaoGov,
            format: self.format,
            profile: self.profile.clone(),
            certificate_ref: Some("cmd:remote-qualified-certificate".into()),
            trust_service_ref: self.trust_service_ref.clone(),
            bytes_to_sign,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalPlan {
    pub provider: Provider,
    pub format: SignatureFormat,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ChaveMovelDigitalAdapter;

impl ChaveMovelDigitalAdapter {
    pub fn build_plan(&self, cfg: &ChaveMovelDigitalConfig) -> Result<ChaveMovelDigitalPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("start-cmd-remote-flow"),
            Operation::required("submit-document-hash"),
            Operation::required(match cfg.user_confirmation {
                ChaveMovelDigitalUserConfirmation::SmsOtp => "confirm-with-sms-otp",
                ChaveMovelDigitalUserConfirmation::MobileApp => "confirm-with-mobile-app",
                ChaveMovelDigitalUserConfirmation::ProviderDefault => {
                    "confirm-with-provider-default"
                }
            }),
            Operation::required("retrieve-cmd-signature-artifact"),
            Operation::required("verify-cmd-signature-evidence"),
        ];
        if cfg.require_professional_attributes {
            operations.push(Operation::required("request-scap-professional-attributes"));
        }
        if cfg.require_timestamp {
            operations.push(Operation::required("attach-qualified-timestamp"));
        }
        Ok(ChaveMovelDigitalPlan {
            provider: Provider::AutenticacaoGov,
            format: cfg.format,
            operations,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalSession {
    pub session_id: String,
    pub profile: String,
    pub format: SignatureFormat,
    pub service_endpoint: String,
    pub client_id: String,
    pub callback_url: String,
    pub signing_hash_hex: String,
    pub trust_service_ref: Option<String>,
    pub require_professional_attributes: bool,
    pub require_timestamp: bool,
    pub user_confirmation: ChaveMovelDigitalUserConfirmation,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalArtifact {
    pub session_id: String,
    pub signature_der: Vec<u8>,
    pub certificate_ref: Option<String>,
    pub algorithm: String,
    pub signed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChaveMovelDigitalGatewayStatus {
    Prepared,
    WaitingUserConfirmation,
    Completed,
    Rejected,
    Expired,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalGatewayStartRequest {
    pub session: ChaveMovelDigitalSession,
    pub subject_hint: Option<String>,
    pub document_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalGatewayStartResponse {
    pub gateway_request_id: String,
    pub status: ChaveMovelDigitalGatewayStatus,
    pub authorize_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChaveMovelDigitalGatewayStatusResponse {
    pub gateway_request_id: String,
    pub status: ChaveMovelDigitalGatewayStatus,
    pub artifact: Option<ChaveMovelDigitalArtifact>,
    pub message: Option<String>,
}

pub trait ChaveMovelDigitalGateway {
    fn start_signature(
        &self,
        request: &ChaveMovelDigitalGatewayStartRequest,
    ) -> Result<ChaveMovelDigitalGatewayStartResponse>;

    fn poll_signature(
        &self,
        session: &ChaveMovelDigitalSession,
        gateway_request_id: &str,
    ) -> Result<ChaveMovelDigitalGatewayStatusResponse>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockChaveMovelDigitalGateway {
    pub complete_immediately: bool,
    pub authorize_url_base: Option<String>,
}

impl Default for MockChaveMovelDigitalGateway {
    fn default() -> Self {
        Self {
            complete_immediately: false,
            authorize_url_base: Some("http://localhost/mock-cmd/confirm".into()),
        }
    }
}

impl ChaveMovelDigitalGateway for MockChaveMovelDigitalGateway {
    fn start_signature(
        &self,
        request: &ChaveMovelDigitalGatewayStartRequest,
    ) -> Result<ChaveMovelDigitalGatewayStartResponse> {
        let gateway_request_id = mock_gateway_request_id(&request.session);
        let authorize_url = self
            .authorize_url_base
            .as_ref()
            .map(|base| format!("{base}?request_id={gateway_request_id}"));
        Ok(ChaveMovelDigitalGatewayStartResponse {
            gateway_request_id,
            status: if self.complete_immediately {
                ChaveMovelDigitalGatewayStatus::Completed
            } else {
                ChaveMovelDigitalGatewayStatus::WaitingUserConfirmation
            },
            authorize_url,
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            message: Some("fluxo CMD mock preparado".into()),
        })
    }

    fn poll_signature(
        &self,
        session: &ChaveMovelDigitalSession,
        gateway_request_id: &str,
    ) -> Result<ChaveMovelDigitalGatewayStatusResponse> {
        let expected = mock_gateway_request_id(session);
        if gateway_request_id != expected {
            return Err(SigningError::InvalidValue {
                field: "gateway_request_id",
                reason: "pedido mock não corresponde à sessão CMD",
            });
        }

        let artifact = self
            .complete_immediately
            .then(|| ChaveMovelDigitalArtifact {
                session_id: session.session_id.clone(),
                signature_der: mock_signature_der(session, gateway_request_id),
                certificate_ref: Some("cmd:mock-qualified-certificate".into()),
                algorithm: "cmd-mock-sha256".into(),
                signed_at: Some(Utc::now()),
            });

        Ok(ChaveMovelDigitalGatewayStatusResponse {
            gateway_request_id: gateway_request_id.into(),
            status: if artifact.is_some() {
                ChaveMovelDigitalGatewayStatus::Completed
            } else {
                ChaveMovelDigitalGatewayStatus::WaitingUserConfirmation
            },
            artifact,
            message: Some("estado CMD mock".into()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpChaveMovelDigitalGatewayConfig {
    pub start_url: String,
    pub status_url: String,
    pub bearer_token_ref: Option<String>,
}

impl HttpChaveMovelDigitalGatewayConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("start_url", &self.start_url)?;
        require_non_empty("status_url", &self.status_url)?;
        Ok(())
    }
}

pub trait ChaveMovelDigitalHttpTransport {
    fn post_json(&self, url: &str, payload: &Value) -> Result<Value>;
}

#[derive(Debug, Clone)]
pub struct HttpChaveMovelDigitalGateway<T> {
    config: HttpChaveMovelDigitalGatewayConfig,
    transport: T,
}

impl<T> HttpChaveMovelDigitalGateway<T>
where
    T: ChaveMovelDigitalHttpTransport,
{
    pub fn new(config: HttpChaveMovelDigitalGatewayConfig, transport: T) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, transport })
    }
}

impl<T> ChaveMovelDigitalGateway for HttpChaveMovelDigitalGateway<T>
where
    T: ChaveMovelDigitalHttpTransport,
{
    fn start_signature(
        &self,
        request: &ChaveMovelDigitalGatewayStartRequest,
    ) -> Result<ChaveMovelDigitalGatewayStartResponse> {
        let payload = json!({
            "session": request.session,
            "subject_hint": request.subject_hint,
            "document_name": request.document_name,
            "bearer_token_ref": self.config.bearer_token_ref,
        });
        let response = self.transport.post_json(&self.config.start_url, &payload)?;
        serde_json::from_value(response)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))
    }

    fn poll_signature(
        &self,
        session: &ChaveMovelDigitalSession,
        gateway_request_id: &str,
    ) -> Result<ChaveMovelDigitalGatewayStatusResponse> {
        let payload = json!({
            "session": session,
            "gateway_request_id": gateway_request_id,
            "bearer_token_ref": self.config.bearer_token_ref,
        });
        let response = self
            .transport
            .post_json(&self.config.status_url, &payload)?;
        serde_json::from_value(response)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ChaveMovelDigitalService;

impl ChaveMovelDigitalService {
    pub fn prepare_session(
        &self,
        cfg: &ChaveMovelDigitalConfig,
        request: &DetachedSignatureRequest,
    ) -> Result<ChaveMovelDigitalSession> {
        cfg.validate()?;
        request.validate()?;
        if request.provider != Provider::AutenticacaoGov {
            return Err(SigningError::InvalidValue {
                field: "provider",
                reason: "pedido CMD deve usar provider autenticacao-gov",
            });
        }

        let created_at = Utc::now();
        let signing_hash_hex = request.signing_hash_hex();
        let session_id = cmd_session_id(cfg, &signing_hash_hex, created_at);
        Ok(ChaveMovelDigitalSession {
            session_id,
            profile: cfg.profile.clone(),
            format: cfg.format,
            service_endpoint: cfg.service_endpoint.clone(),
            client_id: cfg.client_id.clone(),
            callback_url: cfg.callback_url.clone().unwrap_or_default(),
            signing_hash_hex,
            trust_service_ref: cfg.trust_service_ref.clone(),
            require_professional_attributes: cfg.require_professional_attributes,
            require_timestamp: cfg.require_timestamp,
            user_confirmation: cfg.user_confirmation,
            created_at,
        })
    }

    pub fn materialize_signature(
        &self,
        session: &ChaveMovelDigitalSession,
        artifact: ChaveMovelDigitalArtifact,
    ) -> Result<DetachedSignature> {
        if artifact.session_id != session.session_id {
            return Err(SigningError::InvalidValue {
                field: "session_id",
                reason: "artefacto CMD não pertence à sessão indicada",
            });
        }
        if artifact.signature_der.is_empty() {
            return Err(SigningError::EmptyField("signature_der"));
        }
        require_non_empty("algorithm", &artifact.algorithm)?;

        Ok(DetachedSignature {
            format: session.format,
            algorithm: artifact.algorithm,
            signature_der: artifact.signature_der,
            certificate_ref: artifact
                .certificate_ref
                .or_else(|| Some("cmd:remote-qualified-certificate".into())),
            signed_at: artifact.signed_at.unwrap_or_else(Utc::now),
            signing_hash_hex: session.signing_hash_hex.clone(),
        })
    }

    pub fn evidence_for_signature(
        &self,
        session: &ChaveMovelDigitalSession,
        signature: &DetachedSignature,
    ) -> Result<SigningEvidence> {
        require_non_empty("algorithm", &signature.algorithm)?;
        if signature.signature_der.is_empty() {
            return Err(SigningError::EmptyField("signature_der"));
        }
        if signature.format != session.format {
            return Err(SigningError::InvalidValue {
                field: "format",
                reason: "assinatura CMD não corresponde à sessão",
            });
        }
        if signature.signing_hash_hex != session.signing_hash_hex {
            return Err(SigningError::InvalidValue {
                field: "signing_hash_hex",
                reason: "hash assinado não corresponde à sessão CMD",
            });
        }

        Ok(SigningEvidence {
            provider: Provider::AutenticacaoGov,
            format: session.format,
            profile: session.profile.clone(),
            certificate_ref: signature.certificate_ref.clone(),
            trust_service_ref: session.trust_service_ref.clone(),
            signing_hash_hex: signature.signing_hash_hex.clone(),
            signature_hash_hex: hex::encode(Sha256::digest(&signature.signature_der)),
            signed_at: signature.signed_at,
        })
    }
}

fn cmd_session_id(
    cfg: &ChaveMovelDigitalConfig,
    signing_hash_hex: &str,
    created_at: DateTime<Utc>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cfg.profile.as_bytes());
    hasher.update(cfg.service_endpoint.as_bytes());
    hasher.update(cfg.client_id.as_bytes());
    hasher.update(signing_hash_hex.as_bytes());
    hasher.update(
        created_at
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .to_string(),
    );
    format!("cmd-{}", hex::encode(hasher.finalize()))
}

fn mock_gateway_request_id(session: &ChaveMovelDigitalSession) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session.session_id.as_bytes());
    hasher.update(session.signing_hash_hex.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("cmd-mock-{}", &digest[..24])
}

fn mock_signature_der(session: &ChaveMovelDigitalSession, gateway_request_id: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(session.signing_hash_hex.as_bytes());
    hasher.update(gateway_request_id.as_bytes());
    hasher.finalize().to_vec()
}

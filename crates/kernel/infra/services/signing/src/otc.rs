use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{require_non_empty, Config, Operation, Provider, Result, SigningError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OtcDelivery {
    Sms,
    Email,
    App,
}

impl OtcDelivery {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sms => "sms",
            Self::Email => "email",
            Self::App => "app",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcConfig {
    pub profile: String,
    pub delivery: OtcDelivery,
    pub ttl_seconds: i64,
    pub max_attempts: u32,
    pub code_length: usize,
    pub bind_user_auth: bool,
}

impl OtcConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("profile", &self.profile)?;
        if self.ttl_seconds <= 0 {
            return Err(SigningError::InvalidValue {
                field: "ttl_seconds",
                reason: "deve ser maior que zero",
            });
        }
        if self.max_attempts == 0 {
            return Err(SigningError::InvalidValue {
                field: "max_attempts",
                reason: "deve ser maior que zero",
            });
        }
        if self.code_length == 0 {
            return Err(SigningError::InvalidValue {
                field: "code_length",
                reason: "deve ser maior que zero",
            });
        }
        Ok(())
    }

    pub fn base_config(&self) -> Config {
        Config {
            provider: Provider::Otc,
            profile: self.profile.clone(),
            certificate_ref: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcPlan {
    pub provider: Provider,
    pub delivery: OtcDelivery,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OtcAdapter;

impl OtcAdapter {
    pub fn build_plan(&self, cfg: &OtcConfig) -> Result<OtcPlan> {
        cfg.validate()?;
        let mut operations = vec![
            Operation::required("generate-one-time-code"),
            Operation::required("persist-ttl-and-attempt-window"),
            Operation::required("deliver-code"),
        ];
        if cfg.bind_user_auth {
            operations.push(Operation::required("bind-user-auth-context"));
        }
        operations.push(Operation::required("record-verifiable-evidence"));
        Ok(OtcPlan {
            provider: Provider::Otc,
            delivery: cfg.delivery,
            operations,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuedOtc {
    pub code: String,
    pub record: IssuedOtcRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuedOtcRecord {
    pub reference: String,
    pub profile: String,
    pub delivery: OtcDelivery,
    pub subject_ref: String,
    pub destination_ref: String,
    pub code_hash_hex: String,
    pub salt_hex: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_attempts: u32,
    pub attempt_count: u32,
    pub bind_user_auth: bool,
    pub consumed_at: Option<DateTime<Utc>>,
}

impl IssuedOtcRecord {
    pub fn record_attempt(&mut self) {
        self.attempt_count = self.attempt_count.saturating_add(1);
    }

    pub fn mark_consumed(&mut self, at: DateTime<Utc>) {
        self.consumed_at = Some(at);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcAttempt {
    pub code: String,
    pub at: Option<DateTime<Utc>>,
    pub subject_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcVerificationResult {
    pub accepted: bool,
    pub reason: String,
    pub remaining_uses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcIssueRequest {
    pub subject_ref: String,
    pub destination_ref: String,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcIssueResponse {
    pub reference: String,
    pub delivery: OtcDelivery,
    pub destination_ref: String,
    pub expires_at: DateTime<Utc>,
    pub delivered: bool,
    pub delivery_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcVerifyRequest {
    pub reference: String,
    pub subject_ref: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcVerifyResponse {
    pub reference: String,
    pub result: OtcVerificationResult,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcDeliveryRequest {
    pub reference: String,
    pub delivery: OtcDelivery,
    pub destination_ref: String,
    pub code: String,
    pub purpose: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcDeliveryResult {
    pub delivered: bool,
    pub message: Option<String>,
}

pub trait OtcRecordStore {
    fn save_record(&self, record: &IssuedOtcRecord) -> Result<()>;
    fn find_record(&self, reference: &str) -> Result<Option<IssuedOtcRecord>>;
    fn delete_record(&self, reference: &str) -> Result<()>;
}

pub trait OtcDeliveryGateway {
    fn deliver(&self, request: &OtcDeliveryRequest) -> Result<OtcDeliveryResult>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOtcDeliveryConfig {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandOtcDeliveryConfig {
    pub fn validate(&self) -> Result<()> {
        require_non_empty("program", &self.program)
    }
}

#[derive(Debug, Clone)]
pub struct CommandOtcDeliveryGateway {
    config: CommandOtcDeliveryConfig,
}

impl CommandOtcDeliveryGateway {
    pub fn new(config: CommandOtcDeliveryConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }
}

impl OtcDeliveryGateway for CommandOtcDeliveryGateway {
    fn deliver(&self, request: &OtcDeliveryRequest) -> Result<OtcDeliveryResult> {
        let payload = serde_json::to_vec(request)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        let mut child = Command::new(&self.config.program)
            .args(&self.config.args)
            .env("MINI_OTC_REFERENCE", &request.reference)
            .env("MINI_OTC_DELIVERY", request.delivery.as_str())
            .env("MINI_OTC_DESTINATION_REF", &request.destination_ref)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            SigningError::ExternalSignerFailed("stdin indisponível no gateway OTC".into())
        })?;
        stdin
            .write_all(&payload)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SigningError::ExternalSignerFailed(
                stderr.trim().to_string(),
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct MemoryOtcRecordStore {
    records: Mutex<HashMap<String, IssuedOtcRecord>>,
}

impl MemoryOtcRecordStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OtcRecordStore for MemoryOtcRecordStore {
    fn save_record(&self, record: &IssuedOtcRecord) -> Result<()> {
        self.records
            .lock()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
            .insert(record.reference.clone(), record.clone());
        Ok(())
    }

    fn find_record(&self, reference: &str) -> Result<Option<IssuedOtcRecord>> {
        Ok(self
            .records
            .lock()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
            .get(reference)
            .cloned())
    }

    fn delete_record(&self, reference: &str) -> Result<()> {
        self.records
            .lock()
            .map_err(|e| SigningError::ExternalSignerFailed(e.to_string()))?
            .remove(reference);
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MockOtcDeliveryGateway;

impl OtcDeliveryGateway for MockOtcDeliveryGateway {
    fn deliver(&self, request: &OtcDeliveryRequest) -> Result<OtcDeliveryResult> {
        require_non_empty("destination_ref", &request.destination_ref)?;
        Ok(OtcDeliveryResult {
            delivered: true,
            message: Some(format!(
                "mock-otc:{}:{}:{}",
                request.delivery.as_str(),
                request.reference,
                request.code
            )),
        })
    }
}

pub trait OtcCodeGenerator {
    fn generate_numeric_code(&mut self, length: usize) -> Result<String>;
    fn generate_salt(&mut self) -> Result<[u8; 16]>;
}

#[derive(Debug, Default)]
pub struct RandomNumericCodeGenerator;

impl OtcCodeGenerator for RandomNumericCodeGenerator {
    fn generate_numeric_code(&mut self, length: usize) -> Result<String> {
        if length == 0 {
            return Err(SigningError::InvalidValue {
                field: "length",
                reason: "deve ser maior que zero",
            });
        }
        let mut rng = OsRng;
        let code: String = (0..length)
            .map(|_| char::from(b'0' + rng.gen_range(0..10)))
            .collect();
        Ok(code)
    }

    fn generate_salt(&mut self) -> Result<[u8; 16]> {
        let mut rng = OsRng;
        let mut salt = [0u8; 16];
        rng.fill(&mut salt);
        Ok(salt)
    }
}

#[derive(Debug)]
pub struct OtcIssuer<G = RandomNumericCodeGenerator> {
    generator: G,
    now: fn() -> DateTime<Utc>,
}

impl Default for OtcIssuer<RandomNumericCodeGenerator> {
    fn default() -> Self {
        Self::new(RandomNumericCodeGenerator)
    }
}

impl<G> OtcIssuer<G>
where
    G: OtcCodeGenerator,
{
    pub fn new(generator: G) -> Self {
        Self {
            generator,
            now: Utc::now,
        }
    }

    pub fn with_clock(mut self, now: fn() -> DateTime<Utc>) -> Self {
        self.now = now;
        self
    }

    pub fn issue(&mut self, cfg: &OtcConfig, subject_ref: impl Into<String>) -> Result<IssuedOtc> {
        self.issue_for_destination(cfg, subject_ref, "local:unknown")
    }

    pub fn issue_for_destination(
        &mut self,
        cfg: &OtcConfig,
        subject_ref: impl Into<String>,
        destination_ref: impl Into<String>,
    ) -> Result<IssuedOtc> {
        cfg.validate()?;
        let subject_ref = subject_ref.into();
        let destination_ref = destination_ref.into();
        require_non_empty("subject_ref", &subject_ref)?;
        require_non_empty("destination_ref", &destination_ref)?;
        let issued_at = (self.now)();
        let code = self.generator.generate_numeric_code(cfg.code_length)?;
        let salt = self.generator.generate_salt()?;
        let code_hash_hex = hash_otc_code(&salt, &code, &subject_ref);
        let reference = otc_reference(&cfg.profile, &subject_ref, &destination_ref, issued_at);
        Ok(IssuedOtc {
            code: code.clone(),
            record: IssuedOtcRecord {
                reference,
                profile: cfg.profile.clone(),
                delivery: cfg.delivery,
                subject_ref,
                destination_ref,
                code_hash_hex,
                salt_hex: hex::encode(salt),
                issued_at,
                expires_at: issued_at + Duration::seconds(cfg.ttl_seconds),
                max_attempts: cfg.max_attempts,
                attempt_count: 0,
                bind_user_auth: cfg.bind_user_auth,
                consumed_at: None,
            },
        })
    }

    pub fn verify(&self, issued: &IssuedOtcRecord, attempt: &OtcAttempt) -> OtcVerificationResult {
        let at = attempt.at.unwrap_or_else(|| (self.now)());
        if issued.consumed_at.is_some() {
            return rejected("already-consumed", 0);
        }
        if at > issued.expires_at {
            return rejected(
                "expired",
                issued.max_attempts.saturating_sub(issued.attempt_count),
            );
        }
        if issued.attempt_count >= issued.max_attempts {
            return rejected("attempt-limit-reached", 0);
        }
        let remaining_after_attempt = issued
            .max_attempts
            .saturating_sub(issued.attempt_count.saturating_add(1));
        if issued.bind_user_auth && attempt.subject_ref != issued.subject_ref {
            return rejected("subject-mismatch", remaining_after_attempt);
        }
        let Ok(salt) = hex::decode(&issued.salt_hex) else {
            return rejected("invalid-record", remaining_after_attempt);
        };
        let attempted_hash = hash_otc_code(&salt, &attempt.code, &issued.subject_ref);
        if attempted_hash
            .as_bytes()
            .ct_eq(issued.code_hash_hex.as_bytes())
            .unwrap_u8()
            != 1
        {
            return rejected("code-mismatch", remaining_after_attempt);
        }
        OtcVerificationResult {
            accepted: true,
            reason: "accepted".into(),
            remaining_uses: remaining_after_attempt,
        }
    }
}

#[derive(Debug)]
pub struct OtcFlowService<G = RandomNumericCodeGenerator> {
    issuer: OtcIssuer<G>,
}

impl Default for OtcFlowService<RandomNumericCodeGenerator> {
    fn default() -> Self {
        Self::new(OtcIssuer::default())
    }
}

impl<G> OtcFlowService<G>
where
    G: OtcCodeGenerator,
{
    pub fn new(issuer: OtcIssuer<G>) -> Self {
        Self { issuer }
    }

    pub fn issue<S, D>(
        &mut self,
        cfg: &OtcConfig,
        store: &S,
        delivery: &D,
        request: OtcIssueRequest,
    ) -> Result<OtcIssueResponse>
    where
        S: OtcRecordStore,
        D: OtcDeliveryGateway,
    {
        require_non_empty("purpose", &request.purpose)?;
        let issued = self.issuer.issue_for_destination(
            cfg,
            request.subject_ref,
            request.destination_ref.clone(),
        )?;
        store.save_record(&issued.record)?;
        let delivery_result = delivery.deliver(&OtcDeliveryRequest {
            reference: issued.record.reference.clone(),
            delivery: issued.record.delivery,
            destination_ref: issued.record.destination_ref.clone(),
            code: issued.code,
            purpose: request.purpose,
            expires_at: issued.record.expires_at,
        })?;

        Ok(OtcIssueResponse {
            reference: issued.record.reference,
            delivery: cfg.delivery,
            destination_ref: request.destination_ref,
            expires_at: issued.record.expires_at,
            delivered: delivery_result.delivered,
            delivery_message: delivery_result.message,
        })
    }

    pub fn verify<S>(&self, store: &S, request: OtcVerifyRequest) -> Result<OtcVerifyResponse>
    where
        S: OtcRecordStore,
    {
        require_non_empty("reference", &request.reference)?;
        let Some(mut record) = store.find_record(&request.reference)? else {
            return Ok(OtcVerifyResponse {
                reference: request.reference,
                result: rejected("unknown-reference", 0),
                consumed: false,
            });
        };

        let result = self.issuer.verify(
            &record,
            &OtcAttempt {
                code: request.code,
                at: None,
                subject_ref: request.subject_ref,
            },
        );
        record.record_attempt();

        let consumed = result.accepted;
        if consumed {
            record.mark_consumed((self.issuer.now)());
            store.delete_record(&record.reference)?;
        } else {
            store.save_record(&record)?;
        }

        Ok(OtcVerifyResponse {
            reference: request.reference,
            result,
            consumed,
        })
    }
}

fn hash_otc_code(salt: &[u8], code: &str, subject_ref: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mini-kernel:infra-signing:otc:v1");
    hasher.update(salt);
    hasher.update(subject_ref.as_bytes());
    hasher.update([0]);
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

fn otc_reference(
    profile: &str,
    subject_ref: &str,
    destination_ref: &str,
    issued_at: DateTime<Utc>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mini-kernel:infra-signing:otc-reference:v1");
    hasher.update(profile.as_bytes());
    hasher.update([0]);
    hasher.update(subject_ref.as_bytes());
    hasher.update([0]);
    hasher.update(destination_ref.as_bytes());
    hasher.update([0]);
    hasher.update(
        issued_at
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .to_string()
            .as_bytes(),
    );
    let digest = hex::encode(hasher.finalize());
    format!("otc-{}", &digest[..32])
}

fn rejected(reason: &str, remaining_uses: u32) -> OtcVerificationResult {
    OtcVerificationResult {
        accepted: false,
        reason: reason.into(),
        remaining_uses,
    }
}

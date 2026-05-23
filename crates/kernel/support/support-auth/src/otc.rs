use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::error::AuthError;

// ── Tipos públicos ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OtcDelivery {
    Sms,
    Email,
    App,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtcConfig {
    pub profile: String,
    pub delivery: OtcDelivery,
    pub ttl: Duration,
    pub max_attempts: u32,
    /// Número de dígitos do código gerado (2–12).
    pub code_length: usize,
    /// Quando `true`, a verificação rejeita se `subject_ref` não coincidir com
    /// o guardado no estado.
    pub bind_user_auth: bool,
}

/// Código emitido. O campo `code` está em claro — apenas neste artefacto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedOtc {
    pub reference: String,
    pub code: String,
    pub profile: String,
    pub delivery: OtcDelivery,
    pub subject_ref: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_attempts: u32,
    pub bind_user_auth: bool,
}

/// Estado persistível de um OTC emitido. O código nunca aparece em claro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcState {
    pub reference: String,
    pub code_hash: String,
    pub code_salt: String,
    pub profile: String,
    pub delivery: OtcDelivery,
    pub subject_ref: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_attempts: u32,
    pub attempt_count: u32,
    pub bind_user_auth: bool,
    pub last_attempt_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtcVerificationResult {
    pub accepted: bool,
    pub reason: String,
    pub remaining_attempts: u32,
    pub used_at: Option<DateTime<Utc>>,
    pub should_delete: bool,
    pub updated_state: OtcState,
}

// ── Port traits ───────────────────────────────────────────────────────────────

pub trait OtcStateStore: Send + Sync {
    fn save_state(&self, state: &OtcState) -> Result<(), AuthError>;
    fn find_state(&self, reference: &str) -> Result<Option<OtcState>, AuthError>;
    fn delete_state(&self, reference: &str) -> Result<(), AuthError>;
}

/// Permite injectar a geração de códigos e salts (testabilidade determinística).
pub trait CodeGenerator: Send + Sync {
    fn generate_code(&self, length: usize) -> String;
    fn generate_salt(&self) -> String;
}

pub struct DefaultCodeGenerator;

impl CodeGenerator for DefaultCodeGenerator {
    fn generate_code(&self, length: usize) -> String {
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| rng.gen_range(0u8..10).to_string())
            .collect()
    }
    fn generate_salt(&self) -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }
}

fn _assert_otc_service_send_sync()
where
    OtcService: Send + Sync,
{
}

// ── OtcService ────────────────────────────────────────────────────────────────

/// Serviço técnico de emissão e verificação de códigos OTC.
///
/// Toda a lógica é pura — o serviço não persiste estado por si próprio.
pub struct OtcService {
    config: OtcConfig,
    generator: Box<dyn CodeGenerator>,
}

impl OtcService {
    pub fn new(config: OtcConfig) -> Result<Self, AuthError> {
        Self::with_generator(config, Box::new(DefaultCodeGenerator))
    }

    pub fn with_generator(
        config: OtcConfig,
        generator: Box<dyn CodeGenerator>,
    ) -> Result<Self, AuthError> {
        if config.profile.trim().is_empty() {
            return Err(AuthError::RequestInvalid("perfil OTC obrigatório".into()));
        }
        if !(2..=12).contains(&config.code_length) {
            return Err(AuthError::RequestInvalid(
                "code_length deve estar entre 2 e 12".into(),
            ));
        }
        if config.max_attempts == 0 {
            return Err(AuthError::RequestInvalid(
                "max_attempts deve ser pelo menos 1".into(),
            ));
        }
        Ok(Self { config, generator })
    }

    /// Emite um novo OTC. Devolve o artefacto de emissão (com código em claro)
    /// e o estado persistível (com hash+salt).
    pub fn issue(
        &self,
        subject_ref: &str,
        now: DateTime<Utc>,
    ) -> Result<(IssuedOtc, OtcState), AuthError> {
        if subject_ref.trim().is_empty() {
            return Err(AuthError::RequestInvalid("subject_ref obrigatório".into()));
        }

        let reference = Uuid::new_v4().to_string();
        let code = self.generator.generate_code(self.config.code_length);
        let salt = self.generator.generate_salt();
        let code_hash = hash_code(&code, &salt);
        let expires_at = now + self.config.ttl;

        let issued = IssuedOtc {
            reference: reference.clone(),
            code,
            profile: self.config.profile.clone(),
            delivery: self.config.delivery.clone(),
            subject_ref: subject_ref.to_owned(),
            issued_at: now,
            expires_at,
            max_attempts: self.config.max_attempts,
            bind_user_auth: self.config.bind_user_auth,
        };

        let state = OtcState {
            reference,
            code_hash,
            code_salt: salt,
            profile: self.config.profile.clone(),
            delivery: self.config.delivery.clone(),
            subject_ref: subject_ref.to_owned(),
            issued_at: now,
            expires_at,
            max_attempts: self.config.max_attempts,
            attempt_count: 0,
            bind_user_auth: self.config.bind_user_auth,
            last_attempt_at: None,
        };

        Ok((issued, state))
    }

    /// Verifica um código OTC contra um estado já lido do store. Devolve o
    /// resultado com o estado actualizado — o caller é responsável por persistir.
    pub fn verify(
        &self,
        state: &OtcState,
        code: &str,
        subject_ref: &str,
        now: DateTime<Utc>,
    ) -> Result<OtcVerificationResult, AuthError> {
        if now > state.expires_at {
            return Err(AuthError::OtcExpired);
        }
        if state.attempt_count >= state.max_attempts {
            return Err(AuthError::OtcAttemptsExceeded);
        }

        let mut updated = state.clone();
        updated.attempt_count += 1;
        updated.last_attempt_at = Some(now);

        if state.bind_user_auth && state.subject_ref != subject_ref {
            let remaining = state.max_attempts.saturating_sub(updated.attempt_count);
            return Ok(OtcVerificationResult {
                accepted: false,
                reason: "subject_mismatch".into(),
                remaining_attempts: remaining,
                used_at: None,
                should_delete: false,
                updated_state: updated,
            });
        }

        let expected_hash = hash_code(code, &state.code_salt);
        let hashes_match: bool = expected_hash
            .as_bytes()
            .ct_eq(state.code_hash.as_bytes())
            .into();
        if !hashes_match {
            let remaining = state.max_attempts.saturating_sub(updated.attempt_count);
            let should_delete = updated.attempt_count >= state.max_attempts;
            return Ok(OtcVerificationResult {
                accepted: false,
                reason: "code_invalid".into(),
                remaining_attempts: remaining,
                used_at: None,
                should_delete,
                updated_state: updated,
            });
        }

        Ok(OtcVerificationResult {
            accepted: true,
            reason: "accepted".into(),
            remaining_attempts: 0,
            used_at: Some(now),
            should_delete: true,
            updated_state: updated,
        })
    }
}

// ── Funções de fluxo (com store) ──────────────────────────────────────────────

/// Emite e persiste imediatamente o estado. Devolve apenas o artefacto de
/// emissão com o código em claro.
pub fn issue_flow(
    service: &OtcService,
    store: &dyn OtcStateStore,
    subject_ref: &str,
    now: DateTime<Utc>,
) -> Result<IssuedOtc, AuthError> {
    let (issued, state) = service.issue(subject_ref, now)?;
    store
        .save_state(&state)
        .map_err(|_| AuthError::StateUnavailable)?;
    Ok(issued)
}

/// Lê o estado do store, verifica e actualiza a persistência conforme o
/// resultado.
pub fn verify_flow(
    service: &OtcService,
    store: &dyn OtcStateStore,
    reference: &str,
    code: &str,
    subject_ref: &str,
    now: DateTime<Utc>,
) -> Result<OtcVerificationResult, AuthError> {
    let state = store
        .find_state(reference)
        .map_err(|_| AuthError::StateUnavailable)?
        .ok_or(AuthError::StateUnavailable)?;

    let result = service.verify(&state, code, subject_ref, now)?;

    if result.should_delete {
        let _ = store.delete_state(reference);
    } else {
        let _ = store.save_state(&result.updated_state);
    }

    Ok(result)
}

// ── Auxiliares ────────────────────────────────────────────────────────────────

fn hash_code(code: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

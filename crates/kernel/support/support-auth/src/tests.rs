use chrono::{Duration, TimeZone, Utc};

use crate::error::AuthError;
use crate::oidc::{Jwk, Jwks, OidcConfig, OidcFetcher, OidcService, ProviderMetadata};
use crate::otc::{
    issue_flow, verify_flow, CodeGenerator, OtcConfig, OtcDelivery, OtcService, OtcState,
    OtcStateStore,
};

// ── Helpers OIDC ──────────────────────────────────────────────────────────────

struct StaticFetcher {
    metadata: ProviderMetadata,
    jwks: Jwks,
}

impl OidcFetcher for StaticFetcher {
    fn fetch_metadata(&self, _url: &str) -> Result<ProviderMetadata, AuthError> {
        Ok(self.metadata.clone())
    }
    fn fetch_jwks(&self, _url: &str) -> Result<Jwks, AuthError> {
        Ok(self.jwks.clone())
    }
}

fn dummy_config() -> OidcConfig {
    OidcConfig {
        issuer: "https://issuer.example".into(),
        audience: "my-client".into(),
        metadata_url: Some("https://issuer.example/.well-known/openid-configuration".into()),
        jwks_url: None,
        allowed_clock_skew: std::time::Duration::from_secs(30),
        metadata_ttl: std::time::Duration::from_secs(300),
        jwks_ttl: std::time::Duration::from_secs(300),
        allow_insecure_http: false,
        fetch_retries: 0,
        fetch_retry_delay_ms: 0,
    }
}

fn dummy_metadata() -> ProviderMetadata {
    ProviderMetadata {
        issuer: "https://issuer.example".into(),
        jwks_uri: "https://issuer.example/.well-known/jwks.json".into(),
        authorization_endpoint: None,
        token_endpoint: None,
    }
}

fn dummy_jwks_empty() -> Jwks {
    Jwks { keys: vec![] }
}

// ── Testes OIDC — parsing ─────────────────────────────────────────────────────

#[test]
fn oidc_config_validate_rejects_empty_issuer() {
    let mut cfg = dummy_config();
    cfg.issuer = "".into();
    assert_eq!(
        cfg.validate(),
        Err(AuthError::ProviderUnsupported(
            "issuer OIDC obrigatório".into()
        ))
    );
}

#[test]
fn oidc_config_validate_rejects_empty_audience() {
    let mut cfg = dummy_config();
    cfg.audience = "".into();
    assert_eq!(
        cfg.validate(),
        Err(AuthError::ProviderUnsupported(
            "audience OIDC obrigatória".into()
        ))
    );
}

#[test]
fn oidc_config_validate_rejects_no_url() {
    let mut cfg = dummy_config();
    cfg.metadata_url = None;
    cfg.jwks_url = None;
    assert_eq!(
        cfg.validate(),
        Err(AuthError::ProviderUnsupported(
            "metadata_url ou jwks_url OIDC obrigatório".into()
        ))
    );
}

#[test]
fn oidc_config_validate_rejects_http_metadata_url() {
    let mut cfg = dummy_config();
    cfg.metadata_url = Some("http://insecure.example/.well-known/openid-configuration".into());
    assert!(matches!(
        cfg.validate(),
        Err(AuthError::ProviderUnsupported(_))
    ));
}

#[test]
fn oidc_service_new_rejects_empty_jwks_from_fetcher() {
    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: dummy_jwks_empty(),
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();
    let err = svc.resolve_jwks().unwrap_err();
    assert!(matches!(err, AuthError::JwksUnavailable(_)));
}

#[test]
fn oidc_validate_token_rejects_empty_token() {
    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: dummy_jwks_empty(),
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();
    let now = Utc::now();
    assert!(matches!(
        svc.validate_token("", now),
        Err(AuthError::TokenInvalid(_))
    ));
}

#[test]
fn oidc_validate_token_rejects_invalid_format() {
    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: dummy_jwks_empty(),
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();
    let now = Utc::now();
    assert!(matches!(
        svc.validate_token("not.a.valid.jwt.with.too.many.parts", now),
        Err(AuthError::TokenInvalid(_))
    ));
}

#[test]
fn oidc_validate_token_rejects_malformed_header() {
    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: dummy_jwks_empty(),
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();
    let now = Utc::now();
    assert!(matches!(
        svc.validate_token("!!!.payload.sig", now),
        Err(AuthError::TokenInvalid(_))
    ));
}

// ── Teste OIDC end-to-end com chave RSA gerada ────────────────────────────────

#[test]
fn oidc_validate_token_accepts_valid_rs256_jwt() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::Sha256;

    let mut rng = rand::thread_rng();
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let pub_key = RsaPublicKey::from(&priv_key);

    let n = URL_SAFE_NO_PAD.encode(pub_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(pub_key.e().to_bytes_be());

    let jwk = Jwk {
        key_id: "test-kid".into(),
        key_type: "RSA".into(),
        use_: "sig".into(),
        algorithm: "RS256".into(),
        modulus: n,
        exponent: e,
        curve: "".into(),
        x: "".into(),
        y: "".into(),
    };

    let now = Utc::now();
    let exp = now + Duration::minutes(5);

    let header = json!({ "alg": "RS256", "kid": "test-kid", "typ": "JWT" });
    let payload = json!({
        "iss": "https://issuer.example",
        "aud": "my-client",
        "sub": "user-42",
        "iat": now.timestamp(),
        "exp": exp.timestamp(),
    });

    let h_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let p_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let signing_input = format!("{}.{}", h_enc, p_enc);

    let signing_key = SigningKey::<Sha256>::new(priv_key);
    let sig = signing_key.sign_with_rng(&mut rng, signing_input.as_bytes());
    let s_enc = URL_SAFE_NO_PAD.encode(&*sig.to_bytes());
    let token = format!("{}.{}", signing_input, s_enc);

    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: Jwks { keys: vec![jwk] },
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();

    let principal = svc.validate_token(&token, now).unwrap();
    assert_eq!(principal.subject, "user-42");
    assert_eq!(principal.issuer, "https://issuer.example");
}

#[test]
fn oidc_validate_token_rejects_expired_rs256_jwt() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::json;
    use sha2::Sha256;

    let mut rng = rand::thread_rng();
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let pub_key = RsaPublicKey::from(&priv_key);
    let n = URL_SAFE_NO_PAD.encode(pub_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(pub_key.e().to_bytes_be());

    let jwk = Jwk {
        key_id: "kid2".into(),
        key_type: "RSA".into(),
        use_: "".into(),
        algorithm: "RS256".into(),
        modulus: n,
        exponent: e,
        curve: "".into(),
        x: "".into(),
        y: "".into(),
    };

    let past = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let exp = past + Duration::minutes(5);
    let now = Utc::now();

    let header = json!({ "alg": "RS256", "kid": "kid2", "typ": "JWT" });
    let payload = json!({
        "iss": "https://issuer.example",
        "aud": "my-client",
        "sub": "user-x",
        "iat": past.timestamp(),
        "exp": exp.timestamp(),
    });

    let h_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let p_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let signing_input = format!("{}.{}", h_enc, p_enc);
    let signing_key = SigningKey::<Sha256>::new(priv_key);
    let sig = signing_key.sign_with_rng(&mut rng, signing_input.as_bytes());
    let token = format!(
        "{}.{}",
        signing_input,
        URL_SAFE_NO_PAD.encode(&*sig.to_bytes())
    );

    let cfg = dummy_config();
    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: Jwks { keys: vec![jwk] },
    };
    let svc = OidcService::new(cfg, Box::new(fetcher), None).unwrap();

    assert!(matches!(
        svc.validate_token(&token, now),
        Err(AuthError::TokenExpired)
    ));
}

// ── Teste OIDC end-to-end com ES256 ──────────────────────────────────────────

#[test]
fn oidc_validate_token_accepts_valid_es256_jwt() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use p256::ecdsa::signature::RandomizedSigner;
    use p256::ecdsa::SigningKey;
    use serde_json::json;

    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::random(&mut rng);
    let verifying_key = signing_key.verifying_key();
    let point = verifying_key.to_encoded_point(false);
    let x_enc = URL_SAFE_NO_PAD.encode(point.x().unwrap());
    let y_enc = URL_SAFE_NO_PAD.encode(point.y().unwrap());

    let jwk = Jwk {
        key_id: "es256-kid".into(),
        key_type: "EC".into(),
        use_: "sig".into(),
        algorithm: "ES256".into(),
        modulus: "".into(),
        exponent: "".into(),
        curve: "P-256".into(),
        x: x_enc,
        y: y_enc,
    };

    let now = Utc::now();
    let exp = now + Duration::minutes(5);
    let header = json!({ "alg": "ES256", "kid": "es256-kid", "typ": "JWT" });
    let payload = json!({
        "iss": "https://issuer.example",
        "aud": "my-client",
        "sub": "user-es256",
        "iat": now.timestamp(),
        "exp": exp.timestamp(),
    });

    let h_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let p_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let signing_input = format!("{}.{}", h_enc, p_enc);
    let sig: p256::ecdsa::Signature = signing_key.sign_with_rng(&mut rng, signing_input.as_bytes());
    let token = format!(
        "{}.{}",
        signing_input,
        URL_SAFE_NO_PAD.encode(&*sig.to_bytes())
    );

    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: Jwks { keys: vec![jwk] },
    };
    let svc = OidcService::new(dummy_config(), Box::new(fetcher), None).unwrap();
    let principal = svc.validate_token(&token, now).unwrap();
    assert_eq!(principal.subject, "user-es256");
}

#[test]
fn oidc_validate_token_accepts_valid_eddsa_jwt() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey as Ed25519SigningKey};
    use serde_json::json;

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret: [u8; 32] = rng.gen();
    let signing_key = Ed25519SigningKey::from_bytes(&secret);
    let verifying_key = signing_key.verifying_key();
    let x_enc = URL_SAFE_NO_PAD.encode(verifying_key.as_bytes());

    let jwk = Jwk {
        key_id: "eddsa-kid".into(),
        key_type: "OKP".into(),
        use_: "sig".into(),
        algorithm: "EdDSA".into(),
        modulus: "".into(),
        exponent: "".into(),
        curve: "Ed25519".into(),
        x: x_enc,
        y: "".into(),
    };

    let now = Utc::now();
    let exp = now + Duration::minutes(5);
    let header = json!({ "alg": "EdDSA", "kid": "eddsa-kid", "typ": "JWT" });
    let payload = json!({
        "iss": "https://issuer.example",
        "aud": "my-client",
        "sub": "user-eddsa",
        "iat": now.timestamp(),
        "exp": exp.timestamp(),
    });

    let h_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header).unwrap());
    let p_enc = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload).unwrap());
    let signing_input = format!("{}.{}", h_enc, p_enc);
    let sig = signing_key.sign(signing_input.as_bytes());
    let token = format!(
        "{}.{}",
        signing_input,
        URL_SAFE_NO_PAD.encode(sig.to_bytes())
    );

    let fetcher = StaticFetcher {
        metadata: dummy_metadata(),
        jwks: Jwks { keys: vec![jwk] },
    };
    let svc = OidcService::new(dummy_config(), Box::new(fetcher), None).unwrap();
    let principal = svc.validate_token(&token, now).unwrap();
    assert_eq!(principal.subject, "user-eddsa");
}

// ── Testes OTC ────────────────────────────────────────────────────────────────

fn otc_config() -> OtcConfig {
    OtcConfig {
        profile: "email-verify".into(),
        delivery: OtcDelivery::Email,
        ttl: Duration::minutes(10),
        max_attempts: 3,
        code_length: 6,
        bind_user_auth: false,
    }
}

#[test]
fn otc_service_rejects_empty_profile() {
    let mut cfg = otc_config();
    cfg.profile = "".into();
    assert!(matches!(
        OtcService::new(cfg),
        Err(AuthError::RequestInvalid(_))
    ));
}

#[test]
fn otc_service_rejects_zero_max_attempts() {
    let mut cfg = otc_config();
    cfg.max_attempts = 0;
    assert!(matches!(
        OtcService::new(cfg),
        Err(AuthError::RequestInvalid(_))
    ));
}

#[test]
fn otc_service_rejects_invalid_code_length() {
    let mut cfg = otc_config();
    cfg.code_length = 1;
    assert!(matches!(
        OtcService::new(cfg),
        Err(AuthError::RequestInvalid(_))
    ));
}

#[test]
fn otc_issue_and_verify_success() {
    let svc = OtcService::new(otc_config()).unwrap();
    let now = Utc::now();
    let (issued, state) = svc.issue("user-123", now).unwrap();

    assert_eq!(issued.subject_ref, "user-123");
    assert_eq!(issued.code.len(), 6);
    assert!(issued.code.chars().all(|c| c.is_ascii_digit()));
    assert!(!state.code_hash.is_empty());
    assert_ne!(state.code_hash, issued.code);

    let result = svc.verify(&state, &issued.code, "user-123", now).unwrap();
    assert!(result.accepted);
    assert_eq!(result.reason, "accepted");
    assert!(result.should_delete);
    assert!(result.used_at.is_some());
}

#[test]
fn otc_verify_wrong_code_decrements_attempts() {
    let svc = OtcService::new(otc_config()).unwrap();
    let now = Utc::now();
    let (_, state) = svc.issue("user-123", now).unwrap();

    let result = svc.verify(&state, "000000", "user-123", now).unwrap();
    assert!(!result.accepted);
    assert_eq!(result.reason, "code_invalid");
    assert_eq!(result.updated_state.attempt_count, 1);
    assert_eq!(result.remaining_attempts, 2);
    assert!(!result.should_delete);
}

#[test]
fn otc_verify_exhausted_attempts_marks_delete() {
    let svc = OtcService::new(otc_config()).unwrap();
    let now = Utc::now();
    let (_, mut state) = svc.issue("user-123", now).unwrap();
    state.attempt_count = 2; // 1 abaixo do limite

    let result = svc.verify(&state, "000000", "user-123", now).unwrap();
    assert!(!result.accepted);
    assert!(result.should_delete);
    assert_eq!(result.remaining_attempts, 0);
}

#[test]
fn otc_verify_expired_returns_error() {
    let svc = OtcService::new(otc_config()).unwrap();
    let past = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let (_, state) = svc.issue("user-123", past).unwrap();
    let now = Utc::now();
    assert_eq!(
        svc.verify(&state, "123456", "user-123", now),
        Err(AuthError::OtcExpired)
    );
}

#[test]
fn otc_verify_max_attempts_exceeded_returns_error() {
    let svc = OtcService::new(otc_config()).unwrap();
    let now = Utc::now();
    let (_, mut state) = svc.issue("user-123", now).unwrap();
    state.attempt_count = 3; // igual a max_attempts

    assert_eq!(
        svc.verify(&state, "123456", "user-123", now),
        Err(AuthError::OtcAttemptsExceeded)
    );
}

#[test]
fn otc_verify_subject_mismatch_rejects() {
    let mut cfg = otc_config();
    cfg.bind_user_auth = true;
    let svc = OtcService::new(cfg).unwrap();
    let now = Utc::now();
    let (issued, state) = svc.issue("user-A", now).unwrap();

    let result = svc.verify(&state, &issued.code, "user-B", now).unwrap();
    assert!(!result.accepted);
    assert_eq!(result.reason, "subject_mismatch");
}

// ── Teste OTC com CodeGenerator determinístico ────────────────────────────────

struct FixedCodeGenerator {
    code: &'static str,
    salt: &'static str,
}

impl CodeGenerator for FixedCodeGenerator {
    fn generate_code(&self, _length: usize) -> String {
        self.code.into()
    }
    fn generate_salt(&self) -> String {
        self.salt.into()
    }
}

#[test]
fn otc_with_fixed_generator_produces_expected_code() {
    let svc = OtcService::with_generator(
        otc_config(),
        Box::new(FixedCodeGenerator {
            code: "123456",
            salt: "deadbeef",
        }),
    )
    .unwrap();
    let now = Utc::now();
    let (issued, _state) = svc.issue("user-det", now).unwrap();
    assert_eq!(issued.code, "123456");
}

// ── Testes OTC flow com store em memória ─────────────────────────────────────

use std::collections::HashMap;
use std::sync::Mutex;

struct MemOtcStore(Mutex<HashMap<String, OtcState>>);

impl MemOtcStore {
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl OtcStateStore for MemOtcStore {
    fn save_state(&self, state: &OtcState) -> Result<(), AuthError> {
        self.0
            .lock()
            .unwrap()
            .insert(state.reference.clone(), state.clone());
        Ok(())
    }
    fn find_state(&self, reference: &str) -> Result<Option<OtcState>, AuthError> {
        Ok(self.0.lock().unwrap().get(reference).cloned())
    }
    fn delete_state(&self, reference: &str) -> Result<(), AuthError> {
        self.0.lock().unwrap().remove(reference);
        Ok(())
    }
}

#[test]
fn otc_flow_issue_then_verify_cleans_up() {
    let svc = OtcService::new(otc_config()).unwrap();
    let store = MemOtcStore::new();
    let now = Utc::now();

    let issued = issue_flow(&svc, &store, "user-X", now).unwrap();
    assert!(store.find_state(&issued.reference).unwrap().is_some());

    let result = verify_flow(&svc, &store, &issued.reference, &issued.code, "user-X", now).unwrap();
    assert!(result.accepted);
    assert!(store.find_state(&issued.reference).unwrap().is_none());
}

#[test]
fn otc_flow_verify_unknown_reference_returns_state_unavailable() {
    let svc = OtcService::new(otc_config()).unwrap();
    let store = MemOtcStore::new();
    let now = Utc::now();

    assert_eq!(
        verify_flow(&svc, &store, "unknown-ref", "123456", "user-X", now),
        Err(AuthError::StateUnavailable)
    );
}

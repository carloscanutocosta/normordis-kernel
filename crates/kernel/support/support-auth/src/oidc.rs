use std::sync::RwLock;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier;
use rsa::{BigUint, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Sha256, Sha384, Sha512};

use crate::error::AuthError;

// ── Verificação de Send + Sync em tempo de compilação ─────────────────────────

fn _assert_oidc_service_send_sync()
where
    OidcService: Send + Sync,
{
}

// ── Tipos públicos ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    /// URL do discovery endpoint (`.well-known/openid-configuration`). Opcional
    /// quando `jwks_url` já estiver preenchido.
    pub metadata_url: Option<String>,
    /// URL directo do JWKS. Ignorado quando `metadata_url` estiver preenchido e
    /// devolver `jwks_uri`.
    pub jwks_url: Option<String>,
    pub allowed_clock_skew: Duration,
    pub metadata_ttl: Duration,
    pub jwks_ttl: Duration,
    pub allow_insecure_http: bool,
    /// Número de tentativas adicionais em caso de falha de fetch (0 = sem retry).
    pub fetch_retries: u32,
    /// Pausa entre tentativas em milissegundos (0 = sem pausa).
    pub fetch_retry_delay_ms: u64,
}

impl OidcConfig {
    pub fn validate(&self) -> Result<(), AuthError> {
        if self.issuer.trim().is_empty() {
            return Err(AuthError::ProviderUnsupported(
                "issuer OIDC obrigatório".into(),
            ));
        }
        if self.audience.trim().is_empty() {
            return Err(AuthError::ProviderUnsupported(
                "audience OIDC obrigatória".into(),
            ));
        }
        let has_metadata = self
            .metadata_url
            .as_deref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);
        let has_jwks = self
            .jwks_url
            .as_deref()
            .map(|u| !u.trim().is_empty())
            .unwrap_or(false);
        if !has_metadata && !has_jwks {
            return Err(AuthError::ProviderUnsupported(
                "metadata_url ou jwks_url OIDC obrigatório".into(),
            ));
        }
        if !self.allow_insecure_http {
            if let Some(url) = &self.metadata_url {
                validate_https(url, "metadata_url")?;
            }
            if let Some(url) = &self.jwks_url {
                validate_https(url, "jwks_url")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub issuer: String,
    pub jwks_uri: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    #[serde(rename = "kid", default)]
    pub key_id: String,
    #[serde(rename = "kty")]
    pub key_type: String,
    #[serde(rename = "use", default)]
    pub use_: String,
    #[serde(rename = "alg", default)]
    pub algorithm: String,
    // RSA
    #[serde(rename = "n", default)]
    pub modulus: String,
    #[serde(rename = "e", default)]
    pub exponent: String,
    // EC (P-256, P-384) e OKP (Ed25519)
    #[serde(rename = "crv", default)]
    pub curve: String,
    #[serde(rename = "x", default)]
    pub x: String,
    #[serde(rename = "y", default)]
    pub y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalClaims {
    pub subject: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub not_before: Option<DateTime<Utc>>,
    pub issued_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
    pub roles: Vec<String>,
    pub raw: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedPrincipal {
    pub subject: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub claims: TechnicalClaims,
}

// ── Port traits ───────────────────────────────────────────────────────────────

/// Port de busca remota de metadata e JWKS. Implementado por crates de infra.
pub trait OidcFetcher: Send + Sync {
    fn fetch_metadata(&self, url: &str) -> Result<ProviderMetadata, AuthError>;
    fn fetch_jwks(&self, url: &str) -> Result<Jwks, AuthError>;
}

/// Cache persistível opcional de metadata e JWKS. Implementado por crates de infra.
pub trait OidcCacheStore: Send + Sync {
    fn save_metadata(
        &self,
        issuer: &str,
        metadata: &ProviderMetadata,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), AuthError>;
    fn find_metadata(
        &self,
        issuer: &str,
    ) -> Result<Option<(ProviderMetadata, DateTime<Utc>)>, AuthError>;
    fn save_jwks(
        &self,
        issuer: &str,
        jwks: &Jwks,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), AuthError>;
    fn find_jwks(&self, issuer: &str) -> Result<Option<(Jwks, DateTime<Utc>)>, AuthError>;
}

// ── Chave pública resolvida (privado) ─────────────────────────────────────────

enum RawPublicKey {
    Rsa(RsaPublicKey),
    Ec256(p256::ecdsa::VerifyingKey),
    Ec384(p384::ecdsa::VerifyingKey),
    Ed25519(ed25519_dalek::VerifyingKey),
}

// ── OidcService ───────────────────────────────────────────────────────────────

struct ServiceCache {
    metadata: Option<(ProviderMetadata, Instant)>,
    jwks: Option<(Jwks, Instant)>,
}

/// Serviço técnico de validação OIDC/JWT.
///
/// Mantém cache em memória de metadata e JWKS; delega fetch remoto ao
/// [`OidcFetcher`] injetado. A autoridade institucional das claims fica fora
/// deste módulo.
pub struct OidcService {
    config: OidcConfig,
    fetcher: Box<dyn OidcFetcher>,
    cache_store: Option<Box<dyn OidcCacheStore>>,
    cache: RwLock<ServiceCache>,
}

impl OidcService {
    pub fn new(
        config: OidcConfig,
        fetcher: Box<dyn OidcFetcher>,
        cache_store: Option<Box<dyn OidcCacheStore>>,
    ) -> Result<Self, AuthError> {
        config.validate()?;
        Ok(Self {
            config,
            fetcher,
            cache_store,
            cache: RwLock::new(ServiceCache {
                metadata: None,
                jwks: None,
            }),
        })
    }

    pub fn resolve_metadata(&self) -> Result<ProviderMetadata, AuthError> {
        self.do_resolve_metadata(false)
    }

    pub fn resolve_jwks(&self) -> Result<Jwks, AuthError> {
        self.do_resolve_jwks(false)
    }

    pub fn map_claims(
        &self,
        raw: serde_json::Map<String, Value>,
    ) -> Result<TechnicalClaims, AuthError> {
        map_claims_from_map(raw)
    }

    pub fn validate_token(
        &self,
        token: &str,
        now: DateTime<Utc>,
    ) -> Result<ValidatedPrincipal, AuthError> {
        if token.trim().is_empty() {
            return Err(AuthError::TokenInvalid("token JWT ausente".into()));
        }

        let (header, raw_claims, signing_input, signature) = parse_jwt(token)?;

        let metadata = self.do_resolve_metadata(false)?;
        if !metadata.issuer.is_empty() && metadata.issuer != self.config.issuer {
            return Err(AuthError::MetadataUnavailable(format!(
                "issuer da metadata '{}' não coincide com configuração '{}'",
                metadata.issuer, self.config.issuer
            )));
        }

        // Obter chave pública — no máximo um refresh forçado de JWKS no total
        let jwks = self.do_resolve_jwks(false)?;
        let (pub_key, jwks_already_refreshed) = match public_key_from_jwks(&jwks, &header.key_id) {
            Ok(k) => (k, false),
            Err(_) => {
                let refreshed = self.do_resolve_jwks(true)?;
                (public_key_from_jwks(&refreshed, &header.key_id)?, true)
            }
        };

        // Verificar assinatura; se falhar e JWKS ainda não foi refrescado, tenta uma vez
        let sig_ok = verify_signature(
            &header.algorithm,
            &pub_key,
            signing_input.as_bytes(),
            &signature,
        );
        if let Err(e) = sig_ok {
            if jwks_already_refreshed {
                return Err(e);
            }
            let refreshed = self.do_resolve_jwks(true)?;
            let key = public_key_from_jwks(&refreshed, &header.key_id)
                .map_err(|_| AuthError::TokenInvalid("assinatura JWT inválida".into()))?;
            verify_signature(
                &header.algorithm,
                &key,
                signing_input.as_bytes(),
                &signature,
            )?;
        }

        let claims = map_claims_from_map(raw_claims)?;

        if claims.issuer != self.config.issuer {
            return Err(AuthError::ClaimsInvalid(format!(
                "issuer '{}' inválido",
                claims.issuer
            )));
        }
        if !claims.audience.iter().any(|a| a == &self.config.audience) {
            return Err(AuthError::ClaimsInvalid("audience inválida".into()));
        }

        let skew = chrono::Duration::from_std(self.config.allowed_clock_skew)
            .unwrap_or(chrono::Duration::zero());

        if now > claims.expires_at + skew {
            return Err(AuthError::TokenExpired);
        }
        if let Some(nbf) = claims.not_before {
            if now + skew < nbf {
                return Err(AuthError::ClaimsInvalid(
                    "token ainda não está activo".into(),
                ));
            }
        }
        if let Some(iat) = claims.issued_at {
            if iat > now + skew {
                return Err(AuthError::ClaimsInvalid("claim iat futura".into()));
            }
        }

        Ok(ValidatedPrincipal {
            subject: claims.subject.clone(),
            issuer: claims.issuer.clone(),
            audience: claims.audience.clone(),
            claims,
        })
    }

    // ── Internos ──────────────────────────────────────────────────────────────

    fn do_resolve_metadata(&self, force: bool) -> Result<ProviderMetadata, AuthError> {
        if !force {
            if let Ok(cache) = self.cache.read() {
                if let Some((ref m, fetched_at)) = cache.metadata {
                    if fetched_at.elapsed() < self.config.metadata_ttl {
                        return Ok(m.clone());
                    }
                }
            }
            if let Some(store) = &self.cache_store {
                if let Ok(Some((m, _))) = store.find_metadata(&self.config.issuer) {
                    self.write_metadata_cache(m.clone());
                    return Ok(m);
                }
            }
        }

        let url = match &self.config.metadata_url {
            Some(u) if !u.trim().is_empty() => u.clone(),
            _ => {
                let m = ProviderMetadata {
                    issuer: self.config.issuer.clone(),
                    jwks_uri: self.config.jwks_url.clone().unwrap_or_default(),
                    authorization_endpoint: None,
                    token_endpoint: None,
                };
                self.write_metadata_cache(m.clone());
                return Ok(m);
            }
        };

        let metadata = fetch_with_retry(
            || self.fetcher.fetch_metadata(&url),
            self.config.fetch_retries,
            self.config.fetch_retry_delay_ms,
        )?;
        if metadata.issuer.is_empty() || metadata.jwks_uri.is_empty() {
            return Err(AuthError::MetadataUnavailable(
                "metadata OIDC incompleta".into(),
            ));
        }
        self.write_metadata_cache(metadata.clone());
        if let Some(store) = &self.cache_store {
            let _ = store.save_metadata(&self.config.issuer, &metadata, Utc::now());
        }
        Ok(metadata)
    }

    fn do_resolve_jwks(&self, force: bool) -> Result<Jwks, AuthError> {
        if !force {
            if let Ok(cache) = self.cache.read() {
                if let Some((ref j, fetched_at)) = cache.jwks {
                    if fetched_at.elapsed() < self.config.jwks_ttl {
                        return Ok(j.clone());
                    }
                }
            }
            if let Some(store) = &self.cache_store {
                if let Ok(Some((j, _))) = store.find_jwks(&self.config.issuer) {
                    self.write_jwks_cache(j.clone());
                    return Ok(j);
                }
            }
        }

        let metadata = self.do_resolve_metadata(false)?;
        if metadata.jwks_uri.trim().is_empty() {
            return Err(AuthError::JwksUnavailable("JWKS URI ausente".into()));
        }

        let jwks = fetch_with_retry(
            || self.fetcher.fetch_jwks(&metadata.jwks_uri),
            self.config.fetch_retries,
            self.config.fetch_retry_delay_ms,
        )?;
        if jwks.keys.is_empty() {
            return Err(AuthError::JwksUnavailable(
                "JWKS sem chaves disponíveis".into(),
            ));
        }
        self.write_jwks_cache(jwks.clone());
        if let Some(store) = &self.cache_store {
            let _ = store.save_jwks(&self.config.issuer, &jwks, Utc::now());
        }
        Ok(jwks)
    }

    fn write_metadata_cache(&self, m: ProviderMetadata) {
        if let Ok(mut cache) = self.cache.write() {
            cache.metadata = Some((m, Instant::now()));
        }
    }

    fn write_jwks_cache(&self, j: Jwks) {
        if let Ok(mut cache) = self.cache.write() {
            cache.jwks = Some((j, Instant::now()));
        }
    }
}

// ── Funções auxiliares ────────────────────────────────────────────────────────

/// Corre `f` até `retries + 1` vezes com pausa linear entre tentativas.
/// Sem `unwrap`: a última iteração devolve o resultado directamente.
fn fetch_with_retry<T, F>(mut f: F, retries: u32, delay_ms: u64) -> Result<T, AuthError>
where
    F: FnMut() -> Result<T, AuthError>,
{
    for _ in 0..retries {
        if let Ok(v) = f() {
            return Ok(v);
        }
        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
    }
    f()
}

struct JwtHeader {
    algorithm: String,
    key_id: String,
}

type ParsedJwt = (JwtHeader, serde_json::Map<String, Value>, String, Vec<u8>);

/// Devolve (header, claims_map, signing_input, signature_bytes).
fn parse_jwt(token: &str) -> Result<ParsedJwt, AuthError> {
    let parts: Vec<&str> = token.splitn(4, '.').collect();
    if parts.len() != 3 {
        return Err(AuthError::TokenInvalid("formato JWT inválido".into()));
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| AuthError::TokenInvalid("header JWT inválido".into()))?;
    let header_value: Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| AuthError::TokenInvalid("header JWT inválido".into()))?;
    let algorithm = header_value["alg"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::TokenInvalid("algoritmo JWT ausente".into()))?
        .to_owned();
    let key_id = header_value["kid"].as_str().unwrap_or("").to_owned();

    let claims_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AuthError::TokenInvalid("claims JWT inválidas".into()))?;
    let claims_map: serde_json::Map<String, Value> = serde_json::from_slice(&claims_bytes)
        .map_err(|_| AuthError::TokenInvalid("claims JWT inválidas".into()))?;

    let signature = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AuthError::TokenInvalid("assinatura JWT inválida".into()))?;

    let signing_input = format!("{}.{}", parts[0], parts[1]);

    Ok((
        JwtHeader { algorithm, key_id },
        claims_map,
        signing_input,
        signature,
    ))
}

fn map_claims_from_map(raw: serde_json::Map<String, Value>) -> Result<TechnicalClaims, AuthError> {
    let issuer = raw["iss"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::ClaimsInvalid("claim iss ausente".into()))?
        .to_owned();
    let subject = raw["sub"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::ClaimsInvalid("claim sub ausente".into()))?
        .to_owned();
    let audience = audience_from_value(raw.get("aud"))
        .ok_or_else(|| AuthError::ClaimsInvalid("claim aud inválida".into()))?;
    let expires_at = unix_claim(raw.get("exp"))
        .ok_or_else(|| AuthError::ClaimsInvalid("claim exp inválida".into()))?;
    let not_before = optional_unix_claim(raw.get("nbf"))
        .ok_or_else(|| AuthError::ClaimsInvalid("claim nbf inválida".into()))?;
    let issued_at = optional_unix_claim(raw.get("iat"))
        .ok_or_else(|| AuthError::ClaimsInvalid("claim iat inválida".into()))?;

    let scopes = scopes_from_value(raw.get("scope"));
    let roles = roles_from_map(&raw);

    Ok(TechnicalClaims {
        subject,
        issuer,
        audience,
        expires_at,
        not_before,
        issued_at,
        scopes,
        roles,
        raw,
    })
}

fn audience_from_value(value: Option<&Value>) -> Option<Vec<String>> {
    match value? {
        Value::String(s) if !s.is_empty() => Some(vec![s.clone()]),
        Value::Array(arr) => {
            let items: Option<Vec<String>> = arr
                .iter()
                .map(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_owned()))
                .collect();
            items.filter(|v| !v.is_empty())
        }
        _ => None,
    }
}

fn unix_claim(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let secs = value?.as_f64()? as i64;
    DateTime::from_timestamp(secs, 0)
}

fn optional_unix_claim(value: Option<&Value>) -> Option<Option<DateTime<Utc>>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(v) => Some(unix_claim(Some(v))),
    }
}

fn scopes_from_value(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(s)) => s
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_owned()))
            .collect(),
        _ => vec![],
    }
}

fn roles_from_map(raw: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut roles = Vec::new();
    if let Some(Value::Array(arr)) = raw.get("roles") {
        for v in arr {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                roles.push(s.to_owned());
            }
        }
    }
    if let Some(Value::Object(realm)) = raw.get("realm_access") {
        if let Some(Value::Array(arr)) = realm.get("roles") {
            for v in arr {
                if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                    if !roles.contains(&s.to_owned()) {
                        roles.push(s.to_owned());
                    }
                }
            }
        }
    }
    roles
}

fn public_key_from_jwks(jwks: &Jwks, key_id: &str) -> Result<RawPublicKey, AuthError> {
    let candidates: Vec<&Jwk> = if key_id.is_empty() {
        jwks.keys.iter().collect()
    } else {
        jwks.keys.iter().filter(|k| k.key_id == key_id).collect()
    };

    for jwk in &candidates {
        if !jwk.use_.is_empty() && jwk.use_ != "sig" {
            continue;
        }
        match jwk.key_type.as_str() {
            "RSA" => {
                if let Ok(key) = rsa_public_key_from_jwk(jwk) {
                    return Ok(RawPublicKey::Rsa(key));
                }
            }
            "EC" => match jwk.curve.as_str() {
                "P-256" => {
                    if let Ok(key) = ec256_verifying_key_from_jwk(jwk) {
                        return Ok(RawPublicKey::Ec256(key));
                    }
                }
                "P-384" => {
                    if let Ok(key) = ec384_verifying_key_from_jwk(jwk) {
                        return Ok(RawPublicKey::Ec384(key));
                    }
                }
                _ => {}
            },
            "OKP" => {
                if jwk.curve == "Ed25519" {
                    if let Ok(key) = ed25519_verifying_key_from_jwk(jwk) {
                        return Ok(RawPublicKey::Ed25519(key));
                    }
                }
            }
            _ => {}
        }
    }

    Err(AuthError::JwksUnavailable(format!(
        "chave pública não encontrada no JWKS (kid='{}')",
        key_id
    )))
}

fn rsa_public_key_from_jwk(jwk: &Jwk) -> Result<RsaPublicKey, AuthError> {
    let n_bytes = URL_SAFE_NO_PAD
        .decode(&jwk.modulus)
        .map_err(|_| AuthError::JwksUnavailable("módulo RSA base64 inválido".into()))?;
    let e_bytes = URL_SAFE_NO_PAD
        .decode(&jwk.exponent)
        .map_err(|_| AuthError::JwksUnavailable("expoente RSA base64 inválido".into()))?;

    let n = BigUint::from_bytes_be(&n_bytes);
    let e = BigUint::from_bytes_be(&e_bytes);

    RsaPublicKey::new(n, e)
        .map_err(|_| AuthError::JwksUnavailable("chave RSA pública inválida".into()))
}

fn ec_uncompressed_point(jwk: &Jwk) -> Result<Vec<u8>, AuthError> {
    let x = URL_SAFE_NO_PAD
        .decode(&jwk.x)
        .map_err(|_| AuthError::JwksUnavailable("coordenada EC x inválida".into()))?;
    let y = URL_SAFE_NO_PAD
        .decode(&jwk.y)
        .map_err(|_| AuthError::JwksUnavailable("coordenada EC y inválida".into()))?;
    let mut point = Vec::with_capacity(1 + x.len() + y.len());
    point.push(0x04); // uncompressed
    point.extend_from_slice(&x);
    point.extend_from_slice(&y);
    Ok(point)
}

fn ec256_verifying_key_from_jwk(jwk: &Jwk) -> Result<p256::ecdsa::VerifyingKey, AuthError> {
    let point = ec_uncompressed_point(jwk)?;
    p256::ecdsa::VerifyingKey::from_sec1_bytes(&point)
        .map_err(|_| AuthError::JwksUnavailable("chave EC P-256 inválida".into()))
}

fn ec384_verifying_key_from_jwk(jwk: &Jwk) -> Result<p384::ecdsa::VerifyingKey, AuthError> {
    let point = ec_uncompressed_point(jwk)?;
    p384::ecdsa::VerifyingKey::from_sec1_bytes(&point)
        .map_err(|_| AuthError::JwksUnavailable("chave EC P-384 inválida".into()))
}

fn ed25519_verifying_key_from_jwk(jwk: &Jwk) -> Result<ed25519_dalek::VerifyingKey, AuthError> {
    let x = URL_SAFE_NO_PAD
        .decode(&jwk.x)
        .map_err(|_| AuthError::JwksUnavailable("chave Ed25519 x inválida".into()))?;
    let bytes: [u8; 32] = x
        .try_into()
        .map_err(|_| AuthError::JwksUnavailable("chave Ed25519 deve ter 32 bytes".into()))?;
    ed25519_dalek::VerifyingKey::from_bytes(&bytes)
        .map_err(|_| AuthError::JwksUnavailable("chave Ed25519 inválida".into()))
}

fn verify_signature(
    algorithm: &str,
    key: &RawPublicKey,
    signing_input: &[u8],
    signature: &[u8],
) -> Result<(), AuthError> {
    match (algorithm, key) {
        ("RS256", RawPublicKey::Rsa(rsa_key)) => {
            let vk = VerifyingKey::<Sha256>::new(rsa_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS256 inválida".into()))?;
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS256 não verificada".into()))
        }
        ("RS384", RawPublicKey::Rsa(rsa_key)) => {
            let vk = VerifyingKey::<Sha384>::new(rsa_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS384 inválida".into()))?;
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS384 não verificada".into()))
        }
        ("RS512", RawPublicKey::Rsa(rsa_key)) => {
            let vk = VerifyingKey::<Sha512>::new(rsa_key.clone());
            let sig = rsa::pkcs1v15::Signature::try_from(signature)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS512 inválida".into()))?;
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura RS512 não verificada".into()))
        }
        ("ES256", RawPublicKey::Ec256(vk)) => {
            use p256::ecdsa::signature::Verifier as _;
            let sig = p256::ecdsa::Signature::from_slice(signature)
                .map_err(|_| AuthError::TokenInvalid("assinatura ES256 inválida".into()))?;
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura ES256 não verificada".into()))
        }
        ("ES384", RawPublicKey::Ec384(vk)) => {
            use p384::ecdsa::signature::Verifier as _;
            let sig = p384::ecdsa::Signature::from_slice(signature)
                .map_err(|_| AuthError::TokenInvalid("assinatura ES384 inválida".into()))?;
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura ES384 não verificada".into()))
        }
        ("EdDSA", RawPublicKey::Ed25519(vk)) => {
            use ed25519_dalek::Verifier as _;
            let bytes: [u8; 64] = signature.try_into().map_err(|_| {
                AuthError::TokenInvalid("assinatura EdDSA inválida (64 bytes esperados)".into())
            })?;
            let sig = ed25519_dalek::Signature::from_bytes(&bytes);
            vk.verify(signing_input, &sig)
                .map_err(|_| AuthError::TokenInvalid("assinatura EdDSA não verificada".into()))
        }
        (alg, _) => Err(AuthError::ProviderUnsupported(format!(
            "algoritmo JWT não suportado: {}",
            alg
        ))),
    }
}

fn validate_https(url: &str, field: &str) -> Result<(), AuthError> {
    let trimmed = url.trim().to_lowercase();
    if !trimmed.is_empty() && !trimmed.starts_with("https://") {
        return Err(AuthError::ProviderUnsupported(format!(
            "{} deve usar HTTPS",
            field
        )));
    }
    Ok(())
}

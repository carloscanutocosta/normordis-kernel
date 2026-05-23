# MAN — support-auth

## Superfície pública

### OidcService

```rust
OidcService::new(config, fetcher, cache_store?) -> Result<Self, AuthError>
OidcService::resolve_metadata() -> Result<ProviderMetadata, AuthError>
OidcService::resolve_jwks() -> Result<Jwks, AuthError>
OidcService::map_claims(raw) -> Result<TechnicalClaims, AuthError>
OidcService::validate_token(token, now) -> Result<ValidatedPrincipal, AuthError>
```

**Port traits:**
- `OidcFetcher` — `fetch_metadata(url)` + `fetch_jwks(url)` (implementar em infra)
- `OidcCacheStore` — persistência opcional de metadata e JWKS

**Algoritmos suportados:** RS256, RS384, RS512

### OtcService

```rust
OtcService::new(config) -> Result<Self, AuthError>
OtcService::issue(subject_ref, now) -> Result<(IssuedOtc, OtcState), AuthError>
OtcService::verify(state, code, subject_ref, now) -> Result<OtcVerificationResult, AuthError>

issue_flow(service, store, subject_ref, now) -> Result<IssuedOtc, AuthError>
verify_flow(service, store, reference, code, subject_ref, now) -> Result<OtcVerificationResult, AuthError>
```

**Port trait:**
- `OtcStateStore` — `save_state`, `find_state`, `delete_state`

## Invariantes

- `support-auth` trata apenas autenticação protocolar técnica.
- O módulo não decide significado institucional de claims.
- `issuer`, `audience` e validade temporal são validados antes de aceitar token.
- O provider concreto (Keycloak, Entra ID, etc.) é injetável via `OidcFetcher`.
- O código OTC nunca aparece em claro no `OtcState` — apenas hash+salt.
- O estado OTC é efémero; `should_delete=true` no resultado sinaliza limpeza.

## Dependências

- `support-errors` — `MiniError`, `ErrorCode`, `Component`
- `rsa` / `sha2` — verificação de assinaturas JWT RSA
- `base64`, `serde_json` — parsing JWT
- `rand`, `sha2`, `hex` — geração e hash de códigos OTC

## Versionamento

- MINOR para adições compatíveis.
- MAJOR para alterações incompatíveis (requer decisão explícita).

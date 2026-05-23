# support-auth

Tipo: support  
Versão: v0.1.0  
Estado: Experimental  
Owner: Engineering

---

## Objetivo

O crate `support-auth` fornece mecanismos técnicos de autenticação protocolar
para o Mini-Kernel RS.

Trata apenas de protocolos e mecanismos técnicos, sem decidir autoridade
institucional. A relação pessoa-instituição-função pertence a `core-rh`,
`core-org` e (futuramente) `core-security`.

---

## Protocolos — v1

- **OIDC / JWT** — validação de tokens JWT com JWKS remoto via port trait
- **OTC** — emissão e verificação de códigos one-time de entrega variável

## Protocolos — v2 (stubs de tipos)

- LDAP — plan-based (sem cliente de rede)
- SAML — plan-based (sem parser XML)
- WebAuthn / Passkeys — challenge + verify (sem crypto WebAuthn)

---

## Responsabilidades

- validação técnica de tokens JWT (RS256, RS384, RS512)
- resolução de metadata OIDC e JWKS com cache em memória
- cache persistível opcional via `OidcCacheStore`
- emissão e verificação de códigos OTC com hash+salt (SHA-256)
- persistência opcional de estado OTC via `OtcStateStore`
- tipos técnicos para LDAP, SAML e WebAuthn (stubs para v2)

## Não-responsabilidades

- autoridade institucional de claims
- clientes HTTP (port trait `OidcFetcher` — implementado em infra)
- clientes LDAP, parsers XML SAML, crypto WebAuthn
- decisão de políticas de acesso

---

## Códigos de erro

| Código | Uso |
|---|---|
| `MINI.AUTH.TOKEN_INVALID` | Token JWT inválido ou assinatura não verificável |
| `MINI.AUTH.TOKEN_EXPIRED` | Token JWT expirado |
| `MINI.AUTH.CLAIMS_INVALID` | Claims inválidas (issuer, audience, nbf, iat) |
| `MINI.AUTH.METADATA_UNAVAILABLE` | Metadata OIDC indisponível |
| `MINI.AUTH.JWKS_UNAVAILABLE` | JWKS indisponível ou sem chave adequada |
| `MINI.AUTH.PROVIDER_UNSUPPORTED` | Configuração de provider inválida |
| `MINI.AUTH.REQUEST_INVALID` | Pedido de autenticação inválido |
| `MINI.AUTH.OTC_INVALID` | Código OTC inválido |
| `MINI.AUTH.OTC_EXPIRED` | Código OTC expirado |
| `MINI.AUTH.OTC_ATTEMPTS_EXCEEDED` | Limite de tentativas OTC excedido |
| `MINI.AUTH.STATE_UNAVAILABLE` | Estado OTC não encontrado |

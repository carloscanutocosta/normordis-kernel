# email-infra

Adaptadores infra para verificação DNS/MX e envio de email.

## Responsabilidade

- Implementar `core_validation::EmailVerificationPort`.
- Implementar `core_validation::EmailDeliveryPort` para Microsoft Graph.
- Validar rota de entrega por DNS: MX explícito ou fallback A/AAAA.
- Enviar mensagens via Microsoft Graph `sendMail` com bearer token fornecido pelo host.
- Usar servidor DNS configurado pelo host.

## Não responsabilidade

- Validar sintaxe de email; isso pertence a `support-normalization`/`core-validation`.
- Fazer SMTP probing, scoring antispam ou validação de caixa postal.
- Obter tokens OAuth/Microsoft; o host fornece o bearer token.
- Persistir resultados.

## Exemplo mínimo

```rust
use email_infra::{DnsEmailVerifier, GraphEmailSender};
use std::net::SocketAddr;

let dns: SocketAddr = "1.1.1.1:53".parse().unwrap();
let verifier = DnsEmailVerifier::new(dns);

let sender = GraphEmailSender::new("me", "ACCESS_TOKEN");
```

## OWA / Microsoft 365

OWA é uma interface humana. Para envio automático enterprise, usar Microsoft Graph
`sendMail` ou SMTP AUTH quando permitido pelo tenant. O adapter actualmente
implementado é `GraphEmailSender`.

Exemplo para a mailbox institucional `sf0248`:

```rust
use core_validation::{EmailDeliveryPort, EmailMessage};
use email_infra::GraphEmailSender;

let sender = GraphEmailSender::new("sf0248@example.pt", access_token);
let message = EmailMessage::text(
    ["destinatario@example.pt"],
    "Notificação NORMORDIS",
    "Texto da mensagem.",
);

let evidence = sender.send_email(&message)?;
```

O token OAuth é obtido pelo host; este crate não guarda credenciais nem automatiza
a UI do OWA.

Ver [MAN.md](MAN.md) para contrato e limites.

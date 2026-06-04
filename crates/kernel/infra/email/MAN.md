# Manual: email-infra

## Propósito e fronteira

`email-infra` é o adaptador de infraestrutura para verificação DNS/MX e envio
de email.

Mantém a arquitectura hexagonal:

- `support-normalization` valida forma estrutural e normaliza domínio IDNA.
- `core-validation` define o porto `EmailVerificationPort` e os tipos de evidência.
- `email-infra` executa consultas DNS concretas e chamadas a providers de envio.

## Contrato

`DnsEmailVerifier` implementa:

```rust
core_validation::EmailVerificationPort
```

Resultado:

- `EmailRouteStatus::MxFound`: domínio tem MX.
- `EmailRouteStatus::AddressFallbackFound`: domínio não tem MX mas tem A/AAAA,
  permitindo fallback de entrega conforme prática SMTP.
- `EmailRouteStatus::NoRoute`: não foi encontrada rota de entrega.

## Configuração

O host fornece o servidor DNS:

```rust
use email_infra::DnsEmailVerifier;

let verifier = DnsEmailVerifier::new("1.1.1.1:53".parse().unwrap());
```

Isto evita dependência em configuração de sistema operativo dentro do kernel.

## Envio Microsoft Graph

`GraphEmailSender` implementa:

```rust
core_validation::EmailDeliveryPort
```

O adapter chama Microsoft Graph `sendMail`. O host fornece o bearer token; este
crate não gere OAuth, refresh tokens, secrets nem consentimento institucional.

```rust
use core_validation::{EmailDeliveryPort, EmailMessage};
use email_infra::GraphEmailSender;

let sender = GraphEmailSender::new("me", "ACCESS_TOKEN");
let message = EmailMessage::text(["destino@example.pt"], "Assunto", "Corpo");
let evidence = sender.send_email(&message)?;
```

Para contas institucionais específicas, usar o identificador/mailbox em vez de
`"me"`, por exemplo `GraphEmailSender::new("notificacoes@example.pt", token)`.

### Caso OWA / Microsoft 365

OWA é a interface web humana do Exchange Online. Não deve ser automatizada por
scraping, browser automation ou replay de formulários: esse caminho é frágil,
difícil de auditar, incompatível com MFA/Conditional Access e arriscado do ponto
de vista de segurança.

Quando uma conta é acessível via OWA, o caminho enterprise para envio automático é
um destes:

1. **Microsoft Graph `sendMail` delegado** — um utilizador autenticado autoriza a
   app a enviar como ele. Adequado quando a app corre em nome de um utilizador.
2. **Microsoft Graph `sendMail` app-only** — uma app registada no tenant recebe
   permissão `Mail.Send` e é limitada, por política Exchange/Application Access
   Policy, às mailboxes autorizadas. Adequado para contas de serviço.
3. **SMTP AUTH com TLS** — apenas se estiver explicitamente permitido pela política
   do tenant. Requer adapter SMTP dedicado.

### Exemplo: mailbox `sf0248`

Se `sf0248` for o identificador/mailbox institucional, o host deve resolver o seu
UPN/endereço completo, por exemplo `sf0248@example.pt`, obter um bearer token com
permissão `Mail.Send`, e injectar o adapter:

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

Se o token for **delegado** e pertencer ao próprio `sf0248`, também pode ser usado
`GraphEmailSender::new("me", access_token)`. Se o token for **app-only**, usar a
mailbox explícita (`"sf0248@example.pt"`) e restringir a aplicação às mailboxes
autorizadas no Exchange.

O `EmailDeliveryEvidence` devolve o provider, destinatários aceites e, quando o
provider disponibiliza, um identificador externo de pedido/mensagem. O chamador é
responsável por registar esta evidência em `core-audit` quando o envio tiver valor
institucional.

## Limitações

- Não faz SMTP probing nem confirma existência de mailbox.
- Não obtém tokens OAuth nem guarda credenciais.
- Não automatiza OWA/UI web.
- Não implementa SMTP AUTH; Graph é o adapter de envio actualmente disponível.
- Não segue TCP fallback em respostas DNS truncadas.
- Não persiste cache nem evidência.
- Não substitui políticas institucionais de validação de contacto.

## Estado

Production-ready interno/controlado para:

- verificação DNS/MX configurável;
- envio via Microsoft Graph quando o host fornece token válido e política tenant
  adequada.

Requisitos operacionais para produção:

- token OAuth obtido fora deste crate, por fluxo aprovado pela organização;
- permissões Graph mínimas (`Mail.Send`) e escopo restrito;
- MFA/Conditional Access respeitado no fluxo de autenticação;
- secrets fora do código e fora de logs;
- evidência de envio registada pelo serviço chamador quando aplicável;
- política de retenção e classificação definida pela app/domínio.

Para validação “100%” operacional de email, o pipeline deve combinar:

1. validação estrutural/IDNA;
2. DNS/MX;
3. política institucional opcional;
4. confirmação out-of-band, por exemplo email com token.

Nenhuma biblioteca headless consegue provar existência e disponibilidade futura
de uma mailbox apenas por formato ou DNS.

## Expansão SMTP

Também é possível acrescentar envio SMTP mantendo a mesma fronteira:

- o core define um porto de entrega (`EmailDeliveryPort`) e modelos mínimos da
  mensagem;
- `email-infra` fornece adapters concretos, por exemplo SMTP com TLS/auth;
- o host injeta o adapter no serviço que precisa de enviar email.

SMTP deve ser acrescentado com dependência SMTP/TLS adequada, em vez de uma
implementação manual do protocolo.

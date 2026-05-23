# Manual do modulo infra-signing

## Objetivo

`infra-signing` adapta para Rust os bindings de assinatura digital da implementação
Normordis em `kernel/infra/services/signing`. O crate fica em `crates/kernel/infra`
porque materializa capacidades técnicas concretas e planos de integração para
providers de assinatura.

## Contrato publico

Tipos principais:

```rust
Provider
SignatureFormat
Config
RuntimeBinding
Operation

QualifiedCertificateConfig
QualifiedCertificateAdapter
QualifiedPlan

OtcConfig
OtcDelivery
OtcAdapter
OtcPlan
OtcIssuer
OtcFlowService
IssuedOtc
IssuedOtcRecord
OtcAttempt
OtcVerificationResult
OtcCodeGenerator
OtcIssueRequest
OtcIssueResponse
OtcVerifyRequest
OtcVerifyResponse
OtcRecordStore
MemoryOtcRecordStore
OtcDeliveryGateway
MockOtcDeliveryGateway
Pkcs11Mechanism
Pkcs11SigningConfig
Pkcs11SigningAdapter
Pkcs11SigningPlan
PinResolver
NativePkcs11Signer
Pkcs11ModuleProbe
Pkcs11SlotProbe
Pkcs11TokenProbe
Pkcs11ObjectProbe
probe_pkcs11_module
ChaveMovelDigitalConfig
ChaveMovelDigitalUserConfirmation
ChaveMovelDigitalAdapter
ChaveMovelDigitalPlan
ChaveMovelDigitalService
ChaveMovelDigitalSession
ChaveMovelDigitalArtifact
ChaveMovelDigitalGateway
ChaveMovelDigitalGatewayStatus
ChaveMovelDigitalGatewayStartRequest
ChaveMovelDigitalGatewayStartResponse
ChaveMovelDigitalGatewayStatusResponse
MockChaveMovelDigitalGateway
HttpChaveMovelDigitalGateway
HttpChaveMovelDigitalGatewayConfig
ChaveMovelDigitalHttpTransport

CartaoCidadaoPtConfig
CartaoCidadaoPtMode
CartaoCidadaoPtAdapter
CartaoCidadaoPtPlan
CitizenCardAuthAdapter
CitizenCardAuthConfig
CitizenCardAuthMode
CitizenCardMiddleware
CitizenCardOfficialMiddlewareDefaults

DetachedSignatureRequest
DetachedSignature
DetachedSigningService
ExternalSigner
SigningEvidence
CommandSignerConfig
CommandExternalSigner

MiddlewareConfig
MiddlewareKind
AutenticacaoGovConfig
AutenticacaoGovFlow
TsaConfig
HsmConfig
CegerCardConfig
RealProviderAdapter
ProviderPlan

Bit4IdMiddlewareDefaults
CitizenCardPkcs11Defaults
CitizenCardPkcs11Toolchain
EcceCegerToolchain
Pkcs11HsmToolchain
AutenticacaoGovToolchain
ChaveMovelDigitalToolchain
CommandSignerToolchain
bit4id_pkcs11_module_path
citizen_card_pkcs11_module_candidates

SigningError
```

## Providers

### Certificado qualificado

Valida `profile`, `certificate_ref` e `trust_service_ref`. O adapter gera um plano
com operações para carregar certificado, resolver serviço de confiança e produzir
o artefacto de assinatura. Quando configurado, inclui dispositivo qualificado e
timestamp qualificado.

### OTC

Valida `profile`, canal de entrega, TTL, máximo de tentativas e tamanho do código.
`OtcIssuer` emite códigos numéricos e verifica tentativas contra expiração,
limite de tentativas, vínculo ao sujeito autenticado e valor do código. O retorno
separa o código entregue (`IssuedOtc.code`) do registo persistível
(`IssuedOtc.record`), que guarda hash SHA-256 com sal e não precisa persistir o
código em claro. A comparação de código usa `subtle::ConstantTimeEq`.

`OtcFlowService` fecha o fluxo operacional: emite código, grava
`IssuedOtcRecord` num `OtcRecordStore`, chama um `OtcDeliveryGateway`, verifica
tentativas posteriores, incrementa `attempt_count`, elimina o registo quando o
código é aceite e rejeita referências desconhecidas, expiradas, consumidas,
esgotadas ou com sujeito/código inválido. `MemoryOtcRecordStore` e
`MockOtcDeliveryGateway` existem para desenvolvimento/testes; produção deve
substituir o store por persistência durável e o gateway por SMS/email/app real.

### Cartão de Cidadão PT

Valida certificado, referência de PIN, modo (`middleware` ou `autenticacao-gov`),
serviço de confiança e leitor quando o modo é `middleware`. O PIN é apenas uma
referência (`scheme:ref`); o crate nunca guarda nem revela o segredo.

### Autenticação com Cartão de Cidadão

`CitizenCardAuthConfig` cobre autenticação forte com o Cartão de Cidadão por três
modelos técnicos:

- `MutualTls`: o servidor exige certificado de cliente e o browser/sistema
  seleciona o certificado de autenticação do Cartão de Cidadão.
- `SignedChallenge`: a app gera um desafio, o middleware assina com o certificado
  de autenticação e o servidor valida a assinatura e a cadeia.
- `CertificateDiscovery`: a app usa o middleware apenas para descobrir e validar
  o certificado de autenticação presente.

O adapter suporta `OfficialSdkCpp`, `OfficialSdkJava`, `Pkcs11` e
`TlsClientCertificate`. A documentação oficial do middleware Autenticação.gov
publica SDK C++ e Java, com inicialização via `PTEID_InitSDK`/`PTEID_ReleaseSDK`
em C++ e `PTEID_ReaderSet.initSDK()`/`releaseSDK()` em Java. A mesma documentação
mostra APIs oficiais para PAdES (`SignPDF`) e XAdES (`SignXades`,
`SignXadesT`, `SignXadesA`).

### Middleware local / PKCS#11

`MiddlewareConfig` suporta `Pkcs11`, `WindowsCapi` e `MacosKeychain`. Em PKCS#11,
`module_path` é obrigatório. O plano inclui carregar o middleware local, resolver
certificado e produzir assinatura destacada.

`Pkcs11SigningConfig` é o contrato operacional para assinatura direta via
PKCS#11: módulo, slot/token, chave privada por label ou id, certificado,
mecanismo (`CKM_RSA_PKCS`, `CKM_SHA256_RSA_PKCS`, `CKM_ECDSA`, etc.) e `pin_ref`.
O PIN nunca é modelado em claro; o host deve resolver `pin_ref` por `infra-secrets`
ou prompt seguro no momento de login.

Com a feature `native-pkcs11`, `NativePkcs11Signer` usa o crate `cryptoki` para
executar o fluxo real: carregar módulo, inicializar Cryptoki, selecionar slot,
abrir sessão, resolver PIN via `PinResolver`, fazer login, encontrar chave
privada, assinar e finalizar a sessão. Esta feature não é default para manter
builds sem middleware/cartão independentes de PKCS#11.

`probe_pkcs11_module` faz diagnóstico operacional sem PIN: carrega o módulo,
enumera slots, identifica leitor/token e lista objetos públicos visíveis,
incluindo certificados e chaves privadas com `label`, `id_hex`, `sign` e
`private` quando o middleware expõe esses atributos. No Cartão de Cidadão testado
com middleware Autenticação.gov em Windows, o módulo
`C:\Windows\System32\pteidpkcs11.dll` expôs:

- token `CARTAO DE CIDADAO` em `slot:0`;
- certificado de assinatura `CITIZEN SIGNATURE CERTIFICATE`, `id_hex = 46000000`;
- chave de assinatura `CITIZEN SIGNATURE KEY`, `id_hex = 46000000`.

`CitizenCardPkcs11Toolchain` fornece candidatos de módulo para Cartão de Cidadão:

- Windows: `C:\Windows\System32\pteidpkcs11.dll`,
  `C:\Windows\System32\libpteidpkcs11.dll`
- Linux: `/usr/local/lib/libpteidpkcs11.so`, `/usr/lib/libpteidpkcs11.so`,
  `/usr/lib/x86_64-linux-gnu/libpteidpkcs11.so`
- macOS: `/usr/local/lib/libpteidpkcs11.dylib`,
  `/Library/Frameworks/pteidpkcs11.framework/pteidpkcs11`

Estes caminhos são candidatos, não garantias. Instalações reais podem variar
conforme versão do middleware Autenticação.gov e distribuição do sistema.

### Autenticação.gov / SAFE / CMD

`AutenticacaoGovConfig` modela fluxos `CitizenCard`, `ChaveMovelDigital` e `Safe`.
O modo `Safe` exige `callback_url`. O plano cobre iniciar o fluxo, submeter hash,
aguardar confirmação do utilizador, recuperar artefacto de assinatura e, quando
pedido, solicitar atributos profissionais SCAP.

`ChaveMovelDigitalConfig` especializa o fluxo CMD para assinatura remota:
`service_endpoint`, `client_id`, `callback_url`, formato, timestamp, atributos
profissionais e modo de confirmação (`SmsOtp`, `MobileApp` ou
`ProviderDefault`). `ChaveMovelDigitalService::prepare_session` valida o pedido
e devolve uma sessão com `session_id`, hash SHA-256 dos bytes canónicos e
metadados necessários para o gateway autorizado. Depois de o gateway CMD
devolver o artefacto, `materialize_signature` valida a sessão e cria
`DetachedSignature`; `evidence_for_signature` produz `SigningEvidence`.

Este crate não inventa endpoints CMD nem transportes OAuth/SAML/SOAP sem
credenciais/contrato. O boundary de produção esperado é um gateway/CLI/backend
autorizado que recebe `signing_hash_hex`, executa o fluxo remoto da
Autenticação.gov/CMD e devolve o artefacto DER/PAdES/CAdES/XAdES aplicável.

`ChaveMovelDigitalGateway` é o port para esse boundary. `start_signature` recebe
a sessão preparada e devolve `gateway_request_id`, estado, URL opcional de
confirmação e expiração; `poll_signature` consulta o estado e, quando concluído,
devolve `ChaveMovelDigitalArtifact`.

`MockChaveMovelDigitalGateway` suporta desenvolvimento end-to-end sem serviço
externo, podendo ficar em `WaitingUserConfirmation` ou completar imediatamente
com um artefacto determinístico. `HttpChaveMovelDigitalGateway` é o adapter para
produção, mas recebe um `ChaveMovelDigitalHttpTransport` injetado pelo host; isto
permite usar `reqwest`, Tauri HTTP, proxy institucional ou outro cliente sem
prender o crate de infra a um runtime assíncrono ou SDK específico.

### TSA

`TsaConfig` modela um serviço RFC 3161 de selo temporal: endpoint, policy OID,
credenciais opcionais e nonce obrigatório/opcional. O adapter produz plano para
hash do artefacto, pedido de timestamp e verificação do token.

### HSM

`HsmConfig` modela HSM via PKCS#11: módulo, token, chave, PIN por referência,
certificado e TSA opcional. O plano cobre sessão no token, resolução de chave e
assinatura destacada.

### ECCE / CEGER

`CegerCardConfig` representa o cartão/certificado ECCE/CEGER com middleware
Bit4ID disponibilizado pela ECCE. A página
`https://www.ecce.gov.pt/suporte/middleware/software` identifica "Middleware -
Bit4ID", download dos ficheiros de instalação e manuais para Windows, macOS,
Linux e drivers. A referência de suporte geral continua em
`https://www.ecce.gov.pt/suporte/middleware/`. O plano cobre carregar middleware
Bit4ID, verificar leitor, carregar certificado qualificado, pedir PIN e produzir
artefacto.

`EcceCegerToolchain` pluga defaults concretos do Bit4ID por ambiente:

- Windows: `C:\Windows\System32\bit4xpki.dll`
- Linux: `/usr/lib/bit4id/libbit4xpki.so`
- macOS: `/System/Library/Bit4id/pkcs11/libbit4ipki.dylib`

Estes caminhos são defaults do middleware Bit4ID e podem ser substituídos pelo
host através de `MiddlewareConfig.module_path` quando a instalação local divergir.

## Toolchains concretas

- `CommandSignerToolchain` constrói `CommandExternalSigner` para qualquer CLI
  fornecedor/ambiente que aceite bytes por stdin e devolva DER por stdout.
- `Pkcs11HsmToolchain` constrói `HsmConfig` para HSM via PKCS#11.
- `CitizenCardPkcs11Toolchain` constrói `Pkcs11SigningConfig` para assinatura
  direta com Cartão de Cidadão via PKCS#11.
- `ChaveMovelDigitalToolchain` constrói `ChaveMovelDigitalConfig` e o
  `AutenticacaoGovConfig` equivalente para CMD.
- `AutenticacaoGovToolchain` constrói `AutenticacaoGovConfig` para fluxos
  Citizen Card, Chave Móvel Digital ou SAFE.
- `EcceCegerToolchain` constrói `CegerCardConfig` e `MiddlewareConfig` com
  defaults ECCE/Bit4ID.

## Assinatura destacada

`DetachedSignatureRequest` transporta os bytes canónicos a assinar, provider,
formato, profile e referências técnicas. `ExternalSigner` é o port de infra para
um mecanismo concreto de assinatura. `DetachedSigningService` valida o pedido,
invoca o signer e gera `SigningEvidence` com hashes SHA-256 dos bytes assinados
e do artefacto DER recebido.

`CommandExternalSigner` é o adapter concreto inicial: executa um binário externo,
envia `bytes_to_sign` por stdin e espera a assinatura DER por stdout. O processo
recebe metadados por variáveis `MINI_SIGNING_PROVIDER`, `MINI_SIGNING_FORMAT`,
`MINI_SIGNING_PROFILE` e `MINI_SIGNING_HASH_HEX`.

## Invariantes

- `Config` com provider baseado em certificado exige `certificate_ref`.
- `signature_pin_ref` exige formato `scheme:ref`.
- `OtcConfig.ttl_seconds`, `max_attempts` e `code_length` devem ser maiores que zero.
- `OtcIssuer::verify` não altera `IssuedOtcRecord`; o caller deve chamar
  `IssuedOtcRecord::record_attempt` quando persistir uma tentativa.
- `OtcFlowService::verify` altera o estado operacional: incrementa tentativas
  recusadas e consome/elimina o registo no sucesso.
- `IssuedOtcRecord` guarda apenas hash e sal do código, não o código em claro.
- `IssuedOtcRecord.reference` é a única chave pública do desafio OTC; o código
  entregue nunca deve ser persistido.
- `DetachedSignature::validate_for` exige formato e hash assinados coerentes com
  o pedido original.
- `ChaveMovelDigitalArtifact.session_id` deve corresponder à sessão preparada.
- `ChaveMovelDigitalSession.signing_hash_hex` é o vínculo entre bytes canónicos,
  gateway CMD e evidência local.
- Um gateway CMD real deve preservar o vínculo entre `gateway_request_id`,
  `session_id` e `signing_hash_hex`; artefactos fora da sessão são rejeitados.
- Planners técnicos não executam side effects externos; `CommandExternalSigner`
  é o boundary explícito para execução de signer externo.
- Providers de middleware real exigem referências explícitas para módulos,
  endpoints, cartões, tokens, PINs e/ou TSA; segredos são sempre referências.

## Limitações atuais

- PAdES/CAdES/XAdES completos ainda dependem de runtime/host ou backend dedicado;
  `NativePkcs11Signer` produz assinatura destacada DER sobre bytes canónicos.
- Sem SDK nativo embutido para Cartão de Cidadão, Autenticação.gov, ECCE/CEGER,
  TSA, HSM ou EUTL além de PKCS#11 via `cryptoki`.
- Sem persistência de OTC/evidência; o host deve persistir estado, tentativas e evidência.
- Validação de referência de segredo é sintática e não resolve o segredo.

## ToDo

- Implementar adapters nativos para PAdES/CAdES quando houver backend aprovado.
- Integrar `infra-secrets` para resolução segura de PIN/token quando necessário.
- Adicionar persistência de janelas OTC em adapter próprio.
- Ligar evidências de assinatura ao event log documental e/ou `core-audit`.

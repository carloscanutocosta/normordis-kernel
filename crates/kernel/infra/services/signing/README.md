# infra-signing

Adapters de infraestrutura para serviços de assinaturas digitais do Mini-Kernel RS.

## Responsabilidade

- Modelar bindings técnicos para providers de assinatura.
- Validar configuração mínima de certificado qualificado, OTC, Cartão de Cidadão PT,
  Chave Móvel Digital, middleware local/PKCS#11, Autenticação.gov/SAFE, TSA,
  HSM e ECCE/CEGER.
- Modelar autenticação forte com Cartão de Cidadão por mutual TLS, desafio
  assinado, SDK oficial C++/Java ou PKCS#11.
- Produzir planos técnicos de operações para orquestração por runtime/host.
- Emitir, entregar e verificar códigos OTC de confirmação forte com registo
  persistível por hash, limite de tentativas, TTL e consumo no sucesso.
- Criar pedidos de assinatura destacada e evidência técnica verificável.
- Preparar sessões remotas de Chave Móvel Digital e materializar o artefacto
  devolvido por gateway/serviço autorizado.
- Oferecer `MockChaveMovelDigitalGateway` para desenvolvimento e
  `HttpChaveMovelDigitalGateway` com transporte injetável para gateways reais.
- Executar assinatura real via PKCS#11 com a feature `native-pkcs11`, incluindo
  Cartão de Cidadão quando o middleware Autenticação.gov está instalado.
- Executar um signer externo por `CommandExternalSigner` quando o host fornece
  um binário/CLI de HSM, PKCS#11, Cartão de Cidadão ou serviço local.

## Não responsabilidade

- Não embute SDKs vendor-specific de TSA, HSM, middleware de smartcard, ECCE/CEGER
  ou Autenticação.gov; usa PKCS#11 nativo quando disponível.
- Não guarda PINs, certificados ou segredos.
- Não persiste OTC nem evidências; a persistência fica no host/adapter dedicado.
- Não decide autorização de domínio nem finalização documental.
- Não substitui `core-documental`; apenas fornece adapters headless de infra.
- Não chama diretamente endpoints CMD sem contrato/credenciais de fornecedor; o
  host deve plugar gateway, CLI ou backend autorizado.

## Exemplo mínimo

```rust
use infra_signing::{
    CartaoCidadaoPtAdapter, CartaoCidadaoPtConfig, CartaoCidadaoPtMode, SignatureFormat,
};

let cfg = CartaoCidadaoPtConfig {
    profile: "cc-qualified-signature".into(),
    format: SignatureFormat::Pades,
    certificate_ref: "pkcs11:cc-sign-cert".into(),
    signature_pin_ref: "secret:cc-pin".into(),
    mode: CartaoCidadaoPtMode::Middleware,
    trust_service_ref: "eutl:pt-qualified-provider".into(),
    reader_ref: Some("pcsc:reader-1".into()),
    require_timestamp: true,
};

let plan = CartaoCidadaoPtAdapter.build_plan(&cfg)?;
assert_eq!(plan.operations.len(), 7);
# Ok::<(), infra_signing::SigningError>(())
```

## Diagnóstico PKCS#11

Com `native-pkcs11` é possível confirmar o módulo, leitor, token e objetos
visíveis sem pedir PIN:

```powershell
cargo run -p infra-signing --features native-pkcs11 --example probe_pkcs11 -- C:\Windows\System32\pteidpkcs11.dll
```

Num Cartão de Cidadão real o certificado de assinatura costuma surgir como
`CITIZEN SIGNATURE CERTIFICATE` e a chave como `CITIZEN SIGNATURE KEY`, ambos com
`id_hex = 46000000`.

## Fluxo CMD

O fluxo recomendado para Chave Móvel Digital é:

1. `ChaveMovelDigitalService::prepare_session` calcula o hash dos bytes
   canónicos e cria a sessão.
2. Um `ChaveMovelDigitalGateway` inicia a assinatura remota.
3. O host faz polling/recebe callback até obter `ChaveMovelDigitalArtifact`.
4. `materialize_signature` e `evidence_for_signature` fecham a evidência local.

`MockChaveMovelDigitalGateway` permite testar UI e workflow sem credenciais CMD.
`HttpChaveMovelDigitalGateway` recebe um transporte HTTP injetado pelo host, para
evitar hardcode de endpoints e dependências de fornecedor dentro deste crate.

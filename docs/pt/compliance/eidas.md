---
title: "Normordis Kernel — Conformidade eIDAS 2 e Identidade Electrónica"
type: compliance
framework: eIDAS2
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/arquivistica.md
  - docs/pt/compliance/interoperabilidade.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade eIDAS 2 e Identidade Electrónica

## Declaração de Conformidade

O Normordis Kernel implementa a infra-estrutura técnica necessária para produzir documentos com assinatura electrónica qualificada (QES) e para integrar serviços de identificação electrónica europeus e nacionais. Três invariantes materializam esta declaração:

1. **Documentos assinados são imutáveis e auditados** — `services/signing` produz assinaturas sobre documentos finalizados em `core-documental`; cada operação de assinatura gera um `AuditEvent` no `core-audit`. A assinatura é rastreável, o signatário é identificável, e o acto é irreversível no log.

2. **Pipeline de documento suporta PAdES** — o `normordis-pdf` (ecossistema Normordis) gera PDF/A com suporte a PAdES (PDF Advanced Electronic Signature), o formato de assinatura qualificada exigido para documentos administrativos.

3. **Contratos de autenticação preparados para eIDAS 2** — `support-auth` define abstracções de autenticação e sessão agnósticas de mecanismo, prontas a receber implementações baseadas em CMD, Cartão de Cidadão ou Carteira Digital Europeia (EUDIW).

---

## Sumário Executivo

O Regulamento (UE) 2024/1183 (eIDAS 2) actualiza o quadro europeu de identidade electrónica e serviços de confiança, introduzindo a Carteira Digital de Identidade Europeia (EUDIW) e reforçando os requisitos para assinaturas electrónicas qualificadas e prestadores de serviços de confiança qualificados.

Para a Administração Pública portuguesa, eIDAS 2 é especialmente relevante em três vectores: a validade jurídica de documentos assinados electronicamente (equivalência à assinatura manuscrita para QES), a autenticação de utilizadores em serviços digitais (Autenticação.Gov, CMD, Cartão de Cidadão, EUDIW), e a interoperabilidade transfronteiriça de identidades e documentos.

O kernel contribui com o `services/signing` para assinatura de documentos, com `support-auth` para contratos de autenticação, e com `core-documental` para o ciclo de vida de documentos que terminam em assinatura. A integração concreta com a CMD e a EUDIW está planeada; a infra-estrutura de suporte está implementada.

## Âmbito

Cobre o mapeamento entre os requisitos do eIDAS 2 e a implementação do Normordis Kernel versão `0.1.x`. Não cobre:

- Qualificação como Prestador de Serviços de Confiança Qualificado (PSCQ) — o kernel não é um PSCQ
- Emissão de certificados qualificados — responsabilidade do SCEE/ARTE ou de PSCQ acreditados
- Gestão de chaves de utilizadores finais — responsabilidade do dispositivo de criação de assinatura qualificada (QSCD)
- Implementação da EUDIW — responsabilidade dos Estados-Membros; Portugal: eid.portugal.gov.pt

## Referências Normativas

**Regulamentação Europeia**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [eIDAS2] | Regulamento (UE) 2024/1183, de 11 de abril de 2024 | Integralmente | Identidade electrónica europeia, assinaturas e serviços de confiança |
| [eIDAS1] | Regulamento (UE) 910/2014 (eIDAS 1, parcialmente em vigor) | Art. 25.º–35.º | Efeitos jurídicos de assinaturas electrónicas; ainda referência para interoperabilidade transitória |
| [EUDIW] | Regulamento (UE) 2024/1183 | Art. 6.º-a | Carteira Digital de Identidade Europeia — identificação e atributos verificáveis |

**Legislação Nacional (Portugal)**

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [DL-12/2021] | DL n.º 12/2021, de 22 de fevereiro | Integralmente | Revisão do regime nacional de assinaturas electrónicas; alinhamento com eIDAS 1 |
| [CMD-PORT] | Portaria n.º 316/2020, de 31 de dezembro | Integralmente | Requisitos técnicos da Chave Móvel Digital (CMD) |
| [SCEE] | DL n.º 116-A/2006, de 16 de junho | Integralmente | Sistema de Certificação Electrónica do Estado |

**Normas técnicas**

| Ref. | Norma | Relevância |
|------|-------|------------|
| [PADES] | ETSI EN 319 132 — PAdES (PDF Advanced Electronic Signatures) | Formato de assinatura qualificada em PDF |
| [XADES] | ETSI EN 319 132 — XAdES (XML Advanced Electronic Signatures) | Formato de assinatura para documentos XML |
| [ISO32000] | ISO 32000-2 — PDF 2.0 | Especificação base para PDF com assinaturas |

---

## Tipos de Assinatura Electrónica e Posicionamento do Kernel

O eIDAS 2 define três níveis de assinatura electrónica com requisitos e efeitos jurídicos distintos:

```
Assinatura Electrónica Simples (SES)
    Qualquer dado electrónico associado a outros dados
    → kernel: qualquer registo com ActorId em AuditEvent

Assinatura Electrónica Avançada (AdES)
    Uniquamente ligada ao signatário
    Identifica o signatário
    Criada com dados sob controlo exclusivo do signatário
    Detecta alterações subsequentes
    → kernel: services/signing + hash de documento + ActorId

Assinatura Electrónica Qualificada (QES)         ← equivale à assinatura manuscrita
    AdES + certificado qualificado (QSCD)
    Efeito jurídico equivalente à assinatura manuscrita (Art. 25.º)
    → kernel: services/signing + CMD/Cartão de Cidadão + PAdES via normordis-pdf
```

O kernel posiciona-se como a **plataforma de orquestração** da assinatura: prepara o documento, invoca o serviço de assinatura (CMD, SCEE, ou outro PSCQ), e regista o evento no audit trail. A criação da assinatura criptográfica em si é delegada ao QSCD (CMD, Cartão de Cidadão).

---

## Mapeamento por Artigo eIDAS 2

### Art. 6.º-a — Carteira Digital de Identidade Europeia (EUDIW)

A EUDIW permite que cidadãos europeus partilhem atributos verificáveis (identidade, qualificações, atributos profissionais) com serviços públicos e privados.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Contratos de autenticação agnósticos | `support-auth` | Abstracções que suportam EUDIW como mecanismo | Preparado |
| Integração EUDIW | — | Implementação do protocolo OpenID4VP/SD-JWT | Planeado |
| Atributos verificáveis em `core-rh` | `core-rh` | `ActorId` ligável a atributos EUDIW | Planeado |

### Art. 25.º — Efeitos Jurídicos das Assinaturas Electrónicas

O Art. 25.º estabelece que uma QES tem efeito jurídico equivalente à assinatura manuscrita e é admissível como prova em processos judiciais.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Assinatura de documentos finalizados | `services/signing` | Assinatura invocada após estado "finalizado" em `core-documental` | Implementado |
| Imutabilidade pós-assinatura | `core-documental` | Transição de estado unidireccional — documento assinado não editável | Implementado |
| Audit trail do acto de assinatura | `core-audit` | `AuditEvent` com actor, timestamp, hash do documento | Implementado |
| Formato PAdES (PDF qualificado) | `normordis-pdf` (ecossistema) | PDF/A com suporte a PAdES | Implementado (ecossistema) |

### Arts. 26.º–28.º — Assinatura Electrónica Avançada e Qualificada

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Ligação ao signatário (AdES) | `services/signing` + `core-rh` | `ActorId` + dados de certificado | Implementado (base) |
| Detecção de alterações pós-assinatura | `core-audit` (hash encadeado) | Hash do documento no `AuditEvent`; alteração detectável | Implementado |
| Integração com QSCD — CMD | — | Integração Chave Móvel Digital (ARTE/iAP) | Planeado |
| Integração com QSCD — Cartão de Cidadão | — | Integração via SCEE | Planeado |
| Validação de certificados qualificados | — | Verificação contra lista de PSCQ (QTSL europeia) | Planeado |

### Arts. 32.º–35.º — Selos Electrónicos (para pessoas colectivas)

Os selos electrónicos garantem a origem e integridade de documentos emitidos por entidades (não por pessoas singulares) — particularmente relevante para documentos institucionais da AP.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Emissão de documentos institucionais | `core-documental` + `services/signing` | Documentos com identidade da entidade emissora | Implementado (base) |
| Selo electrónico em documentos PDF | `normordis-pdf` + `services/signing` | PAdES com certificado de entidade | Planeado |
| Identificação da entidade emissora | `core-org` | Unidade orgânica emissora registada no documento | Implementado |

### Art. 40.º — Selos Temporais Electrónicos

Os selos temporais qualificados provam que um documento existia num determinado momento.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Timestamp em todos os eventos | `core-audit` | `ControlExecution.timestamp` em cada `AuditEvent` | Implementado |
| Abstracção de tempo testável | `support-clock` | `Clock` trait — determinístico em testes, real em produção | Implementado |
| Selo temporal qualificado (TSA) | — | Integração com TSA qualificado (RFC 3161) | Planeado |

---

## Integração com Serviços de Identidade Portugueses

### Chave Móvel Digital (CMD)

A CMD é o serviço português de assinatura electrónica qualificada gerido pela ARTE. Permite assinar documentos via telemóvel com PIN e autenticação de dois factores.

| Funcionalidade | Crate de integração | Status |
|---------------|--------------------|----|
| Autenticação com CMD | `support-auth` (contrato) | Planeado |
| Assinatura qualificada via CMD | `services/signing` (orquestração) | Planeado |
| Registo de assinatura CMD no audit trail | `core-audit` | Planeado |

### Autenticação.Gov

Serviço de autenticação forte gerido pela ARTE, baseado em Cartão de Cidadão ou CMD.

| Funcionalidade | Crate de integração | Status |
|---------------|--------------------|----|
| Autenticação via Autenticação.Gov (OpenID Connect) | `support-auth` (contrato) | Planeado |
| Atributos de identidade em `core-rh` | `core-rh` | Planeado |

### Cartão de Cidadão

O Cartão de Cidadão contém certificados qualificados de autenticação e assinatura emitidos pelo SCEE.

| Funcionalidade | Crate de integração | Status |
|---------------|--------------------|----|
| Autenticação com Cartão de Cidadão | `support-auth` (contrato) | Planeado |
| Assinatura qualificada via Cartão de Cidadão | `services/signing` | Planeado |

---

## Fluxo de Assinatura Qualificada no Kernel

```
1. Documento em core-documental (estado: "em revisão")
        │
        ▼
2. Validação final — core-validation
        │
        ▼
3. Geração de PDF/A — normordis-pdf (PAdES ready)
        │
        ▼
4. Transição para estado "aguarda assinatura" — core-documental
        │
        ▼
5. Invocação do QSCD — services/signing
        │  (CMD / Cartão de Cidadão / outro PSCQ)
        │
        ▼
6. Assinatura PAdES aplicada ao PDF/A — normordis-pdf + services/signing
        │
        ▼
7. Transição para estado "assinado" — core-documental (imutável)
        │
        ▼
8. AuditEvent { control_id, actor, timestamp, hash_documento } — core-audit
        → Outbox transaccional (mesmo commit)
        → AuditStore (imutável)
```

---

## Matriz de Conformidade

| Requisito eIDAS 2 | Crate / Módulo | Evidência | Status |
|------------------|---------------|-----------|--------|
| Imutabilidade de documento assinado (Art. 25.º) | `core-documental` | Estado "assinado" unidireccional | Implementado |
| Audit trail do acto de assinatura (Art. 25.º) | `core-audit` | `AuditEvent` com actor, timestamp, hash | Implementado |
| Formato PAdES para QES (Arts. 26.º–28.º) | `normordis-pdf` (ecossistema) | PDF/A com suporte PAdES | Implementado (ecossistema) |
| Identificação da entidade emissora (Arts. 32.º–35.º) | `core-org` | Unidade orgânica emissora no documento | Implementado |
| Timestamp em eventos de assinatura (Art. 40.º) | `core-audit`, `support-clock` | `ControlExecution.timestamp` | Implementado |
| Abstracção de autenticação (Art. 11.º) | `support-auth` | Contratos agnósticos de mecanismo | Implementado |
| Assinatura de documentos finalizados | `services/signing` | Orquestração do fluxo de assinatura | Implementado |
| Ligação ao signatário (AdES — Art. 26.º) | `services/signing` + `core-rh` | ActorId + dados de certificado | Implementado (base) |
| Integração CMD — autenticação forte | — | OpenID Connect / ARTE | Planeado |
| Integração CMD — QES | — | Chave Móvel Digital (ARTE/iAP) | Planeado |
| Integração Cartão de Cidadão | — | SCEE | Planeado |
| Validação de certificados qualificados (QTSL) | — | Verificação contra trusted service list | Planeado |
| Selo temporal qualificado (Art. 40.º) | — | TSA qualificado (RFC 3161) | Planeado |
| Integração EUDIW (Art. 6.º-a) | — | OpenID4VP / SD-JWT | Planeado |

## Limitações e Exclusões Conhecidas

- **Não é PSCQ**: o kernel não é qualificado como Prestador de Serviços de Confiança Qualificado. A criação de certificados qualificados e a operação de QSCD são responsabilidades do SCEE, da ARTE/CMD, ou de PSCQ acreditados pelo CNCS.
- **CMD e Cartão de Cidadão**: a integração com os serviços de assinatura e autenticação portugueses está planeada mas não implementada. O kernel dispõe dos contratos (`services/signing`, `support-auth`) mas não das implementações concretas.
- **EUDIW**: a Carteira Digital de Identidade Europeia está em fase de implementação pelos Estados-Membros (2026); Portugal disponibiliza eid.portugal.gov.pt. A integração com o kernel (OpenID4VP / SD-JWT) está identificada como trabalho futuro.
- **Selo temporal qualificado**: `support-clock` fornece timestamps locais de alta fidelidade mas não integra com uma TSA (Time Stamping Authority) qualificada. A integração RFC 3161 está planeada.
- **Validação de certificados**: a verificação de certificados qualificados contra a Trusted Service List (QTSL) europeia não está implementada; está dependente da integração com serviços externos.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AdES** | Advanced Electronic Signature — assinatura electrónica avançada; ligada ao signatário, detecta alterações |
| **Autenticação.Gov** | Serviço português de autenticação forte gerido pela ARTE; baseado em Cartão de Cidadão ou CMD |
| **CAdES** | CMS Advanced Electronic Signature — formato AdES para documentos CMS |
| **Cartão de Cidadão** | Documento de identificação português com certificados qualificados de autenticação e assinatura (SCEE) |
| **CMD** | Chave Móvel Digital — serviço português de assinatura electrónica qualificada via telemóvel (ARTE) |
| **eIDAS 2** | Regulamento (UE) 2024/1183 — quadro europeu de identidade electrónica e serviços de confiança |
| **EUDIW** | European Union Digital Identity Wallet — Carteira Digital de Identidade Europeia; introduzida pelo eIDAS 2 |
| **JAdES** | JSON Advanced Electronic Signature — formato AdES para estruturas JSON |
| **PAdES** | PDF Advanced Electronic Signature (ETSI EN 319 132) — formato de assinatura qualificada em PDF |
| **PSCQ** | Prestador de Serviços de Confiança Qualificado — entidade acreditada para emitir certificados qualificados e operar QSCDs |
| **QES** | Qualified Electronic Signature — assinatura electrónica qualificada; equivalência jurídica à assinatura manuscrita |
| **QSCD** | Qualified Signature Creation Device — dispositivo de criação de assinatura qualificada (ex: smartcard, servidor HSM) |
| **QTSL** | EU Qualified Trusted Services List — lista europeia de prestadores de serviços de confiança qualificados |
| **RFC 3161** | Internet X.509 PKI Time-Stamp Protocol — protocolo de selo temporal qualificado |
| **SCEE** | Sistema de Certificação Electrónica do Estado — emite certificados para Cartão de Cidadão e CMD |
| **SES** | Simple Electronic Signature — assinatura electrónica simples; qualquer dado electrónico associado a outros |
| **TSA** | Time Stamping Authority — entidade que emite selos temporais qualificados (RFC 3161) |
| **XAdES** | XML Advanced Electronic Signature — formato AdES para documentos XML |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — eIDAS 2, CMD, EUDIW, fluxo de assinatura qualificada, PAdES |

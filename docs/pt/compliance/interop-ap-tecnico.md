---
title: "Normordis Kernel — Interoperabilidade AP: Protocolos e Normas Técnicas"
type: compliance
framework: [iAP, RNID, SAF-T, CIUS-PT, WS-Security, SOAP]
status: draft
version: 0.1.0
date: 2026-06-12
lang: pt
audience: [technical, architect, auditor]
approved_by: ""
related:
  - docs/pt/compliance/interoperabilidade.md
  - docs/pt/compliance/eidas.md
  - docs/pt/compliance/seguranca-informacao.md
  - crates/kernel/core/core-ingest/MAN.md
---

# Normordis Kernel — Interoperabilidade AP: Protocolos e Normas Técnicas

## Propósito deste documento

Este documento regista os protocolos, formatos e normas técnicas de troca de dados
vigentes na Administração Pública portuguesa, com base na legislação e regulamentação
aplicável consultada em Junho de 2026. Serve de referência de design para o componente
`core-ingest`, responsável por receber, sanitizar e encaminhar dados externos para
persistência via `core-documental`.

---

## Enquadramento Legal

### Legislação nacional

| Referência | Diploma | Data | Conteúdo relevante |
|------------|---------|------|--------------------|
| **Lei 36/2011** | Lei n.º 36/2011, de 21 de Junho | 21 Jun 2011 | Estabelece a adopção de normas abertas nos sistemas de informação do Estado. Atribui à AMA (hoje ARTE) a responsabilidade pelo RNID. Define que os sistemas de informação da AP devem usar normas técnicas abertas para garantir interoperabilidade. |
| **RCM 91/2012** | Resolução do Conselho de Ministros n.º 91/2012, de 8 de Novembro | 8 Nov 2012 | Aprova o RNID v1 (Regulamento Nacional de Interoperabilidade Digital). Torna obrigatórios os formatos abertos ODT, ODS, ODP e PDF para documentos da AP. Primeira versão do referencial técnico nacional de interoperabilidade. |
| **DL 73/2014** | Decreto-Lei n.º 73/2014 | 2014 | Princípio "once-only": a AP não pode exigir a um cidadão ou empresa documentos que já detenha noutros sistemas. Fundamento legal para a interoperabilidade automática entre sistemas da AP. |
| **RCM 42/2015** | Resolução do Conselho de Ministros n.º 42/2015, de 19 de Junho | 19 Jun 2015 | Define a iAP-PI (Plataforma de Integração da AP) como a plataforma preferencial e obrigatória para comunicação inter-agências. Toda a troca de dados entre organismos da AP deve passar pela iAP-PI. |
| **RCM 2/2018** | Resolução do Conselho de Ministros n.º 2/2018, de 5 de Janeiro | 5 Jan 2018 | Aprova o RNID v2. Revisão do referencial nacional; alinhamento com o Regulamento (UE) 1025/2012 sobre normalização europeia. Distingue especificações técnicas obrigatórias das recomendadas. |
| **DL 49/2024** | Decreto-Lei n.º 49/2024, de 8 de Agosto | 8 Ago 2024 | Mandato mais exigente até à data. Os serviços digitais da AP **devem** integrar obrigatoriamente com: iAP-PI (integração), iAP-GAP (mensagens), iAP-PPAP (pagamentos). Autenticação obrigatoriamente via autenticacao.gov. Conectividade obrigatória via VPN IPsec ou PTT. Aplica-se a todos os sistemas que implementem ou modifiquem substancialmente serviços digitais da AP. |

### Regulamentação europeia com impacto técnico directo

| Referência | Diploma | Data | Conteúdo relevante |
|------------|---------|------|--------------------|
| **Reg. 1025/2012** | Regulamento (UE) n.º 1025/2012 do Parlamento Europeu e do Conselho, de 25 de Outubro de 2012 | 25 Out 2012 | Regulamento europeu sobre normalização. Base para o RNID v2 (RCM 2/2018). Define o processo de adopção de normas europeias em sistemas de informação públicos. |
| **eIDAS** | Regulamento (UE) n.º 910/2014 do Parlamento Europeu e do Conselho, de 23 de Julho de 2014 | 23 Jul 2014 | Identidade electrónica e serviços de confiança. Define os requisitos de assinatura electrónica qualificada (QES). Base para a CMD (Chave Móvel Digital) e para a aceitação do Cartão de Cidadão como meio de autenticação/assinatura. |
| **eIDAS2** | Regulamento (UE) 2024/1183 do Parlamento Europeu e do Conselho, de 11 de Abril de 2024 | 11 Abr 2024 | Revisão do eIDAS. Introduz a Carteira Europeia de Identidade Digital (EUDI Wallet). Alarga os requisitos de interoperabilidade de identidade entre Estados-Membros. |
| **EN 16931-2017** | Norma Europeia EN 16931-1:2017 + EN 16931-2:2017 (CEN) | 2017 | Norma europeia para facturação electrónica. Define o modelo semântico (EN 16931-1) e a sintaxe de ligação para UBL 2.1 e UN/CEFACT CII (EN 16931-2). Base obrigatória para o CIUS-PT. |
| **Directiva 2014/55/UE** | Directiva 2014/55/UE do Parlamento Europeu e do Conselho, de 16 de Abril de 2014 | 16 Abr 2014 | Facturação electrónica nos contratos públicos. Obriga as entidades adjudicantes da AP a receber e processar facturas electrónicas conformes à EN 16931. Transposta em Portugal pelo DL 111-B/2017. |
| **DL 111-B/2017** | Decreto-Lei n.º 111-B/2017, de 31 de Agosto | 31 Ago 2017 | Transpõe a Directiva 2014/55/UE. Obriga a AP a aceitar facturas electrónicas. Datas: grandes empresas desde 2021; PME desde 2023; todas as entidades desde 2025. |

---

## Plataforma iAP — Arquitectura Técnica

A iAP (gerida pela ARTE — Agência para a Reforma Tecnológica do Estado, antiga AMA)
é composta por quatro plataformas independentes:

| Plataforma | Sigla | Função |
|-----------|-------|--------|
| Plataforma de Integração | iAP-PI | ESB central — mediação, orquestração e roteamento de mensagens entre organismos |
| Gateway de Mensagens | iAP-GAP | SMS obrigatório para serviços da AP |
| Plataforma de Pagamentos | iAP-PPAP | Pagamentos electrónicos (Multibanco, MBWay, cartões, PayPal) |
| Interoperabilidade Documental | iAP-ID | Troca de documentos entre sistemas de gestão documental (operacional desde Junho 2025) |

A iAP-PI actua como **ESB (Enterprise Service Bus)**:
- Mediação: tradução de protocolos e transformação de formatos entre sistemas
- Orquestração: motor BPEL para fluxos multi-passo
- Roteamento: WS-Addressing para respostas assíncronas
- Catálogo: directório de serviços disponível a todos os organismos integrados
- Ponto único: um organismo integra uma vez com a iAP e acede a todos os serviços expostos

A iAP-PI tem certificação **ISO 27001** (Segurança de Informação).
Mais de 124 entidades integradas; mais de 6,3 mil milhões de interacções processadas (2025).

---

## Protocolos de Comunicação (iAP-PI)

### Stack de protocolo

```
Transporte:    HTTP (obrigatório) ou HTTPS (recomendado)
Protocolo:     SOAP 1.1 ou SOAP 1.2
Perfil WS:     WS-I Basic Profile 1.1
Descrição:     WSDL 1.1
Async:         WS-Addressing v1.0 (MessageID, RelatesTo, ReplyTo, To, Action)
Fiabilidade:   Entrega At-Least-Once; deduplicação por MessageID
Reenvios:      Máximo 5 tentativas a intervalos de 10 minutos
Alternativa:   REST/JSON (disponível para serviços leves — não predominante)
Orquestração:  BPEL (Business Process Execution Language) em fluxos complexos
```

SOAP/XML é o protocolo primário e historicamente dominante na iAP-PI. REST/JSON
é aceite mas não é o canal principal.

### Autenticação de mensagem (WS-Security)

```
Norma:      WS-Security 1.1 (OASIS)
Perfil:     UsernameToken Profile 1.1
Cifra:      Palavra-passe cifrada com RSA + chave pública da entidade receptora
```

Exemplo de cabeçalho WS-Security (AT):

```xml
<wss:Security>
  <wss:UsernameToken>
    <wss:Username>NIF/subuser</wss:Username>
    <wss:Password><!-- RSA(ChavePublicaAT, password) --></wss:Password>
    <wss:Nonce></wss:Nonce>
    <wss:Created></wss:Created>
  </wss:UsernameToken>
</wss:Security>
```

### Conectividade de rede (DL 49/2024, Art. 8.º)

- **VPN IPsec** para a rede RCTS/Governo, **ou**
- **PTT** (Ponto de Troca de Tráfego governamental)
- Certificados de servidor da cadeia DGITA/ARTE Root CA instalados no cliente

### Autenticação de identidade

| Mecanismo | Norma | Uso |
|-----------|-------|-----|
| SAML 2.0 | OASIS SAML 2.0 | Autenticação federada para serviços web |
| OAuth 2.0 | RFC 6749 | Autenticação API para integrações programáticas |
| Cartão de Cidadão | X.509 no chip; PKI com SCEE Root CA | Autenticação presencial e qualificada |
| CMD | Chave Móvel Digital; eIDAS QES | Assinatura electrónica qualificada à distância |
| SCAP | Sistema de Certificação de Atributos Profissionais | Atributos profissionais certificados em assinatura |

Todos os serviços digitais da AP devem usar **autenticacao.gov** como plataforma de
autenticação (DL 49/2024, Art. 3.º, n.º 2).

### Assinatura de documentos

| Formato | Norma | Uso |
|---------|-------|-----|
| XAdES | ETSI EN 319 132 | Assinatura de documentos XML |
| CAdES | ETSI EN 319 122 | Assinatura de dados binários (CMS) |
| PAdES | ETSI EN 319 132 + ISO 32000 | Assinatura de PDF |
| ASiC | ETSI TS 102 918 | Contentores de assinatura (usado pelo Cartão de Cidadão) |

---

## Formatos XML Mandatados

### SAF-T (PT) — Ficheiro de Auditoria Fiscal

```
Namespace:  urn:OECD:StandardAuditFile-Tax:PT_1.04_01
Versão:     1.04_01 (versão actual)
Autor:      Autoridade Tributária e Aduaneira (AT)
Base:       OECD Standard Audit File for Tax v1.0
XSD:        portaldasfinancas.gov.pt
```

Obrigatório para todos os organismos com obrigações de contabilidade. Estrutura:

```
AuditFile
  ├── Header
  ├── MasterFiles
  │     ├── GeneralLedgerAccounts
  │     ├── Customer* / Supplier* / Product*
  │     └── TaxTable
  ├── GeneralLedgerEntries
  └── SourceDocuments
        ├── SalesInvoices
        ├── MovementOfGoods
        ├── WorkingDocuments
        └── Payments
```

### CIUS-PT — Factura Electrónica B2G

```
Formato:          UBL 2.1 ou UN/CEFACT CII
CustomizationID:  urn:cen.eu:en16931:2017#compliant#urn:feap.gov.pt:CIUS-PT:2.1
Base:             EN 16931-2017 (Norma Europeia de Facturação Electrónica)
Perfil Peppol:    Peppol BIS 3.0 também aceite

Namespaces principais:
  xmlns     = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
  xmlns:cbc = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
  xmlns:cac = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
  xmlns:ext = "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2"
```

Campos obrigatórios: `CustomizationID`, `ID`, `IssueDate`, `InvoiceTypeCode`
(380/381/383), `DocumentCurrencyCode`, `AccountingSupplierParty`,
`AccountingCustomerParty`, `LegalMonetaryTotal`, `InvoiceLine` (1..N).

A partir de 2026: QR Code + ATCUD (Código Único de Documento) obrigatórios;
Assinatura Electrónica Qualificada (QES) em PDF.

Gerido pela eSPap (Entidade de Serviços Partilhados da AP), que é também a
**Autoridade Peppol** de Portugal.

### Dados do Cartão de Cidadão (XML via SDK)

O middleware do Cartão de Cidadão expõe dados via `PTEID_EId.getXmlCCDoc()` em XML.
Não existe namespace público fixo; estrutura definida pelo SDK (autenticacao.gov).

Campos disponíveis: `GivenName`, `Surname`, `Gender`, `DateOfBirth`, `CivilID` (NIC),
`TaxNo` (NIF), `SocialSecurityNo` (NISS), `HealthNo` (NS), `MRZ` (linhas),
`District`, `Municipality`, `CivilParish`, `StreetType`, `StreetName`, códigos
postais, suporte a morada estrangeira. Fotografia em base64 PNG.

Contentores de assinatura usam **ETSI TS 102 918** (ASiC).

### Interoperabilidade Documental iAP (iAP-ID)

Usa **MIP** (Meta-informação para a Interoperabilidade Documental) e **MEF**
(Macroestrutura Funcional) como modelos canónicos de metadados.
Transmissão via subsistema "Large Files" da iAP-PI.
XSD específico não publicado publicamente (operacional desde Junho 2025).

### Webservices AT (e-Fatura / Documentos de Transporte)

Mensagens SOAP com corpo definido por WSDL publicado em portaldasfinancas.gov.pt.
Autenticação via WS-Security UsernameToken Profile 1.1 com password cifrada por RSA
com ChavePublicaAT (`ChavePublicaAT.cer` / `ChaveCifraPublicaAT2020.pem`).

---

## Trocas de Dados Inter-Agências Mais Comuns

| Troca | Organismos | Finalidade |
|-------|-----------|------------|
| Ciclo de vida do Cidadão | IRN ↔ AT ↔ SS ↔ SNS ↔ INCM ↔ MAI | Gestão de NIC, NIF, NISS, NS desde o nascimento |
| Alteração de morada | IRN → AT, SS, MAI, SNS | Propagação automática de actualização de morada |
| Validação de NIF | AT (via iAP) → qualquer organismo AP | Verificação de número de identificação fiscal |
| Validação de factura | AT (via iAP) → sistemas de compras AP | Verificação de conformidade e-Fatura/SAF-T |
| Tarifa Social Energia | AT + SS → distribuidoras | Verificação binária de elegibilidade (sem exposição de dados) |
| Bolsas de estudo | AT + SS → ensino superior | Avaliação automática de situação económica |
| Abertura de conta bancária | IRN + AT → bancos | Abertura desmaterializada de conta |
| Contribuições sociais | Empregadores → SS (PSI/DMR) | Declaração mensal de remunerações e contribuições |
| Registo predial | IRN ↔ AT ↔ DGT | Harmonização do cadastro BUPi |

**PSI** (Plataforma de Serviços de Interoperabilidade) é a plataforma específica da
Segurança Social, gerida pelo Instituto de Informática (II). Transição do DMR para PSI
obrigatória até 2027.

---

## Implicações para `core-ingest`

Com base neste enquadramento, o `core-ingest` tem de suportar **duas categorias** de
dados externos:

### Categoria 1 — Ficheiros binários

PDFs, imagens, DOCX, ZIP e outros ficheiros não estruturados enviados por cidadãos,
entidades privadas ou sistemas externos.

Requisitos mínimos:
- Hash SHA-256 dos bytes raw antes de qualquer parsing
- Verificação de magic bytes / detecção de tipo real vs. `content_type` declarado
- Limite de tamanho configurável
- Scan antimalware via `ScanAdapter` (já implementado)
- Armazenamento como BLOB em `core-documental`

### Categoria 2 — XML de interoperabilidade

Dados estruturados de outros organismos AP via iAP-PI, webservices SOAP, ou PSI.
Formatos concretos: SAF-T PT, CIUS-PT (UBL 2.1), dados CC (SDK), respostas SOAP AT.

Requisitos mínimos:
- **XXE Prevention obrigatório** antes de qualquer parsing XML
  (XML External Entity — vector de ataque clássico em XML governamental)
- Validação XSD quando `schema_id` é declarado na source
- Namespace detection para rotear para o validador correcto
- Preservação dos bytes raw originais para cadeia de evidência

### Tipos a declarar em `core-ingest`

| Tipo | Descrição |
|------|-----------|
| `IngestBundle` | Fronteira real: raw bytes + content_type + source + declared_hash |
| `IngestDecision` | Enum `Accepted` / `Rejected` (substituir String + constantes) |
| `ContentValidator` | Trait: `validate(&self, raw: &[u8], content_type: &str)` |
| `IngestStoragePort` | Trait: `store(&self, pkg: DocumentPackage) -> Result<DocumentRef>` |

A camada infra implementa `IngestStoragePort` usando `core-documental`.
`core-ingest` não depende de `core-documental` directamente — apenas define o port.

### Namespaces XML a reconhecer (mínimo)

| Namespace | Formato | Fonte |
|-----------|---------|-------|
| `urn:OECD:StandardAuditFile-Tax:PT_1.04_01` | SAF-T PT | AT |
| `urn:oasis:names:specification:ubl:schema:xsd:Invoice-2` | CIUS-PT UBL 2.1 | eSPap / fornecedores |
| `http://schemas.xmlsoap.org/soap/envelope/` | SOAP 1.1 | iAP-PI |
| `http://www.w3.org/2003/05/soap-envelope` | SOAP 1.2 | iAP-PI |
| `http://www.w3.org/2005/08/addressing` | WS-Addressing | iAP-PI (async) |
| *(sem namespace fixo)* | Dados CC / SDK | autenticacao.gov middleware |
| *(a definir pela iAP-ID)* | MIP/MEF documental | iAP-ID (2025) |

---

## Requisitos de Segurança Aplicáveis

| Camada | Requisito | Base legal |
|--------|-----------|------------|
| Rede | VPN IPsec ou PTT para alcançar endpoints iAP | DL 49/2024, Art. 8.º |
| Transporte | HTTPS; certificados da cadeia DGITA/ARTE Root CA | DL 49/2024 |
| Mensagem | WS-Security UsernameToken Profile 1.1 (OASIS) | iAP-PI especificação técnica |
| Cifra de passwords | RSA com chave pública da entidade receptora | iAP-PI / AT (ChavePublicaAT) |
| Identidade digital | Cartão de Cidadão (X.509) ou CMD | eIDAS (Reg. 910/2014), DL 49/2024, Art. 3.º |
| Autenticação federada | SAML 2.0 ou OAuth 2.0 via autenticacao.gov | DL 49/2024, Art. 3.º n.º 2 |
| Assinatura de documentos | QES conforme eIDAS (XAdES, CAdES, PAdES) | Reg. 910/2014 / eIDAS2 Reg. 2024/1183 |
| Atributos profissionais | SCAP para atributos certificados em assinatura | autenticacao.gov |
| Protecção de dados | RGPD; protocolos de dados obrigatórios na adesão à iAP | Reg. (UE) 2016/679 |
| Plataforma | ISO 27001 na iAP-PI, iAP-GAP e iAP-PPAP | Certificação ARTE |
| XML | XXE prevention antes de qualquer parsing XML | Boa prática obrigatória (OWASP) |

---

## Fontes Consultadas

### Fontes legais primárias (diplomas)

1. **Lei n.º 36/2011, de 21 de Junho** — Diário da República, 1.ª série, n.º 119.
   Normas abertas nos sistemas de informação do Estado.

2. **Resolução do Conselho de Ministros n.º 91/2012, de 8 de Novembro** — DR 1.ª série, n.º 216.
   RNID v1 — Regulamento Nacional de Interoperabilidade Digital.

3. **Decreto-Lei n.º 107/2012, de 18 de Maio** — DR 1.ª série, n.º 97.
   eSPap — Entidade de Serviços Partilhados da AP.

4. **Decreto-Lei n.º 73/2014** — Princípio "once-only" na AP portuguesa.

5. **Resolução do Conselho de Ministros n.º 42/2015, de 19 de Junho** — DR 1.ª série, n.º 118.
   iAP-PI como plataforma preferencial e obrigatória para comunicação inter-agências.

6. **Resolução do Conselho de Ministros n.º 2/2018, de 5 de Janeiro** — DR 1.ª série, n.º 4.
   RNID v2; alinhamento com Regulamento (UE) 1025/2012.

7. **Portaria n.º 195/2018, de 5 de Julho** — DR 1.ª série.
   Catálogo de serviços partilhados da iAP.

8. **Decreto-Lei n.º 111-B/2017, de 31 de Agosto** — DR 1.ª série.
   Transposição da Directiva 2014/55/UE; facturação electrónica nos contratos públicos.

9. **Decreto-Lei n.º 49/2024, de 8 de Agosto** — DR 1.ª série.
   Mandato de integração obrigatória com iAP; autenticação via autenticacao.gov;
   conectividade VPN IPsec/PTT.

### Fontes europeias primárias

10. **Regulamento (UE) n.º 1025/2012 do Parlamento Europeu e do Conselho, de 25 de Outubro de 2012** —
    JO L 316, 14.11.2012. Normalização europeia; base do RNID v2.

11. **Directiva 2014/55/UE do Parlamento Europeu e do Conselho, de 16 de Abril de 2014** —
    JO L 133, 6.5.2014. Facturação electrónica nos contratos públicos.

12. **Regulamento (UE) n.º 910/2014 do Parlamento Europeu e do Conselho, de 23 de Julho de 2014** (eIDAS) —
    JO L 257, 28.8.2014. Identidade electrónica e serviços de confiança.

13. **Regulamento (UE) 2016/679 do Parlamento Europeu e do Conselho, de 27 de Abril de 2016** (RGPD) —
    JO L 119, 4.5.2016. Protecção de dados pessoais; aplicável às trocas de dados inter-agências.

14. **Norma Europeia EN 16931-1:2017 + EN 16931-2:2017 (CEN)** — Facturação electrónica;
    modelo semântico e sintaxe de ligação para UBL 2.1 e UN/CEFACT CII.

15. **Regulamento (UE) 2024/903 do Parlamento Europeu e do Conselho, de 11 de Abril de 2024**
    (Interoperable Europe Act) — JO L, 22.4.2024. Ver também [interoperabilidade.md](interoperabilidade.md).

16. **Regulamento (UE) 2024/1183 do Parlamento Europeu e do Conselho, de 11 de Abril de 2024**
    (eIDAS2) — JO L, 30.4.2024. Carteira Europeia de Identidade Digital.

### Fontes técnicas e documentação oficial

17. **iAP — Plataforma de Integração (documentação oficial ARTE)** —
    https://www.iap.gov.pt/web/iap/plataforma-de-integracao
    Consultado: Junho 2026.

18. **ISCAPI — Integração de Serviços Comuns da AP (AMA GitHub)** —
    https://amagovpt.github.io/ISCAPI/iap/
    Consultado: Junho 2026.

19. **iAP no Mosaico (portal de transformação digital da AP)** —
    https://mosaico.gov.pt/plataformas-comuns/iap-pi
    Consultado: Junho 2026.

20. **DL 49/2024 — Perguntas Frequentes (Mosaico)** —
    https://guias.mosaico.gov.pt/guias-praticos/decreto-lei-no-49-2024-perguntas-frequentes/
    Consultado: Junho 2026.

21. **RNID 2018 — RCM 2/2018 (Mosaico)** —
    https://mosaico.gov.pt/legislacao-e-regulamentos/rcm-2-2018
    Consultado: Junho 2026.

22. **Lei 36/2011 (dre.tretas.org)** —
    https://dre.tretas.org/dre/284553/lei-36-2011-de-21-de-junho
    Consultado: Junho 2026.

23. **RCM 91/2012 (ANACOM)** —
    https://www.anacom.pt/render.jsp?contentId=1143512
    Consultado: Junho 2026.

24. **SAF-T PT XSD (GitHub — fredericoregateiro)** —
    https://github.com/fredericoregateiro/saft/blob/master/src/SolRIA.SaftAnalyser.Logic/SAFT/SAFTPT1.04_01.xsd
    Consultado: Junho 2026.

25. **CIUS-PT — estrutura UBL 2.1 (PHC GO Help Center)** —
    https://helpcenter.phcgo.net/pt/sug/ptxview.aspx?stamp=!!!dg5eeb7efg4f6e6b16758g
    Consultado: Junho 2026.

26. **eSPap — Normas de Fatura Electrónica** —
    https://www.espap.gov.pt/spfin/normas/Paginas/normas.aspx
    Consultado: Junho 2026.

27. **eInvoicing in Portugal (Comissão Europeia)** —
    https://ec.europa.eu/digital-building-blocks/sites/spaces/DIGITAL/pages/467108897/eInvoicing+in+Portugal
    Consultado: Junho 2026.

28. **PSI — Plataforma de Serviços de Interoperabilidade (Segurança Social)** —
    https://www.plataformaservicos.seg-social.pt/psi/
    Consultado: Junho 2026.

29. **iAP — Interoperabilidade Documental** —
    https://www.iap.gov.pt/web/iap/interoperabilidade-documental
    Consultado: Junho 2026.

30. **autenticacao.gov — Documentação SDK (AMA GitHub)** —
    https://amagovpt.github.io/docs.autenticacao.gov/manual_sdk.html
    Consultado: Junho 2026.

31. **CMD Assinatura — Documentação técnica (AMA GitHub)** —
    https://github.com/amagovpt/doc-CMD-assinatura
    Consultado: Junho 2026.

32. **autenticacao.gov — Integração (AMA GitHub)** —
    https://github.com/amagovpt/doc-AUTENTICACAO
    Consultado: Junho 2026.

33. **AtWS — AT WebService (Delphi, referência técnica de implementação)** —
    https://github.com/nunopicado/AtWS
    Consultado: Junho 2026.

34. **digital.gov.pt — Artigo sobre iAP** —
    https://digital.gov.pt/pt/noticias/iap-a-plataforma-de-interoperabilidade-que-liga-a-administracao-publica-portuguesa
    Consultado: Junho 2026.

---

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-12 | carloscanutocosta | Versão inicial — iAP, SAF-T, CIUS-PT, DL 49/2024, 34 fontes legais e técnicas |

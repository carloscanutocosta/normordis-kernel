# Regras de Negócio — core-ingest

`core-ingest` define ingestão auditável de dados externos (binários e XML da AP)
no fluxo `correlation_id → allowlist → validate → size → hash → scan → content_validate → store → audit`.

Destino final: `core-documental` (BLOB MinIO). Ver contexto legal em
[docs/pt/compliance/interop-ap-tecnico.md](../../docs/pt/compliance/interop-ap-tecnico.md).

---

## IngestSource

### INGEST-R01 — Campos obrigatórios não vazios

`kind`, `subject_id` e `version` são obrigatórios e não podem ser apenas whitespace.

### INGEST-R02 — kind identifica tipo interoperável

`kind` identifica o tipo semântico do bundle. Valores não presentes na allowlist
configurada são rejeitados. Os valores canónicos reconhecidos são:

| `kind` | Norma | Referência legal |
|--------|-------|-----------------|
| `cius-pt-invoice` | CIUS-PT UBL 2.1 (`urn:oasis:names:specification:ubl:schema:xsd:Invoice-2`) | Directiva 2014/55/EU, EN 16931-2017, DL 111-B/2017 |
| `saft-pt` | SAF-T PT v1.04_01 (`urn:OECD:StandardAuditFile-Tax:PT_1.04_01`) | DL 73/2014 |
| `iap-pi-message` | iAP-PI SOAP 1.1/1.2 (WS-I Basic Profile 1.1, WS-Addressing v1.0) | RCM 91/2012, DL 49/2024 |
| `pdf-official` | Documento PDF oficial (sem XSD) | — |
| `pdf-contract` | Contrato em PDF | — |

Extensões de `kind` devem seguir o padrão `<categoria>-<subtipo>` em minúsculas com hífens.

### INGEST-R03 — subject_id corresponde ao bundle

`subject_id` deve corresponder ao sujeito declarado no bundle. Divergências
indicam corrupção ou manipulação do bundle.

### INGEST-R04 — version corresponde ao bundle

`version` deve corresponder à versão declarada pelo remetente. Divergências
indicam bundle com versão inesperada.

---

## IngestBundle

### INGEST-R09 — content_type obrigatório e não vazio

`content_type` deve ser um MIME type não vazio. O pipeline usa-o para seleccionar
o `ContentValidator` adequado.

### INGEST-R10 — hash calculado sobre bytes raw

O hash SHA-256 é calculado sobre `raw` **antes de qualquer parsing**. Esta ordem
é obrigatória para segurança: garante que o que foi recebido é o que foi
auditado, mesmo que o parsing altere a representação interna.

### INGEST-R11 — XXE prevention obrigatória para XML

Quando `content_type` for `application/xml` ou variante (ex.: `text/xml`,
`application/atom+xml`), o `ContentValidator` deve aplicar XXE prevention antes
de qualquer parsing. Conformidade exigida por DL 49/2024 e Lei 36/2011.

---

## IngestEvidence

### INGEST-R05 — Toda decisão produz evidência

Toda decisão de ingestão (aceite ou rejeitada) deve produzir `IngestEvidence`.
Decisões sem evidência não são válidas nem auditáveis.

### INGEST-R06 — decision canónica

`decision` deve ser `accepted` ou `rejected`.

### INGEST-R07 — hash.algorithm declarado

`hash.algorithm` deve indicar o algoritmo usado (ex.: `SHA-256`).

### INGEST-R08 — audit.emitted via caminho canónico

`audit.emitted = true` apenas quando o evento foi construído por
`build_ingest_audit_event()` via `core-audit`. Eventos emitidos por outro
caminho invalidam a cadeia de evidências.

### INGEST-R12 — XSD quando schema_id declarado

Quando `source.kind` implica um schema XSD conhecido (ex.: SAF-T PT, CIUS-PT),
o `ContentValidator` deve validar o XML contra o XSD correspondente **após**
XXE prevention. Falha de validação XSD resulta em `rejected`.

### INGEST-R13 — document_ref obrigatório em accepted

`document_ref` deve estar presente quando `decision = accepted`. A ausência
indica que o bundle foi aceite mas não armazenado — estado inválido e
não auditável.

### INGEST-R14 — processed_at ≥ received_at

`processed_at` não pode ser anterior a `received_at`. Uma evidência com
`processed_at < received_at` indica manipulação de timestamp ou corrupção
de dados — estado inválido para COSO.

### INGEST-R15 — hash.verified implica declared_hash == actual_hash

Quando `hash.verified = true`, `hash.declared_hash` deve ser não vazio e
idêntico a `hash.actual_hash`. `verified = true` significa que o pipeline
verificou e confirmou a correspondência — qualquer divergência invalida
a evidência de integridade.

### INGEST-R16 — declared_hash vazio implica verified = false

Quando `hash.declared_hash` está vazio (remetente não declarou hash),
`hash.verified` deve ser `false`. Este caso ocorre quando o pipeline apenas
regista o hash calculado sem ter o valor declarado para comparar.
(Corolário de INGEST-R15.)

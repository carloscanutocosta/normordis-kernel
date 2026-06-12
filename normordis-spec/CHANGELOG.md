# Changelog

Todas as alterações significativas à `normordis-spec` são registadas aqui.
Formato: [Keep a Changelog](https://keepachangelog.com/); versionamento: [Semantic Versioning](https://semver.org/).

---

## [0.9.0] — 2026-06-12

### Adicionado — core-ingest completo

**Schemas novos:**
- `schemas/core/ingest/ingest-bundle.schema.json` — `IngestBundle` com `raw` em base64 (RFC 4648), `content_type` com pattern MIME, `declared_hash` com pattern `sha256:[0-9a-f]{64}`
- `schemas/core/ingest/ingest-decision.schema.json` — enum `accepted | rejected`
- `schemas/core/ingest/ingest-evidence.schema.json` — evidência completa com `$ref` para todos os sub-schemas
- `schemas/core/ingest/hash-evidence.schema.json` — `HashEvidence` (algorithm, declared_hash, actual_hash, verified)
- `schemas/core/ingest/scan-evidence.schema.json` — `ScanEvidence` (adapter, verdict, reason?)
- `schemas/core/ingest/validation-evidence.schema.json` — `ValidationEvidence` (content_type, valid, reason?)
- `schemas/core/ingest/audit-evidence.schema.json` — `AuditEvidence` (required, emitted, action, event_id?)

**Regras adicionadas:**
- INGEST-R09 — `content_type` obrigatório e não vazio
- INGEST-R10 — hash calculado sobre bytes raw antes de qualquer parsing
- INGEST-R11 — XXE prevention obrigatória para XML (DL 49/2024, Lei 36/2011)
- INGEST-R12 — XSD quando `schema_id` declarado (SAF-T PT, CIUS-PT)
- INGEST-R13 — `document_ref` obrigatório em `accepted` (camada 3)

**Fixtures novas (válidas):**
- `ingest-bundle-pdf.json`, `ingest-bundle-xml.json` — `IngestBundle`
- `ingest-decision-accepted.json`, `ingest-decision-rejected.json` — `IngestDecision`
- `ingest-evidence-accepted.json`, `ingest-evidence-rejected.json` — `IngestEvidence`

**Fixtures novas (inválidas):**
- `ingest-bundle-missing-raw.json`, `ingest-bundle-empty-content-type.json`, `ingest-bundle-invalid-hash-prefix.json` — camada 1
- `ingest-decision-unknown.json`, `ingest-evidence-missing-bundle-id.json` — camada 1
- `ingest-evidence-accepted-missing-document-ref.json` — camada 3 (INGEST-R13)

**Regras adicionadas (gap-closing — 2026-06-12):**
- INGEST-R14 — `processed_at ≥ received_at` (invariante temporal de evidência)
- INGEST-R15 — `hash.verified=true` implica `declared_hash == actual_hash` e não vazio
- INGEST-R16 — `declared_hash` vazio implica `verified=false` (corolário de R15)
- Secção de `source.kind` canónicos: `cius-pt-invoice`, `saft-pt`, `iap-pi-message`, `pdf-official`, `pdf-contract`
- `audit-evidence.action` com pattern `^[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*)+$` (alinhado com `core-audit`)

**Fixtures adicionadas (gap-closing):**
- `ingest-bundle-saft-pt.json` (válida) — `kind=saft-pt`, SAF-T PT v1.04_01
- `ingest-evidence-inverted-timestamps.json` (camada 3) — INGEST-R14
- `ingest-evidence-verified-hash-mismatch.json` (camada 3) — INGEST-R15

### Alterado

- `core-ingest/types.rs`: `IngestBundle.raw: Vec<u8>` passa a serializar como base64 standard via `#[serde(with = "base64_bytes")]`
- `core-ingest/service.rs`: `validate_ingest_evidence()` cobre agora INGEST-R14 e R15
- `rules/core-ingest.md`: revisão de R01–R08, adição de R09–R16; kinds canónicos documentados

---

## [0.8.0] — 2026-06-06

### Adicionado — preparação para autonomização (Modelo C)

**Configuração de ambiente:**
- `NORMORDIS_SPEC_PATH` — variável de ambiente que sobrepõe o caminho relativo
  hardcoded em `spec-conformance/src/lib.rs`. Permite referenciar a spec de
  qualquer localização (submodule, clone CI, path arbitrário).

**Infraestrutura do repositório:**
- `.gitattributes` — garante `eol=lf` em ficheiros `.json`, `.md`, `.yml` e `.sh`
  em todos os ambientes (Windows + Linux). Previne divergências de hash e erros
  de schema entre plataformas.
- `version.json` — versão legível por máquina (`spec_id`, `version`, `stability`,
  `schema_dialect`, `base_uri`). Substituição da necessidade de fazer parse de
  `VERSION.md` em tooling externo.

**CI leve da spec:**
- `ci/spec-ci.yml` — template GitHub Actions para validação autónoma da spec:
  (1) sintaxe JSON, (2) compilação de schemas Draft 2020-12, (3) consistência
  de `$id` com caminho, (4) consistência de versão entre `version.json` e
  `VERSION.md`, (5) fixtures válidas. Não depende de nenhuma implementação Rust ou Go.

**Documentação de extracção:**
- `EXTRACTING.md` — guia passo-a-passo para extrair `normordis-spec/` para repo
  independente via `git filter-repo`, preservando o histórico completo. Inclui
  opções de pós-extracção (submodule vs. NORMORDIS_SPEC_PATH) e configuração do
  novo repo (branch protection, secrets, CI).

**GOVERNANCE.md:**
- Nova secção "Autonomização" documenta o Modelo C adoptado, como cada
  implementação referencia a spec (submodule vs. clone CI), o que a spec
  valida autonomamente vs. o que é responsabilidade de cada runner, e o
  modelo de versionamento independente.

---

## [0.7.0] — 2026-06-06

### Adicionado

**Camada 4 — Scenario fixtures (invariantes inter-registo):**
- `fixtures/scenarios/audit-chain-valid.json` — 3 elos correctamente encadeados
- `fixtures/scenarios/audit-chain-broken-sequence.json` — violação de CHAIN-R01 (sequence repetida)
- `fixtures/scenarios/audit-chain-broken-hash.json` — violação de CHAIN-R02 (previous_hash inconsistente)
- Testes `valid_scenario_chains_pass_all_invariants` e `invalid_scenario_chains_fail_at_least_one_invariant`
- Teste `all_scenario_fixtures_are_mapped` (coverage gate para scenarios)

**Fixtures de regras sem cobertura:**
- `fixtures/invalid/support-storage-key-too-long.json` — viola STOR-R03 (maxLength: 256)
- `fixtures/invalid/support-crypto-argon2id-low-memory.json` — viola CRYPT-R03 (memory_kib < 64)

**IDs de regra nos ficheiros rules/:**
- `core-org.md` — 13 regras com IDs `ORG-R01`…`ORG-R13`
- `core-rh.md` — 8 regras com IDs `RH-R01`…`RH-R08`
- `core-config.md` — 10 regras com IDs `CFG-R01`…`CFG-R10`
- `core-security.md` — 9 regras com IDs `SEC-R01`…`SEC-R09`
- `core-validation.md` — 10 regras com IDs `VAL-R01`…`VAL-R10`
- `core-ingest.md` — 8 regras com IDs `INGEST-R01`…`INGEST-R08`
- `core-exports.md` — 11 regras com IDs `EXP-R01`…`EXP-R11`

**Estrutura de suporte:**
- `conformance/README.md` — guia completo de conformance (Rust + Go); tabelas de validação nativa; exemplos de código Go; secção de scenario fixtures e resolução de $ref
- `fixtures/migration/README.md` — protocolo para fixtures de breaking change MAJOR

**Teste de resolução de $ref:**
- `all_local_schema_refs_resolve` — verifica que todos os `$ref` com URI `normordis.local` têm schema registado com `$id` correspondente

### Corrigido

- `PersonAssignment` na camada 3: substituída verificação manual de campos por `.validate().is_err()`, alinhada com os outros tipos

---

## [0.6.0] — 2026-06-06

### Adicionado

**Guarda de drift (round-trip):**
- Teste `round_trip_rust_to_json_validates_schema` — serializa cada tipo Rust e valida o JSON resultante contra o schema. Detecta campos que o tipo serializa mas o schema não cobre.

**core-rh — UserProfile e refs inter-domínio:**
- `schemas/core/rh/user-profile.schema.json` — usa `$ref` para role + org-unit-ref
- `schemas/core/rh/org-unit-ref.schema.json`
- `schemas/core/rh/org-position-ref.schema.json`

**core-validation — building blocks:**
- `schemas/core/validation/validation-report.schema.json`
- `schemas/core/validation/validation-issue.schema.json`
- `schemas/core/validation/validation-severity.schema.json`

**core-security:**
- `schemas/core/security/auth-level.schema.json`
- `schemas/core/security/resource-classification.schema.json`
- `schemas/core/security/sod-rule.schema.json`
- `schemas/core/security/security-context.schema.json`

**core-exports:**
- `schemas/core/exports/tabular-dataset.schema.json`
- `schemas/core/exports/export-materialization-request.schema.json`

**Fixtures maximais (todos os campos opcionais preenchidos):**
- `audit-event-maximal.json`, `org-unit-maximal.json`, `control-definition-maximal.json`

### Corrigido (drift detectado pelo round-trip)

- `audit-actor`: `actor_name`/`actor_type` aceitam `null` (Option sem skip_serializing_if)
- `audit-chain-link`: `previous_record_hash` genesis serializa `null`
- `org-unit`: 9 campos opcionais aceitam `null`; invariante level=1→sem pai movida para camada 3
- `validation-result`: `context` e `outcomes[].message` aceitam `null`
- vários schemas org/rh: `valid_until` via `anyOf: [NaiveDate, null]`

### `$comment` semânticos inter-domínio

- Campos de ID em delegation, competency, person-assignment, security-context e refs RH documentam que tipo referenciam noutro domínio.

### Diferido

- `ExportSnapshot`: não esquematizado — `validate_export_snapshot()` recomputa o hash do manifesto, tornando impraticável uma fixture hand-crafted, e depende de `DocumentPackage` (core-documental). Coberto pelos tipos derivados (TabularDataset, ExportMaterializationRequest).

---

## [0.5.0] — 2026-06-06

### Adicionado

**core-org — cobertura completa:**
- `schemas/core/org/org-position.schema.json` — OrgPosition com PositionKind discriminado
- `schemas/core/org/competency.schema.json` — Competency com datas NaiveDate
- `schemas/core/org/delegation.schema.json` — Delegation com `allOf` que rejeita `from_position == to_position`
- `schemas/core/org/legal-instrument.schema.json` — LegalInstrument com InstrumentKind
- Fixtures válidas e inválidas para cada tipo

**Schemas de request (write-side):**
- `schemas/core/audit/record-audit-event-request.schema.json`
- `schemas/core/rh/user-identity.schema.json`
- `schemas/core/rh/person-assignment.schema.json`

**Camada 3 — fronteira explícita:**
- `fixtures/invalid/org-unit-inverted-dates.json` — valid_until < valid_from; passa schema, falha OrgUnit::validate()
- `fixtures/invalid/delegation-self-delegation.json` — from_position == to_position; passa schema, falha Delegation::validate()
- `fixtures/invalid/rh-person-assignment-inverted-dates.json` — valid_until < valid_from; passa schema, falha native validate

**Processo:**
- VERSION.md com tabela de historial
- CHANGELOG.md (este ficheiro)
- COVERAGE.md com granularidade de tipo e tabela de camadas

---

## [0.4.0] — 2026-06-06

### Adicionado

- `schemas/core/audit/audit-chain-link.schema.json` — com `if/then/else` para invariante génese
- `schemas/core/audit/audit-chain-report.schema.json`
- `schemas/core/audit/audit-export-manifest.schema.json`
- `schemas/core/audit/audit-manifest-signature.schema.json` — algorithm `enum: ["Ed25519"]`
- `schemas/core/audit/signed-audit-export-manifest.schema.json` — usa `$ref` para os dois anteriores
- Fixtures válidas e inválidas para cada tipo de chain
- `fixtures/invalid/control-definition-inverted-dates.json` — primeiro fixture de camada 3
- Teste `layer3_invalid_fixtures_pass_schema_but_fail_native` no runner Rust
- `LAYER3_INVALID_FIXTURES` — tabela separada no runner

---

## [0.3.0] — 2026-06-06

### Alterado (sem breaking para consumidores — nenhum existia)

- Reorganização `schemas/audit/` → `schemas/core/audit/`, `schemas/config/` → `schemas/core/config/`, etc.
- `$id` URIs actualizados em todos os 14 schemas core

### Adicionado

- `schemas/support/public-error.schema.json` — PublicError (superfície pública de erro)
- `$defs.Component` em `mini-error.schema.json` — reutilizável via `$ref`
- `$ref` de `log-event.schema.json` para `mini-error.schema.json#/$defs/ErrorCode`
- `encrypted-payload.schema.json`: `algorithm` como enum, `kdf.algorithm` como enum, condicional argon2id
- `render-request.schema.json`: `if/then/else` — `variables` exclusivo de `kind=typst_text`
- `rules/support.md` — 25 regras por crate

---

## [0.2.0] — 2026-06-06

### Adicionado

- 12 schemas em `schemas/support/`: mini-error, technical-id, utc-timestamp, postal-code,
  release-notes, normalization-case, storage-key, encrypted-payload, log-event,
  webauthn-challenge, render-request, backup-archive-ref
- Fixtures válidas e inválidas para cada schema support
- `conformance/README.md` expandido com guia Go e tabelas de validação nativa
- `GOVERNANCE.md` com processo completo de evolução da spec
- `COVERAGE.md` com estado por crate
- `rules/` para todos os domínios e support

---

## [0.1.0] — 2026-06-06

### Adicionado

- Estrutura inicial `normordis-spec/`
- 7 schemas `schemas/core/audit/`: audit-event, actor, target, outcome, control-category,
  control-definition, control-execution
- Fixtures válidas e inválidas para core-audit
- `rules/core-audit.md`, `rules/core-org.md` (stub), `rules/core-rh.md` (stub)
- `conformance/README.md` inicial
- `README.md`, `VERSION.md`, `GOVERNANCE.md` iniciais
- Crate `spec-conformance` com runner de 3 camadas e coverage gate

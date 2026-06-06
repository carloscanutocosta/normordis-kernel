# Matriz de Cobertura

Estado dos contratos executáveis em `normordis-spec`. Versão `0.8.0`.

> A fonte de verdade do mapeamento fixture→schema é o runner
> `crates/spec-conformance/tests/contract_conformance.rs`. Esta matriz resume o estado.

## core/*

| Crate | Tipos cobertos | Tipos em falta | Estado |
|-------|---------------|----------------|--------|
| `core-audit` | `AuditEvent`, `AuditActor`, `AuditTarget`, `AuditOutcome`, `ControlDefinition`, `ControlExecution`, `ControlCategory`, `ControlSeverity`, `AuditChainLink`, `AuditChainReport`, `AuditExportManifest`, `AuditManifestSignature`, `SignedAuditExportManifest`, `RecordAuditEventRequest` | `AuditChainState`, `AuditChainIndex`, `ConformanceSummary` | **Completo forte** |
| `core-config` | `AppConfig` (+ sub-profiles) | `StorageBackend`, `StoragePurpose` (enums) | Parcial forte |
| `core-org` | `OrgUnit`, `OrgPosition`, `Competency`, `Delegation`, `LegalInstrument`, `OrgContacts`, `OrgAddress` | `OrgLevel`, `OrgUnitStatus`, `PositionKind` (cobertos inline) | **Completo forte** |
| `core-rh` | `Role`, `UserProfile`, `UserIdentity`, `PersonAssignment`, `OrgUnitRef`, `OrgPositionRef`, `UserRole` | `CurrentSession`, `UserContext` | **Completo forte** |
| `core-security` | `Policy`, `Rule`, `PolicyMode`, `AuthLevel`, `ResourceClassification`, `SodRule`, `SecurityContext` | `OrgScope`, `SessionRef` (transparentes, cobertos inline) | **Completo forte** |
| `core-validation` | `ValidationResult`, `RuleOutcome`, `ValidationStatus`, `ValidationContext`, `ValidationReport`, `ValidationIssue`, `ValidationSeverity` | `Normalized<T>` (genérico, não esquematizável) | **Completo** |
| `core-ingest` | `IngestSource` | `IngestBundle`, `IngestRequest`, `IngestEvidence`, `IngestDecision` | Parcial |
| `core-exports` | `SourceRef`, `TabularDataset`, `ExportMaterializationRequest` | `ExportSnapshot` (ver nota), `Manifest`, `ExportArtefact`, `ExportMaterializationResult`, `ExportFormat`, `InteroperabilityProfile` | Parcial forte |
| `core-documental` | — | Sem contrato nesta versão | Pendente |
| `core-metrics` | — | Sem contrato nesta versão | Pendente |

> **Nota ExportSnapshot:** não esquematizado porque `validate_export_snapshot()`
> recomputa o hash do manifesto (uma fixture hand-crafted falharia sempre a camada 3)
> e depende de `DocumentPackage` de `core-documental`. Será coberto quando core-documental
> entrar na spec.

## support/*

| Crate | Tipos cobertos | Estado |
|-------|---------------|--------|
| `support-address` | `PostalCode` | Parcial |
| `support-auth` | `WebAuthnChallenge` | Parcial |
| `support-backup` | `BackupArchiveRef` | Parcial |
| `support-clock` | `UtcTimestamp` (primitiva via `$ref`) | Completo |
| `support-crypto` | `EncryptedPayload` | Parcial forte |
| `support-docx-to-typst` | `RenderRequest` (kind=docx_to_typst) | Parcial |
| `support-errors` | `MiniError`, `ErrorCode`, `Component`, `PublicError` | **Completo** |
| `support-ids` | `TechnicalId` | Completo |
| `support-logging` | `LogEvent` | Completo |
| `support-normalization` | `NormalizationCase` | Completo |
| `support-pdf` | `RenderRequest` (kind=pdf) | Parcial |
| `support-storage` | `StorageKey` | Completo |
| `support-typst-template` | `RenderRequest` (kind=typst_text) | Parcial |
| `support-versioning` | `ReleaseNotes` | Completo |

## infra/* e runtime/*

| Área | Estado |
|------|--------|
| `infra/*` | Pendente — adapters são implementação, não interoperabilidade |
| `runtime/*` | Pendente — sem contrato nesta versão |

---

## Legenda

| Estado | Critério |
|--------|----------|
| **Completo** | Todos os tipos públicos serializáveis têm schema, fixtures e conformance |
| **Completo forte** | Tipos centrais completos; em falta apenas auxiliares ou requests |
| **Parcial forte** | Cobre mais de um tipo central com fixtures e conformance |
| **Parcial** | Cobre pelo menos um tipo central com fixtures e conformance |
| **Pendente** | Sem schema executável nesta versão |

## Cobertura por camada de validação

| Camada | Descrição | Estado |
|--------|-----------|--------|
| **Camada 1** | JSON Schema (forma, tipo, patterns) | Todos os tipos cobertos |
| **Camada 2** | Desserialização nativa (serde) | Todos os tipos com Deserialize estável |
| **Camada 3** | Validação nativa (invariantes de negócio) | core-audit, core-config, core-org, core-rh, core-security (Policy), core-exports (TabularDataset, ExportMaterializationRequest) |
| **Camada 3 — fronteira explícita** | Fixtures que passam schema mas falham camada 3 | `control-definition-inverted-dates`, `org-unit-inverted-dates`, `org-delegation-self`, `rh-person-assignment-inverted-dates` |
| **Round-trip (guarda de drift)** | Rust → JSON → schema | Todos os tipos com Serialize + Deserialize |
| **Camada 4 — Scenarios** | Invariantes inter-registo (CHAIN-R01, CHAIN-R02) | `fixtures/scenarios/audit-chain-{valid,broken-sequence,broken-hash}.json` |
| **Resolução de $ref** | Todos os $ref locais têm schema com $id correspondente | Verificado por `all_local_schema_refs_resolve` |

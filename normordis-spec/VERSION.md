# Versão da Spec

Versão actual: `0.9.0`

## Semântica de versionamento

- `MAJOR`: quebra de compatibilidade no contrato interoperável.
- `MINOR`: novo domínio, novo schema, novo campo opcional ou nova fixture compatível.
- `PATCH`: correcção editorial, clarificação ou fixture que não muda o contrato.

## Compatibilidade

Uma implementação é conforme a uma versão quando:

- valida todos os schemas dessa versão;
- aceita todos os fixtures válidos dessa versão;
- rejeita todos os fixtures inválidos dessa versão;
- aplica as regras Markdown relevantes que não cabem em JSON Schema;
- passa o runner de conformance associado.

## Historial resumido

| Versão | Tipo | Resumo |
|--------|------|--------|
| `0.1.0` | Fundação | Estrutura base; core-audit (AuditEvent, ControlDefinition, ControlExecution) |
| `0.2.0` | MINOR | 14 schemas support (errors, ids, clock, address, versioning, normalization, storage, crypto, logging, auth, pdf, typst, docx, backup) |
| `0.3.0` | MINOR | Reorganização `schemas/core/*`; PublicError + Component `$defs`; log-event `$ref` |
| `0.4.0` | MINOR | core-audit completo: 5 schemas de chain (AuditChainLink, Report, ExportManifest, Signature, Signed); mecanismo de camada 3 |
| `0.5.0` | MINOR | core-org completo (OrgPosition, Competency, Delegation, LegalInstrument); schemas de request (RecordAuditEventRequest, UserIdentity, PersonAssignment); layer-3 fixtures org/rh |
| `0.6.0` | MINOR | Round-trip Rust→JSON→schema (guarda de drift); UserProfile + OrgUnitRef/OrgPositionRef com refs inter-domínio; ValidationReport/Issue/Severity; core-security (AuthLevel, ResourceClassification, SodRule, SecurityContext); core-exports (TabularDataset, ExportMaterializationRequest); fixtures maximais |
| `0.7.0` | MINOR | Camada 4 scenario fixtures (CHAIN-R01/R02); IDs de regra em todos os ficheiros rules/; fixtures STOR-R03 + CRYPT-R03; PersonAssignment camada 3 normalizado; teste de resolução de $ref; conformance/README.md + fixtures/migration/ |
| `0.8.0` | MINOR | Preparação para autonomização: `NORMORDIS_SPEC_PATH`, `.gitattributes`, `version.json`, secção de autonomização em GOVERNANCE.md, CI leve (`ci/spec-ci.yml`), guia de extracção (`EXTRACTING.md`) |
| `0.9.0` | MINOR | core-ingest completo: `IngestBundle` (raw base64), `IngestDecision`, `IngestEvidence` + sub-schemas (HashEvidence, ScanEvidence, ValidationEvidence, AuditEvidence); regras INGEST-R09..R13; base64 serde em `IngestBundle.raw` |

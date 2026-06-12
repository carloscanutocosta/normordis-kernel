mod support;
use support::{
    all_fixture_paths, assert_native_invalid, assert_native_valid, assert_schema_invalid,
    assert_schema_valid, roundtrip_json, schema_path_for, unresolved_schema_refs,
    validate_scenario, validator_for, ContractSchema, ScenarioKind,
};
use serde_json::Value;
use spec_conformance::spec_root;

// ── Fixtures válidas — por domínio ────────────────────────────────────────────

const VALID_FIXTURES: &[(&str, ContractSchema)] = &[
    // ── core-audit ────────────────────────────────────────────────────────────
    ("fixtures/valid/audit-event-minimal.json", ContractSchema::AuditEvent),
    ("fixtures/valid/audit-event-with-control.json", ContractSchema::AuditEvent),
    ("fixtures/valid/audit-event-maximal.json", ContractSchema::AuditEvent),
    ("fixtures/valid/control-definition-auth.json", ContractSchema::ControlDefinition),
    ("fixtures/valid/control-definition-maximal.json", ContractSchema::ControlDefinition),
    ("fixtures/valid/control-execution-passed.json", ContractSchema::ControlExecution),
    ("fixtures/valid/control-execution-dispensed.json", ContractSchema::ControlExecution),
    ("fixtures/valid/audit-chain-link-genesis.json", ContractSchema::AuditChainLink),
    ("fixtures/valid/audit-chain-link-chained.json", ContractSchema::AuditChainLink),
    ("fixtures/valid/audit-chain-report-verified.json", ContractSchema::AuditChainReport),
    ("fixtures/valid/audit-export-manifest.json", ContractSchema::AuditExportManifest),
    ("fixtures/valid/signed-audit-export-manifest.json", ContractSchema::SignedAuditExportManifest),
    ("fixtures/valid/audit-record-event-request.json", ContractSchema::RecordAuditEventRequest),
    // ── core-config ───────────────────────────────────────────────────────────
    ("fixtures/valid/config-app-config-basic.json", ContractSchema::ConfigAppConfig),
    // ── core-org ──────────────────────────────────────────────────────────────
    ("fixtures/valid/org-unit-root.json", ContractSchema::OrgUnit),
    ("fixtures/valid/org-unit-maximal.json", ContractSchema::OrgUnit),
    ("fixtures/valid/org-position-active.json", ContractSchema::OrgPosition),
    ("fixtures/valid/org-position-outro.json", ContractSchema::OrgPosition),
    ("fixtures/valid/org-competency.json", ContractSchema::Competency),
    ("fixtures/valid/org-delegation.json", ContractSchema::Delegation),
    ("fixtures/valid/org-legal-instrument.json", ContractSchema::LegalInstrument),
    // ── core-rh ───────────────────────────────────────────────────────────────
    ("fixtures/valid/rh-role-active.json", ContractSchema::RhRole),
    ("fixtures/valid/rh-user-identity.json", ContractSchema::UserIdentity),
    ("fixtures/valid/rh-person-assignment.json", ContractSchema::PersonAssignment),
    ("fixtures/valid/rh-user-profile-full.json", ContractSchema::UserProfile),
    ("fixtures/valid/rh-user-profile-minimal.json", ContractSchema::UserProfile),
    // ── core-security ─────────────────────────────────────────────────────────
    ("fixtures/valid/security-policy-strict.json", ContractSchema::SecurityPolicy),
    ("fixtures/valid/security-auth-level.json", ContractSchema::SecurityAuthLevel),
    ("fixtures/valid/security-classification.json", ContractSchema::SecurityClassification),
    ("fixtures/valid/security-sod-rule.json", ContractSchema::SecuritySodRule),
    ("fixtures/valid/security-context-full.json", ContractSchema::SecurityContext),
    ("fixtures/valid/security-context-minimal.json", ContractSchema::SecurityContext),
    // ── core-validation ───────────────────────────────────────────────────────
    ("fixtures/valid/validation-result-passed.json", ContractSchema::ValidationResult),
    ("fixtures/valid/validation-report-clean.json", ContractSchema::ValidationReport),
    ("fixtures/valid/validation-report-with-issues.json", ContractSchema::ValidationReport),
    // ── core-ingest ───────────────────────────────────────────────────────────
    ("fixtures/valid/ingest-source-config-bundle.json", ContractSchema::IngestSource),
    ("fixtures/valid/ingest-bundle-pdf.json", ContractSchema::IngestBundle),
    ("fixtures/valid/ingest-bundle-saft-pt.json", ContractSchema::IngestBundle),
    ("fixtures/valid/ingest-bundle-xml.json", ContractSchema::IngestBundle),
    ("fixtures/valid/ingest-decision-accepted.json", ContractSchema::IngestDecision),
    ("fixtures/valid/ingest-decision-rejected.json", ContractSchema::IngestDecision),
    ("fixtures/valid/ingest-evidence-accepted.json", ContractSchema::IngestEvidence),
    ("fixtures/valid/ingest-evidence-rejected.json", ContractSchema::IngestEvidence),
    // ── core-exports ──────────────────────────────────────────────────────────
    ("fixtures/valid/exports-source-ref-config-profile.json", ContractSchema::ExportsSourceRef),
    ("fixtures/valid/exports-tabular-dataset.json", ContractSchema::ExportsTabularDataset),
    ("fixtures/valid/exports-materialization-request.json", ContractSchema::ExportsMaterializationRequest),
    // ── support ───────────────────────────────────────────────────────────────
    ("fixtures/valid/support-mini-error.json", ContractSchema::SupportMiniError),
    ("fixtures/valid/support-public-error.json", ContractSchema::SupportPublicError),
    ("fixtures/valid/support-technical-id.json", ContractSchema::SupportTechnicalId),
    ("fixtures/valid/support-utc-timestamp.json", ContractSchema::SupportUtcTimestamp),
    ("fixtures/valid/support-postal-code.json", ContractSchema::SupportPostalCode),
    ("fixtures/valid/support-release-notes.json", ContractSchema::SupportReleaseNotes),
    ("fixtures/valid/support-normalization-case.json", ContractSchema::SupportNormalizationCase),
    ("fixtures/valid/support-storage-key.json", ContractSchema::SupportStorageKey),
    ("fixtures/valid/support-encrypted-payload.json", ContractSchema::SupportEncryptedPayload),
    ("fixtures/valid/support-log-event.json", ContractSchema::SupportLogEvent),
    ("fixtures/valid/support-webauthn-challenge.json", ContractSchema::SupportWebAuthnChallenge),
    ("fixtures/valid/support-pdf-render-request.json", ContractSchema::SupportRenderRequest),
    ("fixtures/valid/support-typst-render-request.json", ContractSchema::SupportRenderRequest),
    ("fixtures/valid/support-docx-to-typst-request.json", ContractSchema::SupportRenderRequest),
    ("fixtures/valid/support-backup-archive-ref.json", ContractSchema::SupportBackupArchiveRef),
];

// ── Fixtures inválidas — por domínio ──────────────────────────────────────────

const INVALID_FIXTURES: &[(&str, ContractSchema)] = &[
    // ── core-audit ────────────────────────────────────────────────────────────
    ("fixtures/invalid/audit-event-missing-actor.json", ContractSchema::AuditEvent),
    ("fixtures/invalid/audit-event-missing-occurred-at.json", ContractSchema::AuditEvent),
    ("fixtures/invalid/audit-event-non-utc-offset.json", ContractSchema::AuditEvent),
    ("fixtures/invalid/audit-event-blank-actor-id.json", ContractSchema::AuditEvent),
    ("fixtures/invalid/control-definition-legacy-control-id.json", ContractSchema::ControlDefinition),
    ("fixtures/invalid/control-execution-invalid-result.json", ContractSchema::ControlExecution),
    ("fixtures/invalid/control-execution-dispensed-missing-notes.json", ContractSchema::ControlExecution),
    ("fixtures/invalid/audit-chain-link-missing-record-hash.json", ContractSchema::AuditChainLink),
    ("fixtures/invalid/audit-chain-link-genesis-with-previous.json", ContractSchema::AuditChainLink),
    ("fixtures/invalid/audit-export-manifest-invalid-hash.json", ContractSchema::AuditExportManifest),
    ("fixtures/invalid/signed-audit-export-manifest-missing-signature.json", ContractSchema::SignedAuditExportManifest),
    // ── core-config ───────────────────────────────────────────────────────────
    ("fixtures/invalid/config-app-config-path-traversal.json", ContractSchema::ConfigAppConfig),
    // ── core-org ──────────────────────────────────────────────────────────────
    ("fixtures/invalid/org-unit-child-without-parent.json", ContractSchema::OrgUnit),
    ("fixtures/invalid/org-position-missing-title.json", ContractSchema::OrgPosition),
    ("fixtures/invalid/org-delegation-missing-instrument.json", ContractSchema::Delegation),
    ("fixtures/invalid/org-legal-instrument-invalid-kind.json", ContractSchema::LegalInstrument),
    // ── core-rh ───────────────────────────────────────────────────────────────
    ("fixtures/invalid/rh-role-id-with-space.json", ContractSchema::RhRole),
    ("fixtures/invalid/rh-user-identity-invalid-role.json", ContractSchema::UserIdentity),
    ("fixtures/invalid/rh-person-assignment-missing-basis.json", ContractSchema::PersonAssignment),
    ("fixtures/invalid/rh-user-profile-invalid-role.json", ContractSchema::UserProfile),
    // ── core-security ─────────────────────────────────────────────────────────
    ("fixtures/invalid/security-policy-empty-rules.json", ContractSchema::SecurityPolicy),
    ("fixtures/invalid/security-auth-level-unknown.json", ContractSchema::SecurityAuthLevel),
    ("fixtures/invalid/security-classification-unknown.json", ContractSchema::SecurityClassification),
    ("fixtures/invalid/security-sod-rule-missing-blocked.json", ContractSchema::SecuritySodRule),
    ("fixtures/invalid/security-context-missing-auth-level.json", ContractSchema::SecurityContext),
    // ── core-validation ───────────────────────────────────────────────────────
    ("fixtures/invalid/validation-result-empty-target.json", ContractSchema::ValidationResult),
    ("fixtures/invalid/validation-report-missing-valid.json", ContractSchema::ValidationReport),
    // ── core-ingest ───────────────────────────────────────────────────────────
    ("fixtures/invalid/ingest-source-missing-kind.json", ContractSchema::IngestSource),
    ("fixtures/invalid/ingest-bundle-missing-raw.json", ContractSchema::IngestBundle),
    ("fixtures/invalid/ingest-bundle-empty-content-type.json", ContractSchema::IngestBundle),
    ("fixtures/invalid/ingest-bundle-invalid-hash-prefix.json", ContractSchema::IngestBundle),
    ("fixtures/invalid/ingest-decision-unknown.json", ContractSchema::IngestDecision),
    ("fixtures/invalid/ingest-evidence-missing-bundle-id.json", ContractSchema::IngestEvidence),
    // ── core-exports ──────────────────────────────────────────────────────────
    ("fixtures/invalid/exports-source-ref-blank-subject.json", ContractSchema::ExportsSourceRef),
    ("fixtures/invalid/exports-tabular-dataset-row-not-object.json", ContractSchema::ExportsTabularDataset),
    ("fixtures/invalid/exports-materialization-request-invalid-format.json", ContractSchema::ExportsMaterializationRequest),
    // ── support ───────────────────────────────────────────────────────────────
    ("fixtures/invalid/support-mini-error-bad-code.json", ContractSchema::SupportMiniError),
    ("fixtures/invalid/support-public-error-bad-code.json", ContractSchema::SupportPublicError),
    ("fixtures/invalid/support-technical-id-not-uuid.json", ContractSchema::SupportTechnicalId),
    ("fixtures/invalid/support-utc-timestamp-offset.json", ContractSchema::SupportUtcTimestamp),
    ("fixtures/invalid/support-postal-code-bad-cp3.json", ContractSchema::SupportPostalCode),
    ("fixtures/invalid/support-release-notes-empty-version.json", ContractSchema::SupportReleaseNotes),
    ("fixtures/invalid/support-normalization-case-unknown-op.json", ContractSchema::SupportNormalizationCase),
    ("fixtures/invalid/support-storage-key-path-token.json", ContractSchema::SupportStorageKey),
    ("fixtures/invalid/support-storage-key-too-long.json", ContractSchema::SupportStorageKey),
    ("fixtures/invalid/support-encrypted-payload-missing-ciphertext.json", ContractSchema::SupportEncryptedPayload),
    ("fixtures/invalid/support-crypto-argon2id-low-memory.json", ContractSchema::SupportEncryptedPayload),
    ("fixtures/invalid/support-log-event-bad-level.json", ContractSchema::SupportLogEvent),
    ("fixtures/invalid/support-webauthn-challenge-missing-user.json", ContractSchema::SupportWebAuthnChallenge),
    ("fixtures/invalid/support-pdf-render-request-empty-source.json", ContractSchema::SupportRenderRequest),
    ("fixtures/invalid/support-typst-render-request-bad-kind.json", ContractSchema::SupportRenderRequest),
    ("fixtures/invalid/support-docx-to-typst-request-empty-source.json", ContractSchema::SupportRenderRequest),
    ("fixtures/invalid/support-backup-archive-ref-bad-hash.json", ContractSchema::SupportBackupArchiveRef),
];

// Passam JSON Schema (camada 1) mas falham validação nativa (camada 3).
const LAYER3_INVALID_FIXTURES: &[(&str, ContractSchema)] = &[
    ("fixtures/invalid/control-definition-inverted-dates.json", ContractSchema::ControlDefinition),
    ("fixtures/invalid/org-unit-inverted-dates.json", ContractSchema::OrgUnit),
    ("fixtures/invalid/org-delegation-self.json", ContractSchema::Delegation),
    ("fixtures/invalid/rh-person-assignment-inverted-dates.json", ContractSchema::PersonAssignment),
    (
        "fixtures/invalid/ingest-evidence-accepted-missing-document-ref.json",
        ContractSchema::IngestEvidence,
    ),
    (
        "fixtures/invalid/ingest-evidence-inverted-timestamps.json",
        ContractSchema::IngestEvidence,
    ),
    (
        "fixtures/invalid/ingest-evidence-verified-hash-mismatch.json",
        ContractSchema::IngestEvidence,
    ),
];

// ── Scenario fixtures — por invariante ───────────────────────────────────────

const VALID_SCENARIO_FIXTURES: &[(&str, ScenarioKind)] = &[
    ("fixtures/scenarios/audit-chain-valid.json", ScenarioKind::AuditChain),
    ("fixtures/scenarios/ctrl-execution-active-control.json", ScenarioKind::CtrlExecution),
    ("fixtures/scenarios/org-delegation-active-position.json", ScenarioKind::OrgDelegation),
    ("fixtures/scenarios/rh-assignment-no-overlap.json", ScenarioKind::RhAssignmentOverlap),
];

const INVALID_SCENARIO_FIXTURES: &[(&str, ScenarioKind)] = &[
    ("fixtures/scenarios/audit-chain-broken-hash.json", ScenarioKind::AuditChain),
    ("fixtures/scenarios/audit-chain-broken-sequence.json", ScenarioKind::AuditChain),
    ("fixtures/scenarios/ctrl-execution-inactive-control.json", ScenarioKind::CtrlExecution),
    ("fixtures/scenarios/org-delegation-extinct-position.json", ScenarioKind::OrgDelegation),
    ("fixtures/scenarios/rh-assignment-overlap.json", ScenarioKind::RhAssignmentOverlap),
];

// ── Cobertura: nenhum fixture fica fora do runner ─────────────────────────────

#[test]
fn all_valid_contract_fixtures_are_mapped() {
    let actual = all_fixture_paths("fixtures/valid");
    let mut expected = VALID_FIXTURES
        .iter()
        .map(|(p, _)| (*p).to_string())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual, expected, "há fixtures válidas sem mapeamento de conformance");
}

#[test]
fn all_invalid_contract_fixtures_are_mapped() {
    let actual = all_fixture_paths("fixtures/invalid");
    let mut expected = INVALID_FIXTURES
        .iter()
        .chain(LAYER3_INVALID_FIXTURES.iter())
        .map(|(p, _)| (*p).to_string())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual, expected, "há fixtures inválidas sem mapeamento de conformance");
}

#[test]
fn all_scenario_fixtures_are_mapped() {
    let actual = all_fixture_paths("fixtures/scenarios");
    let mut expected = VALID_SCENARIO_FIXTURES
        .iter()
        .chain(INVALID_SCENARIO_FIXTURES.iter())
        .map(|(p, _)| (*p).to_string())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual, expected, "há scenario fixtures sem mapeamento de conformance");
}

// ── Grupo 1: fixtures válidas passam schema ───────────────────────────────────

#[test]
fn valid_fixtures_pass_schema() {
    for (path, schema) in VALID_FIXTURES {
        assert_schema_valid(path, *schema);
    }
}

// ── Grupo 2: fixtures inválidas falham schema ─────────────────────────────────

#[test]
fn invalid_fixtures_fail_schema() {
    for (path, schema) in INVALID_FIXTURES {
        assert_schema_invalid(path, *schema);
    }
}

// ── Grupo 0: round-trip Rust → JSON → schema (guarda de drift) ───────────────

#[test]
fn round_trip_rust_to_json_validates_schema() {
    for (path, schema) in VALID_FIXTURES {
        let roundtripped = roundtrip_json(path, *schema);
        let validator = validator_for(*schema);
        assert!(
            validator.is_valid(&roundtripped),
            "DRIFT em {path}:\n{errors}",
            errors = validator
                .iter_errors(&roundtripped)
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

// ── Grupo 3: fixtures válidas desserializam e validam no Rust ─────────────────

#[test]
fn valid_fixtures_deserialize_and_validate_in_rust() {
    for (path, schema) in VALID_FIXTURES {
        assert_native_valid(path, *schema);
    }
}

// ── Grupo 4: camada 3 — passam schema, falham validação nativa ────────────────

#[test]
fn layer3_invalid_fixtures_pass_schema_but_fail_native() {
    for (path, schema) in LAYER3_INVALID_FIXTURES {
        let validator = validator_for(*schema);
        let instance = {
            let p = spec_root().join(path);
            let content = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
            serde_json::from_str::<Value>(&content)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", p.display()))
        };
        assert!(
            validator.is_valid(&instance),
            "{path} deveria passar o schema (inválido só na camada 3) mas falhou:\n{errors}",
            errors = validator
                .iter_errors(&instance)
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        assert_native_invalid(path, *schema);
    }
}

// ── Camada 4: scenario fixtures ───────────────────────────────────────────────

#[test]
fn valid_scenarios_pass_all_invariants() {
    for (path, kind) in VALID_SCENARIO_FIXTURES {
        validate_scenario(path, *kind)
            .unwrap_or_else(|e| panic!("{path} deveria passar todas as invariantes:\n{e}"));
    }
}

#[test]
fn invalid_scenarios_fail_at_least_one_invariant() {
    for (path, kind) in INVALID_SCENARIO_FIXTURES {
        assert!(
            validate_scenario(path, *kind).is_err(),
            "{path} deveria falhar pelo menos uma invariante mas passou"
        );
    }
}

// ── Resolução de $ref ─────────────────────────────────────────────────────────

#[test]
fn all_local_schema_refs_resolve() {
    let unresolved = unresolved_schema_refs();
    assert!(
        unresolved.is_empty(),
        "Os seguintes $ref locais não têm schema correspondente:\n{}",
        unresolved.join("\n")
    );
}

// ── Ponto 3: todos os campos required têm cobertura de rejeição ───────────────

#[test]
fn required_fields_are_enforced_by_schema() {
    // Para cada schema distinto em VALID_FIXTURES, pega no primeiro fixture válido,
    // remove cada campo required um a um, e verifica que o schema rejeita o resultado.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut to_test: Vec<(ContractSchema, &str)> = Vec::new();

    for (fixture_path, schema) in VALID_FIXTURES {
        let sp = schema_path_for(*schema);
        if seen.insert(sp) {
            to_test.push((*schema, fixture_path));
        }
    }

    for (schema, fixture_path) in &to_test {
        let schema_json = {
            let p = spec_root().join(schema_path_for(*schema));
            let content = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("cannot read schema {}: {e}", p.display()));
            serde_json::from_str::<Value>(&content)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", p.display()))
        };

        let required: Vec<String> = schema_json
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if required.is_empty() {
            continue;
        }

        let base = {
            let p = spec_root().join(fixture_path);
            let content = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
            serde_json::from_str::<Value>(&content)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", p.display()))
        };

        let Value::Object(_) = &base else {
            // Schema de primitivo (ex: StorageKey é string) — sem campos required de objecto.
            continue;
        };

        let validator = validator_for(*schema);

        for field in &required {
            let mut modified = base.clone();
            modified
                .as_object_mut()
                .unwrap()
                .remove(field.as_str());
            assert!(
                !validator.is_valid(&modified),
                "Schema {:?}: campo required '{}' não é rejeitado quando omitido\n\
                 (fixture base: {fixture_path})",
                schema,
                field,
            );
        }
    }
}

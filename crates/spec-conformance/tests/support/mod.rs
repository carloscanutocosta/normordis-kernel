#![allow(dead_code, unused_imports)]

use core_audit::{
    AuditChainLink, AuditChainReport, AuditEvent, AuditExportManifest, ControlDefinition,
    ControlExecution, SignedAuditExportManifest,
};
use core_config::{validate_app_config, AppConfig};
use core_exports::{ExportMaterializationRequest, SourceRef, TabularDataset};
use core_ingest::{
    validate_ingest_bundle, validate_ingest_evidence, IngestBundle, IngestDecision, IngestEvidence,
    IngestSource,
};
use core_org::{Competency, Delegation, LegalInstrument, OrgPosition, OrgUnit};
use core_rh::{PersonAssignment, Role, UserIdentity, UserProfile};
use core_security::{
    validate_policy, AuthLevel, Policy, ResourceClassification, SecurityContext, SodRule,
};
use core_validation::{ValidationReport, ValidationResult};
use jsonschema::Validator;
use serde_json::Value;
use spec_conformance::spec_root;
use support_address::PostalCode;
use support_auth::WebAuthnChallenge;
use support_crypto::EncryptedPayload;
use support_errors::{MiniError, PublicError};
use support_ids::TechnicalId;
use support_logging::LogEvent;
use support_storage::StorageKey;
use support_versioning::ReleaseNotes;

// ── ContractSchema ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum ContractSchema {
    AuditEvent,
    ControlDefinition,
    ControlExecution,
    ConfigAppConfig,
    OrgUnit,
    RhRole,
    SecurityPolicy,
    ValidationResult,
    IngestSource,
    ExportsSourceRef,
    SupportMiniError,
    SupportTechnicalId,
    SupportUtcTimestamp,
    SupportPostalCode,
    SupportReleaseNotes,
    SupportNormalizationCase,
    SupportStorageKey,
    SupportEncryptedPayload,
    SupportLogEvent,
    SupportWebAuthnChallenge,
    SupportRenderRequest,
    SupportBackupArchiveRef,
    SupportPublicError,
    AuditChainLink,
    AuditChainReport,
    AuditExportManifest,
    SignedAuditExportManifest,
    OrgPosition,
    Competency,
    Delegation,
    LegalInstrument,
    RecordAuditEventRequest,
    UserIdentity,
    PersonAssignment,
    UserProfile,
    ValidationReport,
    SecurityAuthLevel,
    SecurityClassification,
    SecuritySodRule,
    SecurityContext,
    ExportsTabularDataset,
    ExportsMaterializationRequest,
    IngestBundle,
    IngestDecision,
    IngestEvidence,
}

// ── ScenarioKind ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum ScenarioKind {
    AuditChain,
    CtrlExecution,
    OrgDelegation,
    RhAssignmentOverlap,
}

// ── Helpers básicos ───────────────────────────────────────────────────────────

pub fn load(relative: &str) -> Value {
    let path = spec_root().join(relative);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}

pub fn all_fixture_paths(dir: &str) -> Vec<String> {
    let root = spec_root().join(dir);
    let mut paths = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", root.display()))
        .map(|entry| {
            let entry = entry.unwrap_or_else(|e| panic!("entry error in {}: {e}", root.display()));
            let name = entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| panic!("non UTF-8 name in {}", root.display()));
            format!("{dir}/{name}")
        })
        .filter(|p| p.ends_with(".json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

pub fn schema_path_for(schema: ContractSchema) -> &'static str {
    match schema {
        ContractSchema::AuditEvent => "schemas/core/audit/audit-event.schema.json",
        ContractSchema::ControlDefinition => "schemas/core/audit/control-definition.schema.json",
        ContractSchema::ControlExecution => "schemas/core/audit/control-execution.schema.json",
        ContractSchema::ConfigAppConfig => "schemas/core/config/app-config.schema.json",
        ContractSchema::OrgUnit => "schemas/core/org/org-unit.schema.json",
        ContractSchema::RhRole => "schemas/core/rh/role.schema.json",
        ContractSchema::SecurityPolicy => "schemas/core/security/policy.schema.json",
        ContractSchema::ValidationResult => "schemas/core/validation/validation-result.schema.json",
        ContractSchema::IngestSource => "schemas/core/ingest/source.schema.json",
        ContractSchema::ExportsSourceRef => "schemas/core/exports/source-ref.schema.json",
        ContractSchema::SupportMiniError => "schemas/support/mini-error.schema.json",
        ContractSchema::SupportTechnicalId => "schemas/support/technical-id.schema.json",
        ContractSchema::SupportUtcTimestamp => "schemas/support/utc-timestamp.schema.json",
        ContractSchema::SupportPostalCode => "schemas/support/postal-code.schema.json",
        ContractSchema::SupportReleaseNotes => "schemas/support/release-notes.schema.json",
        ContractSchema::SupportNormalizationCase => {
            "schemas/support/normalization-case.schema.json"
        }
        ContractSchema::SupportStorageKey => "schemas/support/storage-key.schema.json",
        ContractSchema::SupportEncryptedPayload => "schemas/support/encrypted-payload.schema.json",
        ContractSchema::SupportLogEvent => "schemas/support/log-event.schema.json",
        ContractSchema::SupportWebAuthnChallenge => {
            "schemas/support/webauthn-challenge.schema.json"
        }
        ContractSchema::SupportRenderRequest => "schemas/support/render-request.schema.json",
        ContractSchema::SupportBackupArchiveRef => "schemas/support/backup-archive-ref.schema.json",
        ContractSchema::SupportPublicError => "schemas/support/public-error.schema.json",
        ContractSchema::AuditChainLink => "schemas/core/audit/audit-chain-link.schema.json",
        ContractSchema::AuditChainReport => "schemas/core/audit/audit-chain-report.schema.json",
        ContractSchema::AuditExportManifest => {
            "schemas/core/audit/audit-export-manifest.schema.json"
        }
        ContractSchema::SignedAuditExportManifest => {
            "schemas/core/audit/signed-audit-export-manifest.schema.json"
        }
        ContractSchema::OrgPosition => "schemas/core/org/org-position.schema.json",
        ContractSchema::Competency => "schemas/core/org/competency.schema.json",
        ContractSchema::Delegation => "schemas/core/org/delegation.schema.json",
        ContractSchema::LegalInstrument => "schemas/core/org/legal-instrument.schema.json",
        ContractSchema::RecordAuditEventRequest => {
            "schemas/core/audit/record-audit-event-request.schema.json"
        }
        ContractSchema::UserIdentity => "schemas/core/rh/user-identity.schema.json",
        ContractSchema::PersonAssignment => "schemas/core/rh/person-assignment.schema.json",
        ContractSchema::UserProfile => "schemas/core/rh/user-profile.schema.json",
        ContractSchema::ValidationReport => "schemas/core/validation/validation-report.schema.json",
        ContractSchema::SecurityAuthLevel => "schemas/core/security/auth-level.schema.json",
        ContractSchema::SecurityClassification => {
            "schemas/core/security/resource-classification.schema.json"
        }
        ContractSchema::SecuritySodRule => "schemas/core/security/sod-rule.schema.json",
        ContractSchema::SecurityContext => "schemas/core/security/security-context.schema.json",
        ContractSchema::ExportsTabularDataset => "schemas/core/exports/tabular-dataset.schema.json",
        ContractSchema::ExportsMaterializationRequest => {
            "schemas/core/exports/export-materialization-request.schema.json"
        }
        ContractSchema::IngestBundle => "schemas/core/ingest/ingest-bundle.schema.json",
        ContractSchema::IngestDecision => "schemas/core/ingest/ingest-decision.schema.json",
        ContractSchema::IngestEvidence => "schemas/core/ingest/ingest-evidence.schema.json",
    }
}

// ── Schema registry (Ponto 4) ─────────────────────────────────────────────────
// Carrega todos os schemas do disco de uma vez; elimina listas manuais de supporting.

fn collect_all_schemas(dir: &std::path::Path) -> Vec<(String, Value)> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(dir).expect("schemas dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_all_schemas(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<Value>(&content) {
                    if let Some(id) = v.get("$id").and_then(|i| i.as_str()) {
                        result.push((id.to_string(), v));
                    }
                }
            }
        }
    }
    result
}

pub fn validator_for(schema: ContractSchema) -> Validator {
    let main_schema = load(schema_path_for(schema));
    let mut opts = jsonschema::options();
    for (id, s) in collect_all_schemas(&spec_root().join("schemas")) {
        if let Ok(resource) = jsonschema::Resource::from_contents(s) {
            opts.with_resource(id, resource);
        }
    }
    opts.build(&main_schema)
        .unwrap_or_else(|e| panic!("failed to compile {:?}: {e}", schema))
}

// ── Validação de schema (camadas 1) ───────────────────────────────────────────

pub fn assert_schema_valid(path: &str, schema: ContractSchema) {
    let validator = validator_for(schema);
    let instance = load(path);
    assert!(
        validator.is_valid(&instance),
        "{path} deveria ser válido mas falhou:\n{errors}",
        errors = validator
            .iter_errors(&instance)
            .map(|e| format!("  - {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

pub fn assert_schema_invalid(path: &str, schema: ContractSchema) {
    let validator = validator_for(schema);
    let instance = load(path);
    assert!(
        !validator.is_valid(&instance),
        "{path} deveria ser inválido mas passou o schema",
    );
}

// ── Validação nativa (camadas 2+3) ────────────────────────────────────────────

pub fn assert_native_valid(path: &str, schema: ContractSchema) {
    let instance = load(path);
    match schema {
        ContractSchema::AuditEvent => {
            let v = serde_json::from_value::<AuditEvent>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ControlDefinition => {
            let v = serde_json::from_value::<ControlDefinition>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ControlExecution => {
            let v = serde_json::from_value::<ControlExecution>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ConfigAppConfig => {
            let v = serde_json::from_value::<AppConfig>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            validate_app_config(&v).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::OrgUnit => {
            let v = serde_json::from_value::<OrgUnit>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::RhRole => {
            let v =
                serde_json::from_value::<Role>(instance).unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SecurityPolicy => {
            let v = serde_json::from_value::<Policy>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            validate_policy(&v).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ValidationResult => {
            serde_json::from_value::<ValidationResult>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::IngestSource => {
            let v = serde_json::from_value::<IngestSource>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!v.kind.trim().is_empty() && !v.subject_id.trim().is_empty());
        }
        ContractSchema::ExportsSourceRef => {
            let v = serde_json::from_value::<SourceRef>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!v.kind.trim().is_empty() && !v.subject_id.trim().is_empty());
        }
        ContractSchema::SupportMiniError => {
            serde_json::from_value::<MiniError>(instance).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportPublicError => {
            serde_json::from_value::<PublicError>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportTechnicalId => {
            let v = serde_json::from_value::<TechnicalId>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!v.as_str().trim().is_empty());
        }
        ContractSchema::SupportUtcTimestamp => {
            let v = instance
                .as_str()
                .unwrap_or_else(|| panic!("{path}: expected string"));
            assert!(v.ends_with('Z'), "{path}: deveria terminar em Z");
        }
        ContractSchema::SupportPostalCode => {
            let v = serde_json::from_value::<PostalCode>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            support_address::validate_postal_parts(&v.cp4, &v.cp3)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportReleaseNotes => {
            let v = serde_json::from_value::<ReleaseNotes>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportNormalizationCase => {
            let input = instance["input"]
                .as_str()
                .unwrap_or_else(|| panic!("{path}: missing input"));
            let expected = instance["expected"]
                .as_str()
                .unwrap_or_else(|| panic!("{path}: missing expected"));
            assert_eq!(support_normalization::normalize_for_lookup(input), expected);
        }
        ContractSchema::SupportStorageKey => {
            let v = instance
                .as_str()
                .unwrap_or_else(|| panic!("{path}: expected string"));
            StorageKey::new(v).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportEncryptedPayload => {
            serde_json::from_value::<EncryptedPayload>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportLogEvent => {
            serde_json::from_value::<LogEvent>(instance).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportWebAuthnChallenge => {
            let v = serde_json::from_value::<WebAuthnChallenge>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(!v.id.trim().is_empty() && !v.value.trim().is_empty());
        }
        ContractSchema::AuditChainLink => {
            serde_json::from_value::<AuditChainLink>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::AuditChainReport => {
            serde_json::from_value::<AuditChainReport>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::AuditExportManifest => {
            serde_json::from_value::<AuditExportManifest>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SignedAuditExportManifest => {
            serde_json::from_value::<SignedAuditExportManifest>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::OrgPosition => {
            let v = serde_json::from_value::<OrgPosition>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::Competency => {
            let v = serde_json::from_value::<Competency>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::Delegation => {
            let v = serde_json::from_value::<Delegation>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::LegalInstrument => {
            serde_json::from_value::<LegalInstrument>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::RecordAuditEventRequest => {
            // Schema-only contract; sem Deserialize estável como tipo público.
        }
        ContractSchema::UserIdentity => {
            let v = serde_json::from_value::<UserIdentity>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::PersonAssignment => {
            serde_json::from_value::<PersonAssignment>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::UserProfile => {
            let v = serde_json::from_value::<UserProfile>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ValidationReport => {
            serde_json::from_value::<ValidationReport>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SecurityAuthLevel => {
            serde_json::from_value::<AuthLevel>(instance).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SecurityClassification => {
            serde_json::from_value::<ResourceClassification>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SecuritySodRule => {
            serde_json::from_value::<SodRule>(instance).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SecurityContext => {
            serde_json::from_value::<SecurityContext>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ExportsTabularDataset => {
            let v = serde_json::from_value::<TabularDataset>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::ExportsMaterializationRequest => {
            let v = serde_json::from_value::<ExportMaterializationRequest>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            v.validate().unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::SupportRenderRequest | ContractSchema::SupportBackupArchiveRef => {
            // Contratos de interoperabilidade — validação estrutural via schema é suficiente.
        }
        ContractSchema::IngestBundle => {
            let v = serde_json::from_value::<IngestBundle>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            validate_ingest_bundle(&v).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::IngestDecision => {
            serde_json::from_value::<IngestDecision>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        ContractSchema::IngestEvidence => {
            let v = serde_json::from_value::<IngestEvidence>(instance)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            validate_ingest_evidence(&v).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
    }
}

// ── Round-trip: Rust → JSON → schema ─────────────────────────────────────────

pub fn roundtrip_json(path: &str, schema: ContractSchema) -> Value {
    macro_rules! rt {
        ($instance:expr, $T:ty) => {{
            let typed = serde_json::from_value::<$T>($instance)
                .unwrap_or_else(|e| panic!("round-trip: cannot deserialize {path}: {e}"));
            serde_json::to_value(&typed)
                .unwrap_or_else(|e| panic!("round-trip: cannot serialize {path}: {e}"))
        }};
    }
    let instance = load(path);
    match schema {
        ContractSchema::AuditEvent => rt!(instance, AuditEvent),
        ContractSchema::ControlDefinition => rt!(instance, ControlDefinition),
        ContractSchema::ControlExecution => rt!(instance, ControlExecution),
        ContractSchema::AuditChainLink => rt!(instance, AuditChainLink),
        ContractSchema::AuditChainReport => rt!(instance, AuditChainReport),
        ContractSchema::AuditExportManifest => rt!(instance, AuditExportManifest),
        ContractSchema::SignedAuditExportManifest => rt!(instance, SignedAuditExportManifest),
        ContractSchema::ConfigAppConfig => rt!(instance, AppConfig),
        ContractSchema::OrgUnit => rt!(instance, OrgUnit),
        ContractSchema::OrgPosition => rt!(instance, OrgPosition),
        ContractSchema::Competency => rt!(instance, Competency),
        ContractSchema::Delegation => rt!(instance, Delegation),
        ContractSchema::LegalInstrument => rt!(instance, LegalInstrument),
        ContractSchema::RhRole => rt!(instance, Role),
        ContractSchema::SecurityPolicy => rt!(instance, Policy),
        ContractSchema::ValidationResult => rt!(instance, ValidationResult),
        ContractSchema::ValidationReport => rt!(instance, ValidationReport),
        ContractSchema::IngestSource => rt!(instance, IngestSource),
        ContractSchema::ExportsSourceRef => rt!(instance, SourceRef),
        ContractSchema::SupportMiniError => rt!(instance, MiniError),
        ContractSchema::SupportPublicError => rt!(instance, PublicError),
        ContractSchema::SupportPostalCode => rt!(instance, PostalCode),
        ContractSchema::SupportWebAuthnChallenge => rt!(instance, WebAuthnChallenge),
        ContractSchema::SupportEncryptedPayload => rt!(instance, EncryptedPayload),
        ContractSchema::SupportTechnicalId => rt!(instance, TechnicalId),
        ContractSchema::SupportLogEvent => rt!(instance, LogEvent),
        ContractSchema::SupportStorageKey => rt!(instance, StorageKey),
        ContractSchema::SupportReleaseNotes => rt!(instance, ReleaseNotes),
        ContractSchema::UserIdentity => rt!(instance, UserIdentity),
        ContractSchema::PersonAssignment => rt!(instance, PersonAssignment),
        ContractSchema::UserProfile => rt!(instance, UserProfile),
        ContractSchema::SecurityAuthLevel => rt!(instance, AuthLevel),
        ContractSchema::SecurityClassification => rt!(instance, ResourceClassification),
        ContractSchema::SecuritySodRule => rt!(instance, SodRule),
        ContractSchema::SecurityContext => rt!(instance, SecurityContext),
        ContractSchema::ExportsTabularDataset => rt!(instance, TabularDataset),
        ContractSchema::ExportsMaterializationRequest => {
            rt!(instance, ExportMaterializationRequest)
        }
        ContractSchema::IngestBundle => rt!(instance, IngestBundle),
        ContractSchema::IngestDecision => rt!(instance, IngestDecision),
        ContractSchema::IngestEvidence => rt!(instance, IngestEvidence),
        _ => instance,
    }
}

// ── Camada 3 — assert_native_invalid ─────────────────────────────────────────

pub fn assert_native_invalid(path: &str, schema: ContractSchema) {
    let instance = load(path);
    match schema {
        ContractSchema::ControlDefinition => {
            let v = serde_json::from_value::<ControlDefinition>(instance)
                .unwrap_or_else(|e| panic!("{path} deveria desserializar: {e}"));
            assert!(
                v.validate().is_err(),
                "{path} deveria falhar validação nativa"
            );
        }
        ContractSchema::OrgUnit => {
            let v = serde_json::from_value::<OrgUnit>(instance)
                .unwrap_or_else(|e| panic!("{path} deveria desserializar: {e}"));
            assert!(
                v.validate().is_err(),
                "{path} deveria falhar validação nativa"
            );
        }
        ContractSchema::Delegation => {
            let v = serde_json::from_value::<Delegation>(instance)
                .unwrap_or_else(|e| panic!("{path} deveria desserializar: {e}"));
            assert!(
                v.validate().is_err(),
                "{path} deveria falhar validação nativa"
            );
        }
        ContractSchema::PersonAssignment => {
            let v = serde_json::from_value::<PersonAssignment>(instance)
                .unwrap_or_else(|e| panic!("{path} deveria desserializar: {e}"));
            assert!(
                v.validate().is_err(),
                "{path} deveria falhar validação nativa"
            );
        }
        ContractSchema::IngestEvidence => {
            let v = serde_json::from_value::<IngestEvidence>(instance)
                .unwrap_or_else(|e| panic!("{path} deveria desserializar: {e}"));
            assert!(
                validate_ingest_evidence(&v).is_err(),
                "{path} deveria falhar validação nativa"
            );
        }
        other => panic!("assert_native_invalid não implementado para {other:?}"),
    }
}

// ── Camada 4 — Scenario validators ───────────────────────────────────────────

pub fn validate_scenario(path: &str, kind: ScenarioKind) -> Result<(), String> {
    let raw = load(path);
    match kind {
        ScenarioKind::AuditChain => {
            let links = raw
                .as_array()
                .ok_or_else(|| format!("{path}: expected array"))?;
            validate_chain_scenario(links)
        }
        ScenarioKind::CtrlExecution => validate_ctrl_execution_scenario(path, &raw),
        ScenarioKind::OrgDelegation => validate_org_delegation_scenario(path, &raw),
        ScenarioKind::RhAssignmentOverlap => {
            let assignments = raw
                .as_array()
                .ok_or_else(|| format!("{path}: expected array"))?;
            validate_rh_assignment_overlap_scenario(assignments)
        }
    }
}

fn validate_chain_scenario(links: &[Value]) -> Result<(), String> {
    // Camada 1: cada elo passa o schema individualmente.
    let validator = validator_for(ContractSchema::AuditChainLink);
    for (i, link) in links.iter().enumerate() {
        if !validator.is_valid(link) {
            let errors = validator
                .iter_errors(link)
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("elo[{i}] falhou schema:\n{errors}"));
        }
    }
    // CHAIN-R01: sequence monotonicamente crescente.
    for w in links.windows(2) {
        let prev = w[0]["sequence"].as_u64().ok_or("sequence ausente")?;
        let curr = w[1]["sequence"].as_u64().ok_or("sequence ausente")?;
        if curr <= prev {
            return Err(format!("CHAIN-R01: sequence {curr} não é maior que {prev}"));
        }
    }
    // CHAIN-R02: previous_record_hash de cada elo == record_hash do anterior.
    for w in links.windows(2) {
        let prev_hash = w[0]["record_hash"].as_str().ok_or("record_hash ausente")?;
        let curr_prev = w[1]["previous_record_hash"]
            .as_str()
            .ok_or("previous_record_hash ausente")?;
        if prev_hash != curr_prev {
            return Err(format!(
                "CHAIN-R02: previous_record_hash={curr_prev} ≠ record_hash={prev_hash} do elo anterior"
            ));
        }
    }
    Ok(())
}

// CTRL-R03: ControlExecution não deve referenciar ControlDefinition inactiva.
fn validate_ctrl_execution_scenario(path: &str, scenario: &Value) -> Result<(), String> {
    let active = scenario["definition"]["active"]
        .as_bool()
        .ok_or_else(|| format!("{path}: definition.active não é booleano"))?;
    if !active {
        return Err(
            "CTRL-R03: ControlExecution referencia ControlDefinition inactiva (active=false)"
                .to_string(),
        );
    }
    Ok(())
}

// ORG-R08: Delegation não pode referenciar OrgPosition com status=extinct.
fn validate_org_delegation_scenario(path: &str, scenario: &Value) -> Result<(), String> {
    let status = scenario["from_position"]["status"]
        .as_str()
        .ok_or_else(|| format!("{path}: from_position.status ausente"))?;
    if status == "extinct" {
        return Err("ORG-R08: Delegation referencia OrgPosition com status=extinct".to_string());
    }
    Ok(())
}

// RH-R04: Pessoa não pode ter duas afetações activas para o mesmo cargo em períodos sobrepostos.
fn validate_rh_assignment_overlap_scenario(assignments: &[Value]) -> Result<(), String> {
    for i in 0..assignments.len() {
        for j in (i + 1)..assignments.len() {
            let a = &assignments[i];
            let b = &assignments[j];
            if a["position_id"] != b["position_id"] {
                continue;
            }
            // Datas em formato ISO "YYYY-MM-DD" — comparação lexicográfica é correcta.
            let a_from = a["valid_from"].as_str().ok_or("valid_from ausente")?;
            let b_from = b["valid_from"].as_str().ok_or("valid_from ausente")?;
            let a_until = a["valid_until"].as_str();
            let b_until = b["valid_until"].as_str();

            // Sem sobreposição sse: a_until <= b_from OU b_until <= a_from.
            let a_ends_before_b = a_until.map(|u| u <= b_from).unwrap_or(false);
            let b_ends_before_a = b_until.map(|u| u <= a_from).unwrap_or(false);

            if !a_ends_before_b && !b_ends_before_a {
                return Err(format!(
                    "RH-R04: afetações [{i}] e [{j}] para position_id '{}' têm períodos sobrepostos",
                    a["position_id"].as_str().unwrap_or("?")
                ));
            }
        }
    }
    Ok(())
}

// ── Resolução de $ref ─────────────────────────────────────────────────────────

pub fn collect_local_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref") {
                if r.starts_with("https://normordis.local/") {
                    refs.push(r.clone());
                }
            }
            for v in map.values() {
                collect_local_refs(v, refs);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_local_refs(v, refs);
            }
        }
        _ => {}
    }
}

pub fn unresolved_schema_refs() -> Vec<String> {
    let schemas_dir = spec_root().join("schemas");

    let mut known_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    fn collect_ids(dir: &std::path::Path, ids: &mut std::collections::HashSet<String>) {
        for entry in std::fs::read_dir(dir).expect("schemas dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_ids(&path, ids);
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<Value>(&content) {
                        if let Some(id) = v.get("$id").and_then(|i| i.as_str()) {
                            ids.insert(id.to_string());
                        }
                    }
                }
            }
        }
    }
    collect_ids(&schemas_dir, &mut known_ids);

    let mut unresolved: Vec<String> = Vec::new();
    fn check_refs(
        dir: &std::path::Path,
        known: &std::collections::HashSet<String>,
        unresolved: &mut Vec<String>,
    ) {
        for entry in std::fs::read_dir(dir).expect("schemas dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                check_refs(&path, known, unresolved);
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(v) = serde_json::from_str::<Value>(&content) {
                        let mut refs = Vec::new();
                        collect_local_refs(&v, &mut refs);
                        for r in refs {
                            let base = r.split('#').next().unwrap_or(&r);
                            if !known.contains(base) {
                                unresolved.push(format!("{} → {base}", path.display()));
                            }
                        }
                    }
                }
            }
        }
    }
    check_refs(&schemas_dir, &known_ids, &mut unresolved);
    unresolved
}

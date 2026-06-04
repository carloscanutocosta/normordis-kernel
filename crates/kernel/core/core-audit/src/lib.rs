//! # core-audit
//!
//! Camada de evidência auditável do kernel NORMORDIS.
//!
//! Grava eventos como evidência imutável, encadeia-os num hash chain SHA-256 verificável,
//! mantém um Registo de Controlos alinhado com COSO e mede conformidade.
//!
//! ## Posicionamento
//!
//! ```text
//! support-logging  → diagnóstico técnico (pode rodar, expirar, ser filtrado)
//! core-audit       → evidência institucional (append-only, verificável, exportável)
//! ```
//!
//! O `core-audit` não executa controlos nem decide compliance. **Regista, relaciona
//! e preserva evidência da sua execução**, respondendo às perguntas COSO:
//! *Quem fez? O quê? Quando? O controlo foi executado? Posso provar?*
//!
//! ## Dois serviços, dois stores
//!
//! ```text
//! ┌─────────────────────────────┐   ┌───────────────────────────────────┐
//! │      AuditService<S>        │   │    ControlRegistryService<S>      │
//! │                             │   │                                   │
//! │  record_event(...)          │   │  define_control(...)              │
//! │  list_by_actor(...)         │   │  record_control_execution(...)    │
//! │  verify_chain()             │   │  list_executions_by_event(...)    │
//! │  sign_and_export(...)       │   │  conformance_summary(...)         │
//! └──────────┬──────────────────┘   └──────────────┬────────────────────┘
//!            │ impl AuditStore                      │ impl ControlRegistryStore
//!            ▼                                      ▼
//!   StorageAuditStore<S>              StorageControlRegistryStore<S>
//!   (testes / volumes pequenos)       (testes / volumes pequenos)
//!
//!   AuditSqliteStore                  ControlRegistrySqliteStore
//!   (adapter-audit-sqlite)            (adapter-audit-sqlite)
//!   ← recomendado para produção →
//! ```
//!
//! ## Início rápido — registar um evento com controlo COSO
//!
//! ```rust,ignore
//! use core_audit::{
//!     AuditActor, AuditOutcome, AuditService, AuditStoreConfig, AuditTarget,
//!     ControlExecutionResult, ControlRegistryConfig, ControlRegistryService,
//!     RecordAuditEventRequest, StorageAuditStore, StorageControlRegistryStore,
//!     builtin_control_catalog,
//! };
//!
//! // 1. Configurar os dois serviços (aqui com storage em memória para exemplo)
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let storage = support_storage::MemoryStorage::new(); // exemplo
//! # let ctrl_storage = support_storage::MemoryStorage::new();
//! let audit_svc = AuditService::new(
//!     StorageAuditStore::new(storage, AuditStoreConfig::default()),
//! );
//! let ctrl_svc = ControlRegistryService::new(
//!     StorageControlRegistryStore::new(ctrl_storage, ControlRegistryConfig::default()),
//! );
//!
//! // 2. (Opcional) Carregar o catálogo base de 50 controlos COSO
//! for control in builtin_control_catalog() {
//!     ctrl_svc.define_control(&control)?;
//! }
//!
//! // 3. Registar um evento com outcome e controlo primário
//! let event = audit_svc.record_event(
//!     RecordAuditEventRequest::new(
//!         "document.classification.changed",
//!         AuditActor::new("inspector-7")?,
//!         AuditTarget::new("document", "doc-123")?,
//!         AuditOutcome::Success,
//!     )
//!     .with_control_id("CTRL-AUTH-004")  // Delegação válida
//!     .with_details(serde_json::json!({
//!         "from": "restricted",
//!         "to": "confidential",
//!     })),
//! )?;
//!
//! // 4. Registar execuções de controlos secundários sobre o mesmo evento
//! ctrl_svc.record_control_execution(
//!     "CTRL-TRACE-001",        // Evento auditável registado
//!     &event.event_id,
//!     ControlExecutionResult::Passed,
//!     None,
//!     None,
//! )?;
//! ctrl_svc.record_control_execution(
//!     "CTRL-INT-001",          // Hash calculado
//!     &event.event_id,
//!     ControlExecutionResult::Passed,
//!     Some("sha256:a3f9...".to_string()),
//!     None,
//! )?;
//!
//! // 5. Verificar todos os controlos do evento
//! let executions = ctrl_svc.list_executions_by_event(&event.event_id)?;
//! // → [CTRL-TRACE-001: Passed, CTRL-INT-001: Passed]
//!
//! // 6. Medir conformidade de um controlo (para Balanced Scorecard)
//! let summary = ctrl_svc.conformance_summary("CTRL-AUTH-004")?;
//! let rate = summary.conformance_rate(); // → Some(0.97)
//!
//! // 7. Verificar integridade da cadeia
//! let report = audit_svc.verify_chain()?;
//! println!("{} eventos verificados", report.checked_events);
//! # Ok(())
//! # }
//! ```
//!
//! ## Catálogo base de controlos
//!
//! [`builtin_control_catalog()`] devolve os 50 controlos canónicos transversais do NORMORDIS,
//! organizados em 10 categorias ([`ControlCategory`]): `AUTH`, `VAL`, `TRACE`, `DOC`, `INT`,
//! `PRIV`, `SEC`, `ING`, `EXP`, `CONT`. Cada controlo referencia as normas que endereça
//! (COSO, ISO 27001, ISO 15489, RGPD, eIDAS).
//!
//! Controlos específicos de domínio de negócio **não pertencem a este catálogo** — vivem
//! nos módulos de domínio.
//!
//! ## Conformidade e Balanced Scorecard
//!
//! [`ConformanceSummary`] agrega contagens de [`ControlExecutionResult::Passed`],
//! [`ControlExecutionResult::Failed`] e [`ControlExecutionResult::Dispensed`].
//! [`ConformanceSummary::conformance_rate()`] calcula `passed / (passed + failed)` —
//! dispensa não entra no denominador porque representa decisão formal, não falha.
//!
//! ## Integridade da evidência
//!
//! ```text
//! record_hash = SHA-256({ schema_version, sequence, previous_record_hash, event })
//! ```
//!
//! Cada leitura recomputa o hash e rejeita eventos adulterados com [`AuditError::IntegrityFailed`].
//! [`AuditService::verify_chain`] valida toda a cadeia. [`AuditService::verify_chain_since`]
//! verifica apenas eventos novos — O(novos_eventos) em vez de O(total).
//!
//! ## Restrições de dependências
//!
//! `core-audit` nunca depende de `rusqlite`, `tauri`, `core-config` ou `support-logging`.
//! Violações são detectadas por testes em `manifest_tests`.

mod actor;
mod builtin_controls;
mod chain;
mod config;
mod control_category;
mod control_definition;
mod control_execution;
mod control_registry;
mod control_service;
mod error;
mod event;
mod integrity;
mod outcome;
mod policy;
mod query;
mod service;
mod signature;
mod store;
mod target;

// ── Audit Events ──────────────────────────────────────────────────────────────

pub use actor::AuditActor;
pub use chain::{
    compute_manifest_hash, compute_record_hash, AuditChainIndex, AuditChainIndexEntry,
    AuditChainLink, AuditChainReport, AuditChainState, AuditExportManifest,
};
pub use config::{AuditStoreConfig, DEFAULT_AUDIT_EVENTS_NAMESPACE};
pub use event::AuditEvent;
pub use integrity::event_hash;
pub use outcome::AuditOutcome;
pub use query::audit_event_key;
pub use service::{AuditService, RecordAuditEventRequest};
pub use signature::{
    sign_manifest, verify_signed_manifest, AuditManifestSignature, AuditSigningKey,
    SignedAuditExportManifest, AUDIT_SIGNATURE_ALGORITHM,
};
pub use store::{AuditStore, StorageAuditStore};
pub use target::AuditTarget;

// ── Control Registry ──────────────────────────────────────────────────────────

pub use builtin_controls::builtin_control_catalog;
pub use control_category::{ControlCategory, ControlSeverity};
pub use control_definition::ControlDefinition;
pub use control_execution::{ControlExecution, ControlExecutionResult};
pub use control_registry::{
    ControlRegistryConfig, ControlRegistryStore, StorageControlRegistryStore,
    DEFAULT_CONTROL_REGISTRY_NAMESPACE,
};
pub use control_service::{ConformanceSummary, ControlRegistryService};

// ── Errors ────────────────────────────────────────────────────────────────────

pub use error::{
    AuditError, AUDIT_COMPONENT, CHAIN_VERIFICATION_FAILED, DESERIALIZATION_FAILED,
    DETAILS_TOO_LARGE, DUPLICATE_CONTROL_EXECUTION, DUPLICATE_EVENT, INTEGRITY_FAILED,
    INVALID_ACTOR, INVALID_CONTROL_DEFINITION, INVALID_CONTROL_EXECUTION, INVALID_CONTROL_ID,
    INVALID_EVENT_TYPE, INVALID_TARGET, OPERATION_FAILED, SENSITIVE_DETAILS, SERIALIZATION_FAILED,
    SIGNATURE_VERIFICATION_FAILED, SIGN_FAILED, STORE_FAILED,
};

// ── Policy constants ──────────────────────────────────────────────────────────

pub use policy::{
    DEFAULT_MAX_ACTOR_FIELD_CHARS, DEFAULT_MAX_CONTROL_ID_CHARS, DEFAULT_MAX_CONTROL_NAME_CHARS,
    DEFAULT_MAX_CONTROL_NOTES_CHARS, DEFAULT_MAX_CONTROL_REFERENCE_CHARS,
    DEFAULT_MAX_DETAILS_BYTES, DEFAULT_MAX_EVENT_TYPE_CHARS, DEFAULT_MAX_TARGET_FIELD_CHARS,
};

#[cfg(test)]
mod manifest_tests {
    use std::fs;

    fn manifest() -> String {
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap()
    }

    #[test]
    fn core_audit_does_not_depend_on_sqlite() {
        let manifest = manifest();

        assert!(!manifest.contains("rusqlite"));
        assert!(!manifest.contains("adapter-sqlite"));
    }

    #[test]
    fn core_audit_does_not_depend_on_tauri() {
        assert!(!manifest().contains("tauri"));
    }

    #[test]
    fn core_audit_does_not_depend_on_core_config() {
        assert!(!manifest().contains("core-config"));
    }

    #[test]
    fn core_audit_does_not_depend_on_support_logging() {
        assert!(!manifest().contains("support-logging"));
    }
}

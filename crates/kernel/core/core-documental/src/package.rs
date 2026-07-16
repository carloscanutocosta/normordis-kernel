//! Validação do envelope documental canónico (`DocumentPackage`).
//!
//! Os tipos de dados (`DocumentPackage`, `Artefact`, etc.) vivem em `core-doc-types`
//! (crate neutro partilhado com `core-exports`). Este módulo re-exporta-os e
//! adiciona `validate_document_package` que usa `DocumentalError`.

pub use core_doc_types::{Artefact, DocumentPackage, EngineRef, HashResult, TemplateRef};

use crate::DocumentalError;

pub fn validate_document_package(pkg: &DocumentPackage) -> Result<(), DocumentalError> {
    if pkg.document_id.trim().is_empty() {
        return Err(DocumentalError::EmptyField("document_id".into()));
    }
    if pkg.template.template_id.trim().is_empty() || pkg.template.template_version.trim().is_empty()
    {
        return Err(DocumentalError::InvalidPackage(
            "template exige template_id e template_version".into(),
        ));
    }
    if pkg.engine.engine_id.trim().is_empty() || pkg.engine.engine_version.trim().is_empty() {
        return Err(DocumentalError::InvalidPackage(
            "engine exige engine_id e engine_version".into(),
        ));
    }
    if pkg.artefacts.is_empty() {
        return Err(DocumentalError::InvalidPackage(
            "artefacts deve conter pelo menos um elemento".into(),
        ));
    }
    for (i, a) in pkg.artefacts.iter().enumerate() {
        if a.kind.trim().is_empty() || a.artefact_ref.trim().is_empty() {
            return Err(DocumentalError::InvalidPackage(format!(
                "artefacts[{i}] exige kind e ref"
            )));
        }
        if a.hash_result.algorithm.trim().is_empty() || a.hash_result.hash.trim().is_empty() {
            return Err(DocumentalError::InvalidPackage(format!(
                "artefacts[{i}].hash_result exige algorithm e hash"
            )));
        }
    }
    Ok(())
}

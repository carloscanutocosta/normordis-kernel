//! Envelope documental canónico (`DocumentPackage`) — tipo de fronteira para
//! custódia definitiva e exportação (Gate F).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::DocumentalError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateRef {
    pub template_id: String,
    pub template_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineRef {
    pub engine_id: String,
    pub engine_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HashResult {
    pub algorithm: String,
    pub hash: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artefact {
    pub kind: String,
    #[serde(rename = "ref")]
    pub artefact_ref: String,
    pub hash_result: HashResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<usize>,
}

/// Envelope documental canónico mínimo aceite pelo core para custódia definitiva.
///
/// Identifica a instância, a definição documental, o motor de produção e os
/// artefactos associados. Espelha o tipo `DocumentPackage` do Go kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentPackage {
    pub document_id: String,
    pub created_at: DateTime<Utc>,
    pub template: TemplateRef,
    pub engine: EngineRef,
    pub artefacts: Vec<Artefact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

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

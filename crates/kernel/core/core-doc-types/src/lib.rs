//! Tipos partilhados do envelope documental canónico.
//!
//! `DocumentPackage` e os seus tipos auxiliares são usados em toda a pipeline
//! de produção/custódia/exportação. Extraídos para crate neutro para evitar
//! que `core-documental` e `core-exports` dependam um do outro.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
/// artefactos associados.
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

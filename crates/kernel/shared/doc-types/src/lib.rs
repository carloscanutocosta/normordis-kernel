//! Shared Kernel — tipos do envelope documental canónico.
//!
//! Contém [`DocumentPackage`] e os seus tipos auxiliares, partilhados entre
//! `core-documental` (validação de entrada) e `core-exports` (construção do
//! envelope para Gate F), sem criar dependência circular entre eles.
//!
//! ## Regra de evolução
//!
//! Este crate deve permanecer **neutro**: só tipos de dados serializáveis, sem
//! lógica de domínio, sem ports e sem dependências de outros bounded contexts.
//! Se um tipo precisar de validação rica ou de comportamento, pertence ao bounded
//! context que o consome — não aqui.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Referência ao template NDT usado na produção do documento.
///
/// Identifica o template pelo seu `template_id` e a versão exacta utilizada,
/// permitindo reconstruir o documento exactamente como foi produzido mesmo que
/// o template evolua posteriormente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateRef {
    /// Identificador canónico do template (ex: `"tpl-certidao-v2"`).
    pub template_id: String,
    /// Versão do template no momento da produção (ex: `"v2"`).
    pub template_version: String,
    /// Instante em que o template estava em vigor — usado para auditoria
    /// temporal quando o mesmo `template_version` pode ter sido alterado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
}

/// Referência ao motor de renderização que produziu o documento.
///
/// Permite identificar exatamente qual versão do motor (ex: Typst 0.13, pdf-pipeline
/// 2.1) gerou o artefacto, fundamental para reprodutibilidade e auditoria forense.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineRef {
    /// Identificador do motor (ex: `"typst"`, `"pdf-pipeline"`).
    pub engine_id: String,
    /// Versão semântica do motor (ex: `"0.13.0"`).
    pub engine_version: String,
}

/// Resultado de uma operação de hash sobre um artefacto ou conteúdo.
///
/// Permite verificar integridade e detectar adulteração em qualquer momento
/// posterior à produção, independentemente do algoritmo usado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HashResult {
    /// Algoritmo usado (ex: `"sha256"`, `"blake3"`).
    pub algorithm: String,
    /// Valor hexadecimal do hash.
    pub hash: String,
    /// Momento em que o hash foi calculado.
    pub timestamp: DateTime<Utc>,
    /// Tipo de entrada hashada (ex: `"artefact"`, `"template"`, `"content"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_kind: Option<String>,
    /// Referência à entrada (ex: path, id, URL interna).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_ref: Option<String>,
    /// Metadados adicionais livres (ex: encoding, charset, compressão).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

/// Artefacto produzido durante o pipeline documental.
///
/// Um `DocumentPackage` pode conter múltiplos artefactos (ex: PDF principal,
/// JSON de dados, assinatura detached, thumbnail). Cada artefacto é identificado
/// pelo seu `kind` e verificável pelo `hash_result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artefact {
    /// Tipo do artefacto (ex: `"pdf"`, `"json"`, `"signature"`, `"thumbnail"`).
    pub kind: String,
    /// Referência interna ao artefacto (path, URL interna, content-address hash).
    #[serde(rename = "ref")]
    pub artefact_ref: String,
    /// Hash de integridade do conteúdo do artefacto.
    pub hash_result: HashResult,
    /// MIME type do artefacto (ex: `"application/pdf"`, `"application/json"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    /// Tamanho em bytes do artefacto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<usize>,
}

/// Envelope documental canónico para Gate F.
///
/// Representa o pacote completo de um documento produzido — identifica a instância,
/// o template usado, o motor de renderização e todos os artefactos gerados.
/// É o contrato de entrada em `core-documental` (`validate_document_package`) e
/// o contrato de saída de `core-exports`.
///
/// ## Exemplo
///
/// ```rust
/// use core_doc_types::{DocumentPackage, TemplateRef, EngineRef, Artefact, HashResult};
/// use chrono::Utc;
///
/// let pkg = DocumentPackage {
///     document_id: "doc-certidao-2026-001".into(),
///     created_at: Utc::now(),
///     template: TemplateRef {
///         template_id: "tpl-certidao-v2".into(),
///         template_version: "v2".into(),
///         valid_at: None,
///     },
///     engine: EngineRef {
///         engine_id: "typst".into(),
///         engine_version: "0.13.0".into(),
///     },
///     artefacts: vec![Artefact {
///         kind: "pdf".into(),
///         artefact_ref: "sha256:abc123...".into(),
///         hash_result: HashResult {
///             algorithm: "sha256".into(),
///             hash: "abc123...".into(),
///             timestamp: Utc::now(),
///             input_kind: Some("artefact".into()),
///             input_ref: None,
///             meta: None,
///         },
///         mime: Some("application/pdf".into()),
///         size_bytes: Some(142_500),
///     }],
///     subject: None,
///     meta: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentPackage {
    /// Identificador único da instância documental.
    pub document_id: String,
    /// Momento de criação do envelope.
    pub created_at: DateTime<Utc>,
    /// Template NDT utilizado na produção.
    pub template: TemplateRef,
    /// Motor de renderização utilizado.
    pub engine: EngineRef,
    /// Lista de artefactos produzidos (PDF, JSON, assinatura, etc.).
    pub artefacts: Vec<Artefact>,
    /// Dados de assunto do documento (schema definido pelo tipo documental).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Value>,
    /// Metadados livres para extensão sem quebrar o contrato.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

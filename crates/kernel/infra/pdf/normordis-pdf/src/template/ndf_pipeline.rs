use regex::Regex;
use serde_json::Value;

use super::data::NdtData;
use super::resolver;
use crate::ndf::{
    audit::{Actor, AuditEvent, EventType, NdfAudit},
    integrity::{canonical_hash, NdfIntegrity},
    NdfDocument, NdfMeta, NdfOrigin, NDF_VERSION,
};
use crate::{NormaxisPdfError, Result};

// ── CompileOptions ────────────────────────────────────────────────────────────

/// Options controlling how `compile_ndt()` builds an `NdfDocument`.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Unique document identifier.
    /// If `None`, a UUID v4 is generated automatically.
    pub document_id: Option<String>,

    /// Actor responsible for this generation (stored in audit chain).
    pub generated_by: Actor,

    /// NDT template identifier for origin traceability.
    pub ndt_template_id: Option<String>,

    /// SHA-256 hash of the NDT template file.
    /// If `None`, computed automatically from the `ndt` input.
    pub ndt_template_hash: Option<String>,

    /// If `true` (default), error when any `{{placeholder}}` remains unresolved.
    pub validate_resolved: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            document_id: None,
            generated_by: Actor::System {
                id: "normordis-pdf".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                instance_id: None,
            },
            ndt_template_id: None,
            ndt_template_hash: None,
            validate_resolved: true,
        }
    }
}

// ── compile_ndt ───────────────────────────────────────────────────────────────

/// Compiles an NDT template + data into a fully resolved `NdfDocument`.
///
/// Pipeline:
/// 1. Parse NDT (JSON or TOML)
/// 2. Validate required placeholders
/// 3. Deep-substitute `{{placeholders}}` in all body string fields
/// 4. Check no unresolved placeholders remain (`validate_resolved`)
/// 5. Compute integrity hashes (RFC 8785 / JCS)
/// 6. Build and return `NdfDocument`
pub fn compile_ndt(ndt: &str, data: &NdtData, options: CompileOptions) -> Result<NdfDocument> {
    let doc = super::parse_ndt(ndt)
        .map_err(|e| NormaxisPdfError::NdfCompileError(e.to_string()))?;

    if let Some(ref placeholders) = doc.placeholders {
        super::validator::validate(placeholders, data)
            .map_err(|e| NormaxisPdfError::NdfCompileError(e.to_string()))?;
    }

    // Serialize and resolve body + style
    let body_val = serde_json::to_value(&doc.body)
        .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;
    let resolved_content = resolve_value_placeholders(body_val, data);

    let styles_val = serde_json::to_value(&doc.style)
        .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;

    if options.validate_resolved {
        let content_str = serde_json::to_string(&resolved_content)
            .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;
        let re = Regex::new(r"\{\{[a-zA-Z0-9_.]+\}\}").expect("static regex");
        if let Some(m) = re.find(&content_str) {
            return Err(NormaxisPdfError::NdfCompileError(format!(
                "unresolved placeholder '{}' in content after substitution",
                m.as_str()
            )));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    let meta_title = doc
        .meta
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_default();
    let meta_compat = doc.meta.as_ref().and_then(|m| m.compat_mode);
    let meta = NdfMeta {
        title: meta_title,
        entity: String::new(),
        entity_id: None,
        lang: "pt-PT".into(),
        document_ref: None,
        document_type: None,
        classification: "public".into(),
        subject: None,
        keywords: None,
        created_at: now.clone(),
        valid_from: None,
        valid_until: None,
        supersedes: None,
        compat_mode: meta_compat,
        numbering: None,
    };
    let meta_val = serde_json::to_value(&meta)
        .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;

    let integrity = NdfIntegrity::compute(&resolved_content, &styles_val, &meta_val)?;
    let document_id = options
        .document_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let ndt_template_hash = options.ndt_template_hash.unwrap_or_else(|| {
        let v = serde_json::from_str::<Value>(ndt).unwrap_or(Value::Null);
        canonical_hash(&v)
    });

    let first_event = AuditEvent {
        seq: 1,
        event_type: EventType::DocumentGenerated,
        timestamp: now.clone(),
        actor: options.generated_by.clone(),
        content_hash: Some(integrity.content_hash.clone()),
        note: None,
        extra: Default::default(),
    };

    Ok(NdfDocument {
        ndf: NDF_VERSION.into(),
        origin: NdfOrigin {
            ndt_template_id: options.ndt_template_id,
            ndt_version: None,
            ndt_template_hash: Some(ndt_template_hash),
            ndt_data_hash: None,
            engine_version: env!("CARGO_PKG_VERSION").into(),
            engine_backend: "normordis-pdf".into(),
            generated_at: now,
            generated_by: options.generated_by,
        },
        revision: None,
        meta,
        output: serde_json::to_value(&doc.style).ok(),
        styles: styles_val,
        content: resolved_content,
        integrity,
        audit: NdfAudit {
            document_id,
            events: vec![first_event],
        },
        outputs: vec![],
        signatures: vec![],
    })
}

// ── parse_ndf / render_ndf / verify_ndf ──────────────────────────────────────

/// Parses an NDF document from JSON (canonical or pretty-printed).
pub fn parse_ndf(json: &str) -> Result<NdfDocument> {
    serde_json::from_str(json).map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))
}

/// Verifies the integrity hashes of an NDF document.
pub fn verify_ndf(json: &str) -> Result<crate::ndf::integrity::IntegrityReport> {
    let ndf = parse_ndf(json)?;
    ndf.verify_integrity()
}

/// Renders an NDF document to PDF bytes.
pub fn render_ndf(ndf_json: &str) -> Result<Vec<u8>> {
    let ndf = parse_ndf(ndf_json)?;

    let body: Vec<super::model::BodyElement> =
        serde_json::from_value(ndf.content.clone())
            .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;

    let ndt_doc = super::model::NdtDocument {
        ndt: "1.1.0".into(),
        id: None,
        meta: Some(super::model::NdtMeta {
            title: Some(ndf.meta.title.clone()),
            description: None,
            author: None,
            version: None,
            created_at: None,
            compat_mode: ndf.meta.compat_mode,
        }),
        style: serde_json::from_value(ndf.styles.clone()).ok(),
        fonts: None,
        page: None,
        output: None,
        signature: None,
        placeholders: None,
        zones: None,
        body,
    };

    let empty_data = NdtData {
        ndt_data: "1.0.0".into(),
        template_id: None,
        template_version: None,
        data: Default::default(),
    };

    let style = crate::styles::DocumentStyle::default();
    let elements = super::renderer::render_template(&ndt_doc, &empty_data, &style)
        .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;

    crate::document::Document {
        title: ndf.meta.title,
        style,
        fonts: crate::fonts::FontRegistry::default(),
        header: None,
        sectioned_header: None,
        footer: None,
        sectioned_footer: None,
        watermark: None,
        elements,
        footnotes: vec![],
        toc_entries: None,
        compression: crate::document::CompressionLevel::Default,
        standard: crate::document::PdfStandard::Pdf17,
        signature: None,
        traceability: None,
        accessibility: crate::compliance::ua::AccessibilityConfig::default(),
    }
    .render_to_bytes()
}

/// Renders an NDF document to a `PreparedPdf` ready for external PKCS#7 signing.
pub fn render_ndf_prepared_for_signing(
    ndf_json: &str,
    opts: crate::signing::SignatureOptions,
) -> Result<crate::signing::PreparedPdf> {
    let ndf = parse_ndf(ndf_json)?;

    let body: Vec<super::model::BodyElement> =
        serde_json::from_value(ndf.content.clone())
            .map_err(|e| NormaxisPdfError::SerdeError(e.to_string()))?;

    let ndt_doc = super::model::NdtDocument {
        ndt: "1.1.0".into(),
        id: None,
        meta: Some(super::model::NdtMeta {
            title: Some(ndf.meta.title.clone()),
            description: None,
            author: None,
            version: None,
            created_at: None,
            compat_mode: ndf.meta.compat_mode,
        }),
        style: serde_json::from_value(ndf.styles.clone()).ok(),
        fonts: None,
        page: None,
        output: None,
        signature: None,
        placeholders: None,
        zones: None,
        body,
    };

    let empty_data = NdtData {
        ndt_data: "1.0.0".into(),
        template_id: None,
        template_version: None,
        data: Default::default(),
    };

    let style = crate::styles::DocumentStyle::default();
    let elements = super::renderer::render_template(&ndt_doc, &empty_data, &style)
        .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;

    crate::document::Document {
        title: ndf.meta.title,
        style,
        fonts: crate::fonts::FontRegistry::default(),
        header: None,
        sectioned_header: None,
        footer: None,
        sectioned_footer: None,
        watermark: None,
        elements,
        footnotes: vec![],
        toc_entries: None,
        compression: crate::document::CompressionLevel::Default,
        standard: crate::document::PdfStandard::Pdf17,
        signature: None,
        traceability: None,
        accessibility: crate::compliance::ua::AccessibilityConfig::default(),
    }
    .render_prepared_for_signing(opts)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Recursively substitutes `{{placeholder}}` patterns in all string nodes of a JSON value.
fn resolve_value_placeholders(value: Value, data: &NdtData) -> Value {
    match value {
        Value::String(s) => Value::String(resolver::resolve_string(&s, data)),
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|v| resolve_value_placeholders(v, data)).collect())
        }
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k, resolve_value_placeholders(v, data));
            }
            Value::Object(new_map)
        }
        other => other,
    }
}

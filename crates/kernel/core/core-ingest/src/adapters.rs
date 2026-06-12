use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::IngestError;
use crate::types::{
    ContentValidator, IngestBundle, IngestStoragePort, ScanAdapter, ScanInput, ScanResult,
};

// ── ScanAdapter de teste ───────────────────────────────────────────────────────

/// Scanner determinístico para testes: rejeita hashes específicos, aceita os restantes.
pub struct DeterministicScanner {
    pub adapter_name: String,
    /// Mapa de hash → razão de rejeição. Hashes ausentes resultam em "clean".
    pub rejected_hashes: HashMap<String, String>,
}

impl Default for DeterministicScanner {
    fn default() -> Self {
        Self {
            adapter_name: "deterministic".into(),
            rejected_hashes: HashMap::new(),
        }
    }
}

impl ScanAdapter for DeterministicScanner {
    fn scan(&self, input: &ScanInput) -> Result<ScanResult, IngestError> {
        let adapter = non_empty_or(&self.adapter_name, "deterministic");
        if let Some(reason) = self.rejected_hashes.get(&input.bundle_hash) {
            return Ok(ScanResult {
                adapter: adapter.into(),
                verdict: "rejected".into(),
                reason: Some(reason.clone()),
            });
        }
        Ok(ScanResult {
            adapter: adapter.into(),
            verdict: "clean".into(),
            reason: None,
        })
    }

    fn adapter_id(&self) -> &str {
        non_empty_or(&self.adapter_name, "deterministic")
    }
}

// ── ContentValidator de teste ─────────────────────────────────────────────────

/// Validador de conteúdo que aceita sempre — para uso em testes.
pub struct PassthroughContentValidator;

impl ContentValidator for PassthroughContentValidator {
    fn validate(&self, _raw: &[u8], _content_type: &str) -> Result<(), IngestError> {
        Ok(())
    }
}

/// Validador que rejeita qualquer conteúdo com `content_type` indicado — para testes de rejeição.
pub struct RejectingContentValidator {
    pub rejected_content_types: Vec<String>,
    pub reason: String,
}

impl ContentValidator for RejectingContentValidator {
    fn validate(&self, _raw: &[u8], content_type: &str) -> Result<(), IngestError> {
        if self.rejected_content_types.iter().any(|ct| ct == content_type) {
            return Err(IngestError::ContentValidationFailed {
                content_type: content_type.into(),
                reason: self.reason.clone(),
            });
        }
        Ok(())
    }
}

// ── IngestStoragePort de teste ────────────────────────────────────────────────

/// Storage em memória para testes — preserva document_ref por bundle_id (idempotente).
pub struct MemoryStoragePort {
    records: Mutex<HashMap<String, String>>,
}

impl Default for MemoryStoragePort {
    fn default() -> Self {
        Self { records: Mutex::new(HashMap::new()) }
    }
}

impl IngestStoragePort for MemoryStoragePort {
    fn store(&self, bundle: &IngestBundle, verified_hash: &str) -> Result<String, IngestError> {
        let mut records = self.records.lock().expect("MemoryStoragePort mutex poisoned");
        if let Some(existing) = records.get(&bundle.bundle_id) {
            return Ok(existing.clone());
        }
        // slug dos primeiros 12 chars do hex após "sha256:"
        let hash_slug = verified_hash
            .get(7..19)
            .unwrap_or_else(|| &verified_hash[..verified_hash.len().min(12)]);
        let doc_ref = format!("doc:{}", hash_slug);
        records.insert(bundle.bundle_id.clone(), doc_ref.clone());
        Ok(doc_ref)
    }
}

// ── Auxiliares ────────────────────────────────────────────────────────────────

fn non_empty_or<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    if s.is_empty() { fallback } else { s }
}

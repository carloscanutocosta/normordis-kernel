use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::IngestError;
use crate::types::{RouteInput, RouteResult, Router, ScanAdapter, ScanInput, ScanResult};

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
        let adapter = if self.adapter_name.is_empty() {
            "deterministic"
        } else {
            &self.adapter_name
        };
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
        if self.adapter_name.is_empty() {
            "deterministic"
        } else {
            &self.adapter_name
        }
    }
}

/// Router em memória para testes: preserva a rota por hash do bundle.
pub struct MemoryRouter {
    target: String,
    records: Mutex<HashMap<String, RouteResult>>,
}

impl MemoryRouter {
    pub fn new(target: impl Into<String>) -> Self {
        let t = target.into();
        let target = if t.is_empty() {
            "ingest/config-bundle".into()
        } else {
            t
        };
        Self {
            target,
            records: Mutex::new(HashMap::new()),
        }
    }
}

impl Router for MemoryRouter {
    fn route(&self, input: &RouteInput) -> Result<RouteResult, IngestError> {
        let mut records = self.records.lock().expect("MemoryRouter mutex poisoned");

        if let Some(existing) = records.get(&input.bundle_hash) {
            return Ok(existing.clone());
        }

        // "sha256:HEXHEX..." → 12 chars a partir do índice 7 (após "sha256:")
        let hash_slug = input
            .bundle_hash
            .get(7..19)
            .unwrap_or_else(|| &input.bundle_hash[..input.bundle_hash.len().min(12)]);

        let result = RouteResult {
            target: self.target.clone(),
            route_ref: format!("route:{hash_slug}"),
        };
        records.insert(input.bundle_hash.clone(), result.clone());
        Ok(result)
    }
}

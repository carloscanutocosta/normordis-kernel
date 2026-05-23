use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::MefError;

/// Código hierárquico MEF validado e normalizado (trim, não vazio).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MefCode(String);

impl MefCode {
    pub fn new(code: impl Into<String>) -> Result<Self, MefError> {
        let code = code.into().trim().to_string();
        if code.is_empty() {
            return Err(MefError::EmptyCode);
        }
        Ok(Self(code))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MefCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Referência ao diploma legal que fundamenta uma versão da tabela MEF.
///
/// Exemplos: "Portaria n.º 1258/2009, de 15 de outubro" ou
/// "Despacho n.º 12/2024, de 3 de janeiro".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomaRef {
    /// Identificação normativa completa do diploma publicado.
    pub reference: String,
    /// Data de publicação ISO (YYYY-MM-DD), quando disponível.
    pub date: Option<String>,
}

impl DiplomaRef {
    pub fn new(reference: impl Into<String>) -> Result<Self, MefError> {
        let reference = reference.into().trim().to_string();
        if reference.is_empty() {
            return Err(MefError::EmptyDiplomaRef);
        }
        Ok(Self { reference, date: None })
    }

    pub fn with_date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }
}

/// Uma entrada da classificação MEF num dado período de vigência.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MefEntry {
    pub code: MefCode,
    pub label: String,
    pub parent_code: Option<MefCode>,
    pub is_usable: bool,
    /// Instante a partir do qual esta versão é válida.
    pub effective_from: DateTime<Utc>,
    /// None enquanto a versão está activa.
    pub effective_to: Option<DateTime<Utc>>,
    pub changed_by: String,
    pub change_reason: Option<String>,
    /// Diploma legal que fundamenta esta versão (portaria, decreto, despacho…).
    pub diploma: Option<DiplomaRef>,
}

impl MefEntry {
    pub fn is_root(&self) -> bool {
        self.parent_code.is_none()
    }

    pub fn is_active(&self) -> bool {
        self.effective_to.is_none()
    }
}

/// Pedido para inserir ou actualizar uma entrada MEF.
#[derive(Debug, Clone)]
pub struct UpsertMefEntryRequest {
    pub code: MefCode,
    pub label: String,
    pub parent_code: Option<MefCode>,
    pub is_usable: bool,
    pub changed_by: String,
    pub change_reason: Option<String>,
    /// Diploma que autoriza esta versão. Obrigatório quando a alteração é normativa.
    pub diploma: Option<DiplomaRef>,
}

impl UpsertMefEntryRequest {
    pub fn validate(&self) -> Result<(), MefError> {
        if self.changed_by.trim().is_empty() {
            return Err(MefError::EmptyChangedBy);
        }
        Ok(())
    }
}

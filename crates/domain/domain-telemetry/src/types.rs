use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::TelemetryError;

/// Identificador único de um evento de telemetria.
/// Tipicamente gerado pelo chamador com UUID v4.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UsageEventId(String);

impl UsageEventId {
    pub fn new(s: impl Into<String>) -> Result<Self, TelemetryError> {
        let s = s.into().trim().to_string();
        if s.is_empty() {
            return Err(TelemetryError::EmptyField("event_id"));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UsageEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identificador de sessão — agrupa eventos do mesmo ciclo de vida (open → close).
/// Gerado pelo chamador quando a app é aberta; incluído em todos os eventos até fecho.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(s: impl Into<String>) -> Result<Self, TelemetryError> {
        let s = s.into().trim().to_string();
        if s.is_empty() {
            return Err(TelemetryError::EmptyField("session_id"));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Tipo de evento de utilização que uma app pode emitir.
/// Contexto adicional (código de erro, nome da acção) vai em `metadata`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageEventType {
    /// App aberta pelo utilizador — inicia uma sessão.
    AppOpened,
    /// App fechada normalmente — termina uma sessão.
    AppClosed,
    /// App falhou com erro irrecuperável.
    AppFailed,
    /// Acção de negócio completada com sucesso (ex: documento criado).
    ActionCompleted,
    /// Acção de negócio falhou.
    ActionFailed,
    /// Acesso negado a recurso ou operação.
    PermissionDenied,
}

impl UsageEventType {
    pub fn as_str(&self) -> &str {
        match self {
            UsageEventType::AppOpened => "AppOpened",
            UsageEventType::AppClosed => "AppClosed",
            UsageEventType::AppFailed => "AppFailed",
            UsageEventType::ActionCompleted => "ActionCompleted",
            UsageEventType::ActionFailed => "ActionFailed",
            UsageEventType::PermissionDenied => "PermissionDenied",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, TelemetryError> {
        match s {
            "AppOpened" => Ok(UsageEventType::AppOpened),
            "AppClosed" => Ok(UsageEventType::AppClosed),
            "AppFailed" => Ok(UsageEventType::AppFailed),
            "ActionCompleted" => Ok(UsageEventType::ActionCompleted),
            "ActionFailed" => Ok(UsageEventType::ActionFailed),
            "PermissionDenied" => Ok(UsageEventType::PermissionDenied),
            other => Err(TelemetryError::Storage(format!(
                "tipo de evento desconhecido: {other}"
            ))),
        }
    }
}

impl std::fmt::Display for UsageEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Evento de utilização emitido por uma app registada.
///
/// `app_id` referencia o identificador do catálogo (`domain-registry`).
/// `session_id` agrupa todos os eventos de um ciclo abrir→fechar.
/// `metadata` transporta contexto adicional (ex: `"action" → "criar_documento"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageEvent {
    pub event_id: UsageEventId,
    pub app_id: String,
    pub session_id: SessionId,
    pub user_id: String,
    pub org_unit_id: Option<String>,
    pub event_type: UsageEventType,
    pub occurred_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl AppUsageEvent {
    pub fn validate(&self) -> Result<(), TelemetryError> {
        if self.app_id.trim().is_empty() {
            return Err(TelemetryError::EmptyField("app_id"));
        }
        if self.user_id.trim().is_empty() {
            return Err(TelemetryError::EmptyField("user_id"));
        }
        Ok(())
    }
}

/// Período de tempo para agregação de estatísticas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsagePeriod {
    /// Um dia específico.
    Day(NaiveDate),
    /// Um mês de um ano (month: 1–12).
    Month { year: i32, month: u32 },
    /// Um ano completo.
    Year(i32),
}

impl UsagePeriod {
    pub fn validate(&self) -> Result<(), TelemetryError> {
        if let UsagePeriod::Month { month, .. } = self {
            if *month == 0 || *month > 12 {
                return Err(TelemetryError::InvalidPeriod(format!(
                    "mês inválido: {month}"
                )));
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for UsagePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsagePeriod::Day(d) => write!(f, "{d}"),
            UsagePeriod::Month { year, month } => write!(f, "{year}-{month:02}"),
            UsagePeriod::Year(y) => write!(f, "{y}"),
        }
    }
}

/// Estatísticas de utilização de uma app num determinado período.
///
/// `avg_session_duration_secs` é `None` quando não há sessões completas
/// (pares AppOpened + AppClosed com o mesmo `session_id`) no período.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageStats {
    pub app_id: String,
    pub period: UsagePeriod,
    /// Número de eventos AppOpened no período.
    pub opened_count: u64,
    /// Número de eventos AppClosed no período.
    pub closed_count: u64,
    /// Número de eventos AppFailed no período.
    pub failure_count: u64,
    /// Sessões únicas iniciadas no período (AppOpened distintos por session_id).
    pub session_count: u64,
    /// Utilizadores únicos que abriram a app no período.
    pub unique_users: u64,
    /// Duração média de sessão em segundos para sessões completas (open+close).
    /// `None` se não existirem sessões com AppClosed correspondente.
    pub avg_session_duration_secs: Option<f64>,
}

/// Filtro para pesquisa de eventos brutos. Campos `None` não aplicam filtro.
#[derive(Debug, Clone, Default)]
pub struct UsageEventFilter {
    pub app_id: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub org_unit_id: Option<String>,
    pub event_type: Option<UsageEventType>,
    pub occurred_after: Option<DateTime<Utc>>,
    pub occurred_before: Option<DateTime<Utc>>,
}

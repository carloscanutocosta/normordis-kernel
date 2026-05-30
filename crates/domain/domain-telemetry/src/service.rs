use crate::{
    AppUsageEvent, AppUsageStats, TelemetryError, TelemetryRepository,
    UsageEventFilter, UsagePeriod,
};

/// Ponto de entrada único para registo e consulta de telemetria de uso de apps.
///
/// Valida os invariantes de domínio antes de delegar ao repositório.
pub struct TelemetryService<R: TelemetryRepository> {
    repo: R,
}

impl<R: TelemetryRepository> TelemetryService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Regista um evento de utilização.
    ///
    /// Valida que `app_id` e `user_id` são não-vazios.
    /// Os identificadores `event_id` e `session_id` são validados em construção.
    pub fn record_event(&self, event: AppUsageEvent) -> Result<(), R::Error> {
        event.validate()?;
        self.repo.record(&event)
    }

    /// Pesquisa eventos brutos com filtro opcional e paginação por offset.
    pub fn query_events(
        &self,
        filter: &UsageEventFilter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AppUsageEvent>, R::Error> {
        self.repo.query_events(filter, limit, offset)
    }

    /// Estatísticas de utilização de uma app num período.
    /// Devolve `None` se não existirem eventos no período.
    pub fn stats_for_app(
        &self,
        app_id: &str,
        period: &UsagePeriod,
    ) -> Result<Option<AppUsageStats>, R::Error> {
        if app_id.trim().is_empty() {
            return Err(TelemetryError::EmptyField("app_id").into());
        }
        period.validate()?;
        self.repo.stats_for_app(app_id, period)
    }

    /// Top N apps por utilização no período (número de aberturas).
    pub fn top_apps(
        &self,
        period: &UsagePeriod,
        limit: usize,
    ) -> Result<Vec<AppUsageStats>, R::Error> {
        period.validate()?;
        self.repo.top_apps(period, limit)
    }
}

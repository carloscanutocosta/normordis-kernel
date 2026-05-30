use crate::{
    AppUsageEvent, AppUsageStats, TelemetryError, UsageEventFilter, UsagePeriod,
};

pub trait TelemetryRepository {
    type Error: From<TelemetryError>;

    /// Persiste um evento de utilização.
    /// Idempotente: um `event_id` duplicado é ignorado silenciosamente.
    fn record(&self, event: &AppUsageEvent) -> Result<(), Self::Error>;

    /// Pesquisa eventos que satisfazem o filtro.
    /// Campos `None` no filtro não aplicam restrição.
    /// Resultados ordenados por `occurred_at` DESC.
    /// `offset` permite paginação.
    fn query_events(
        &self,
        filter: &UsageEventFilter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AppUsageEvent>, Self::Error>;

    /// Estatísticas agregadas de uma app num período.
    /// Devolve `None` se não existirem eventos para o par (app_id, período).
    fn stats_for_app(
        &self,
        app_id: &str,
        period: &UsagePeriod,
    ) -> Result<Option<AppUsageStats>, Self::Error>;

    /// Top N apps por número de aberturas no período, ordenadas DESC.
    /// `avg_session_duration_secs` é sempre `None` neste resultado (não calculado).
    fn top_apps(
        &self,
        period: &UsagePeriod,
        limit: usize,
    ) -> Result<Vec<AppUsageStats>, Self::Error>;
}

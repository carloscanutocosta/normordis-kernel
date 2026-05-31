/*!
 * Adapter SQLite para telemetria de uso de apps (domain-telemetry).
 *
 * Tabela:
 *   platform_app_usage_events — eventos de uso (append-only, nunca UPDATE/DELETE)
 *
 * Estatísticas são derivadas por SQL em tempo de consulta — não existem
 * tabelas de agregação pré-computadas. Para volumes institucionais da AP
 * (dezenas de apps × milhares de eventos/mês) este modelo é adequado.
 *
 * Duração média de sessão: auto-join emparelhando AppOpened com o primeiro
 * AppClosed de cada session_id (MIN subquery evita dupla contagem se existir
 * mais do que um AppClosed para a mesma sessão).
 *
 * Idempotência: INSERT OR IGNORE garante que event_id duplicados são ignorados
 * silenciosamente (retry de rede transparente para o chamador).
 */

use chrono::{NaiveDate, NaiveTime};
use rusqlite::{params, Connection};
use thiserror::Error;

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use domain_telemetry::{
    AppUsageEvent, AppUsageStats, SessionId, TelemetryError, TelemetryRepository, UsageEventFilter,
    UsageEventId, UsageEventType, UsagePeriod,
};
use support_errors::MiniError;

// ─── Migrations ───────────────────────────────────────────────────────────────

pub const TELEMETRY_MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE IF NOT EXISTS platform_app_usage_events (
        event_id      TEXT NOT NULL PRIMARY KEY,
        app_id        TEXT NOT NULL,
        session_id    TEXT NOT NULL,
        user_id       TEXT NOT NULL,
        org_unit_id   TEXT,
        event_type    TEXT NOT NULL,
        occurred_at   TEXT NOT NULL,
        metadata      TEXT NOT NULL DEFAULT '{}'
    );

    CREATE INDEX IF NOT EXISTS idx_telemetry_app_time
        ON platform_app_usage_events (app_id, occurred_at DESC);

    CREATE INDEX IF NOT EXISTS idx_telemetry_user_time
        ON platform_app_usage_events (user_id, occurred_at DESC);

    CREATE INDEX IF NOT EXISTS idx_telemetry_session
        ON platform_app_usage_events (session_id);

    CREATE INDEX IF NOT EXISTS idx_telemetry_time
        ON platform_app_usage_events (occurred_at DESC);
"#];

// ─── Erros ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum TelemetrySqliteError {
    #[error(transparent)]
    Adapter(#[from] MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] TelemetryError),
}

// ─── Store ────────────────────────────────────────────────────────────────────

pub struct TelemetrySqliteStore {
    conn: Connection,
}

impl TelemetrySqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, TelemetrySqliteError> {
        let conn = open_relational_connection(config)?;
        run_relational_migrations(&conn, TELEMETRY_MIGRATIONS)?;
        Ok(Self { conn })
    }
}

// ─── Helpers internos ─────────────────────────────────────────────────────────

fn decode_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Converte um `UsagePeriod` em intervalo [start, end) como strings RFC3339.
fn period_to_range(period: &UsagePeriod) -> (String, String) {
    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

    let (start, end) = match period {
        UsagePeriod::Day(date) => {
            let next = *date + chrono::Duration::days(1);
            (
                date.and_time(midnight).and_utc(),
                next.and_time(midnight).and_utc(),
            )
        }
        UsagePeriod::Month { year, month } => {
            let (ny, nm) = if *month == 12 {
                (*year + 1, 1u32)
            } else {
                (*year, *month + 1)
            };
            (
                NaiveDate::from_ymd_opt(*year, *month, 1)
                    .unwrap()
                    .and_time(midnight)
                    .and_utc(),
                NaiveDate::from_ymd_opt(ny, nm, 1)
                    .unwrap()
                    .and_time(midnight)
                    .and_utc(),
            )
        }
        UsagePeriod::Year(year) => (
            NaiveDate::from_ymd_opt(*year, 1, 1)
                .unwrap()
                .and_time(midnight)
                .and_utc(),
            NaiveDate::from_ymd_opt(*year + 1, 1, 1)
                .unwrap()
                .and_time(midnight)
                .and_utc(),
        ),
    };

    (start.to_rfc3339(), end.to_rfc3339())
}

fn row_to_event(
    event_id: String,
    app_id: String,
    session_id: String,
    user_id: String,
    org_unit_id: Option<String>,
    event_type_str: String,
    occurred_at_str: String,
    metadata_json: String,
) -> Result<AppUsageEvent, TelemetrySqliteError> {
    let event_type = UsageEventType::from_str(&event_type_str)?;
    let metadata: std::collections::HashMap<String, String> = serde_json::from_str(&metadata_json)?;
    Ok(AppUsageEvent {
        event_id: UsageEventId::new(event_id)?,
        app_id,
        session_id: SessionId::new(session_id)?,
        user_id,
        org_unit_id,
        event_type,
        occurred_at: decode_datetime(&occurred_at_str),
        metadata,
    })
}

// ─── TelemetryRepository impl ─────────────────────────────────────────────────

impl TelemetryRepository for TelemetrySqliteStore {
    type Error = TelemetrySqliteError;

    fn record(&self, event: &AppUsageEvent) -> Result<(), Self::Error> {
        let metadata_json = serde_json::to_string(&event.metadata)?;
        // INSERT OR IGNORE: event_id duplicado é ignorado silenciosamente.
        self.conn.execute(
            "INSERT OR IGNORE INTO platform_app_usage_events \
             (event_id, app_id, session_id, user_id, org_unit_id, \
              event_type, occurred_at, metadata) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.event_id.as_str(),
                event.app_id,
                event.session_id.as_str(),
                event.user_id,
                event.org_unit_id,
                event.event_type.as_str(),
                event.occurred_at.to_rfc3339(),
                metadata_json,
            ],
        )?;
        Ok(())
    }

    fn query_events(
        &self,
        filter: &UsageEventFilter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AppUsageEvent>, Self::Error> {
        let event_type_str: Option<&str> = filter.event_type.as_ref().map(UsageEventType::as_str);
        let after_str: Option<String> = filter.occurred_after.map(|dt| dt.to_rfc3339());
        let before_str: Option<String> = filter.occurred_before.map(|dt| dt.to_rfc3339());

        let mut stmt = self.conn.prepare(
            "SELECT event_id, app_id, session_id, user_id, org_unit_id, \
                    event_type, occurred_at, metadata \
             FROM platform_app_usage_events \
             WHERE (?1 IS NULL OR app_id      = ?1) \
               AND (?2 IS NULL OR session_id  = ?2) \
               AND (?3 IS NULL OR user_id     = ?3) \
               AND (?4 IS NULL OR org_unit_id = ?4) \
               AND (?5 IS NULL OR event_type  = ?5) \
               AND (?6 IS NULL OR occurred_at >= ?6) \
               AND (?7 IS NULL OR occurred_at <  ?7) \
             ORDER BY occurred_at DESC \
             LIMIT ?8 OFFSET ?9",
        )?;

        let raw: Vec<(
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            String,
            String,
        )> = stmt
            .query_map(
                params![
                    filter.app_id.as_deref(),
                    filter.session_id.as_deref(),
                    filter.user_id.as_deref(),
                    filter.org_unit_id.as_deref(),
                    event_type_str,
                    after_str.as_deref(),
                    before_str.as_deref(),
                    limit as i64,
                    offset as i64,
                ],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        raw.into_iter()
            .map(|(ei, ai, si, ui, ou, et, oa, mj)| row_to_event(ei, ai, si, ui, ou, et, oa, mj))
            .collect()
    }

    fn stats_for_app(
        &self,
        app_id: &str,
        period: &UsagePeriod,
    ) -> Result<Option<AppUsageStats>, Self::Error> {
        let (start, end) = period_to_range(period);

        // Uma única query de agregação — elimina a count check prévia (3→2 round-trips).
        // Se não existirem eventos, total=0 e devolvemos None.
        let (total, opened, closed, failed, session_count, unique_users):
            (i64, i64, i64, i64, i64, i64) =
            self.conn.query_row(
                "SELECT \
                    COUNT(*)                                                           AS total, \
                    COUNT(*) FILTER (WHERE event_type = 'AppOpened')                  AS opened, \
                    COUNT(*) FILTER (WHERE event_type = 'AppClosed')                  AS closed, \
                    COUNT(*) FILTER (WHERE event_type = 'AppFailed')                  AS failed, \
                    COUNT(DISTINCT session_id) FILTER (WHERE event_type = 'AppOpened') AS sessions, \
                    COUNT(DISTINCT user_id)                                            AS unique_users \
                 FROM platform_app_usage_events \
                 WHERE app_id = ?1 AND occurred_at >= ?2 AND occurred_at < ?3",
                params![app_id, start, end],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )?;

        if total == 0 {
            return Ok(None);
        }

        // Duração média de sessão: subquery de MIN evita dupla contagem quando
        // existem múltiplos AppClosed para a mesma sessão.
        let avg_duration: Option<f64> = self.conn.query_row(
            "WITH first_close AS ( \
                SELECT session_id, MIN(occurred_at) AS occurred_at \
                FROM platform_app_usage_events \
                WHERE event_type = 'AppClosed' \
                GROUP BY session_id \
            ), \
            session_durations AS ( \
                SELECT (julianday(fc.occurred_at) - julianday(o.occurred_at)) * 86400.0 AS dur \
                FROM platform_app_usage_events o \
                JOIN first_close fc ON o.session_id = fc.session_id \
                WHERE o.app_id      = ?1 \
                  AND o.event_type  = 'AppOpened' \
                  AND o.occurred_at >= ?2 \
                  AND o.occurred_at <  ?3 \
            ) \
            SELECT AVG(dur) FROM session_durations",
            params![app_id, start, end],
            |row| row.get(0),
        )?;

        Ok(Some(AppUsageStats {
            app_id: app_id.to_owned(),
            period: period.clone(),
            opened_count: opened as u64,
            closed_count: closed as u64,
            failure_count: failed as u64,
            session_count: session_count as u64,
            unique_users: unique_users as u64,
            avg_session_duration_secs: avg_duration,
        }))
    }

    fn top_apps(
        &self,
        period: &UsagePeriod,
        limit: usize,
    ) -> Result<Vec<AppUsageStats>, Self::Error> {
        let (start, end) = period_to_range(period);

        let mut stmt = self.conn.prepare(
            "SELECT \
                app_id, \
                COUNT(*) FILTER (WHERE event_type = 'AppOpened')                   AS opened, \
                COUNT(*) FILTER (WHERE event_type = 'AppClosed')                   AS closed, \
                COUNT(*) FILTER (WHERE event_type = 'AppFailed')                   AS failed, \
                COUNT(DISTINCT session_id) FILTER (WHERE event_type = 'AppOpened') AS sessions, \
                COUNT(DISTINCT user_id)                                             AS unique_users \
             FROM platform_app_usage_events \
             WHERE occurred_at >= ?1 AND occurred_at < ?2 \
             GROUP BY app_id \
             ORDER BY opened DESC \
             LIMIT ?3",
        )?;

        let raw: Vec<(String, i64, i64, i64, i64, i64)> = stmt
            .query_map(params![start, end, limit as i64], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(raw
            .into_iter()
            .map(
                |(app_id, opened, closed, failed, sessions, users)| AppUsageStats {
                    app_id,
                    period: period.clone(),
                    opened_count: opened as u64,
                    closed_count: closed as u64,
                    failure_count: failed as u64,
                    session_count: sessions as u64,
                    unique_users: users as u64,
                    avg_session_duration_secs: None,
                },
            )
            .collect())
    }
}

// ─── Testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Duration, NaiveDate, Utc};
    use tempfile::tempdir;

    use super::*;
    use domain_telemetry::{TelemetryService, UsageEventFilter, UsagePeriod};

    fn open_tmp() -> (tempfile::TempDir, TelemetrySqliteStore) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("telemetry.db");
        let store =
            TelemetrySqliteStore::open(&SqliteRelationalConfig::read_write_create(&path)).unwrap();
        (dir, store)
    }

    fn event(
        event_id: &str,
        app_id: &str,
        session_id: &str,
        user_id: &str,
        event_type: UsageEventType,
        occurred_at: chrono::DateTime<Utc>,
    ) -> AppUsageEvent {
        AppUsageEvent {
            event_id: UsageEventId::new(event_id).unwrap(),
            app_id: app_id.to_owned(),
            session_id: SessionId::new(session_id).unwrap(),
            user_id: user_id.to_owned(),
            org_unit_id: None,
            event_type,
            occurred_at,
            metadata: HashMap::new(),
        }
    }

    fn jan2025() -> UsagePeriod {
        UsagePeriod::Month {
            year: 2025,
            month: 1,
        }
    }

    fn ts(offset_ms: i64) -> chrono::DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339("2025-01-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + Duration::milliseconds(offset_ms)
    }

    #[test]
    fn record_and_query_event() {
        let (_dir, store) = open_tmp();
        let e = event(
            "ev-1",
            "app-x",
            "sess-1",
            "user-a",
            UsageEventType::AppOpened,
            ts(0),
        );
        store.record(&e).unwrap();

        let result = store
            .query_events(&UsageEventFilter::default(), 10, 0)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_id.as_str(), "ev-1");
        assert_eq!(result[0].event_type, UsageEventType::AppOpened);
    }

    #[test]
    fn record_duplicate_event_id_is_idempotent() {
        let (_dir, store) = open_tmp();
        let e = event(
            "ev-dup",
            "app-x",
            "s1",
            "u1",
            UsageEventType::AppOpened,
            ts(0),
        );
        store.record(&e).unwrap();
        store.record(&e).unwrap(); // segunda vez — não deve errar

        let result = store
            .query_events(&UsageEventFilter::default(), 10, 0)
            .unwrap();
        assert_eq!(
            result.len(),
            1,
            "evento duplicado não deve ser inserido duas vezes"
        );
    }

    #[test]
    fn query_filter_by_app_id() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-y",
                "s2",
                "u1",
                UsageEventType::AppOpened,
                ts(1),
            ))
            .unwrap();

        let result = store
            .query_events(
                &UsageEventFilter {
                    app_id: Some("app-x".into()),
                    ..Default::default()
                },
                10,
                0,
            )
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].app_id, "app-x");
    }

    #[test]
    fn query_filter_by_session_id() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "sess-a",
                "u1",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "sess-a",
                "u1",
                UsageEventType::AppClosed,
                ts(100),
            ))
            .unwrap();
        store
            .record(&event(
                "e3",
                "app-x",
                "sess-b",
                "u1",
                UsageEventType::AppOpened,
                ts(200),
            ))
            .unwrap();

        let result = store
            .query_events(
                &UsageEventFilter {
                    session_id: Some("sess-a".into()),
                    ..Default::default()
                },
                10,
                0,
            )
            .unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.session_id.as_str() == "sess-a"));
    }

    #[test]
    fn query_filter_by_event_type() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppFailed,
                ts(1),
            ))
            .unwrap();

        let result = store
            .query_events(
                &UsageEventFilter {
                    event_type: Some(UsageEventType::AppFailed),
                    ..Default::default()
                },
                10,
                0,
            )
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_type, UsageEventType::AppFailed);
    }

    #[test]
    fn query_filter_by_user_id() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "alice",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s2",
                "bob",
                UsageEventType::AppOpened,
                ts(1),
            ))
            .unwrap();

        let result = store
            .query_events(
                &UsageEventFilter {
                    user_id: Some("alice".into()),
                    ..Default::default()
                },
                10,
                0,
            )
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_id, "alice");
    }

    #[test]
    fn query_filter_by_date_range() {
        let (_dir, store) = open_tmp();
        let base = chrono::DateTime::parse_from_rfc3339("2025-01-10T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                base,
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s2",
                "u1",
                UsageEventType::AppOpened,
                base + Duration::days(10),
            ))
            .unwrap();

        let result = store
            .query_events(
                &UsageEventFilter {
                    occurred_after: Some(base + Duration::days(5)),
                    ..Default::default()
                },
                10,
                0,
            )
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_id.as_str(), "e2");
    }

    #[test]
    fn query_limit_respected() {
        let (_dir, store) = open_tmp();
        for i in 0..5 {
            store
                .record(&event(
                    &format!("ev-{i}"),
                    "app-x",
                    &format!("s-{i}"),
                    "u1",
                    UsageEventType::AppOpened,
                    ts(i * 100),
                ))
                .unwrap();
        }
        assert_eq!(
            store
                .query_events(&UsageEventFilter::default(), 3, 0)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn query_pagination_with_offset() {
        let (_dir, store) = open_tmp();
        for i in 0..5i64 {
            store
                .record(&event(
                    &format!("ev-{i}"),
                    "app-x",
                    &format!("s-{i}"),
                    "u1",
                    UsageEventType::AppOpened,
                    ts(i * 100),
                ))
                .unwrap();
        }
        let page1 = store
            .query_events(&UsageEventFilter::default(), 2, 0)
            .unwrap();
        let page2 = store
            .query_events(&UsageEventFilter::default(), 2, 2)
            .unwrap();
        let page3 = store
            .query_events(&UsageEventFilter::default(), 2, 4)
            .unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);

        // Nenhum evento duplicado entre páginas.
        let all_ids: Vec<_> = page1
            .iter()
            .chain(&page2)
            .chain(&page3)
            .map(|e| e.event_id.as_str().to_owned())
            .collect();
        let unique: std::collections::HashSet<_> = all_ids.iter().collect();
        assert_eq!(
            all_ids.len(),
            unique.len(),
            "eventos duplicados entre páginas"
        );
    }

    #[test]
    fn stats_no_events_returns_none() {
        let (_dir, store) = open_tmp();
        assert!(store.stats_for_app("app-x", &jan2025()).unwrap().is_none());
    }

    #[test]
    fn stats_basic_counts() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "alice",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s1",
                "alice",
                UsageEventType::AppClosed,
                ts(100),
            ))
            .unwrap();
        store
            .record(&event(
                "e3",
                "app-x",
                "s2",
                "bob",
                UsageEventType::AppOpened,
                ts(200),
            ))
            .unwrap();
        store
            .record(&event(
                "e4",
                "app-x",
                "s2",
                "bob",
                UsageEventType::AppFailed,
                ts(300),
            ))
            .unwrap();

        let stats = store.stats_for_app("app-x", &jan2025()).unwrap().unwrap();
        assert_eq!(stats.opened_count, 2);
        assert_eq!(stats.closed_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.session_count, 2);
        assert_eq!(stats.unique_users, 2);
    }

    #[test]
    fn stats_avg_session_duration() {
        let (_dir, store) = open_tmp();
        let t0 = ts(0);
        // Sessão 1: 1 segundo
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                t0,
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppClosed,
                t0 + Duration::seconds(1),
            ))
            .unwrap();
        // Sessão 2: 3 segundos
        store
            .record(&event(
                "e3",
                "app-x",
                "s2",
                "u1",
                UsageEventType::AppOpened,
                ts(10000),
            ))
            .unwrap();
        store
            .record(&event(
                "e4",
                "app-x",
                "s2",
                "u1",
                UsageEventType::AppClosed,
                ts(10000) + Duration::seconds(3),
            ))
            .unwrap();

        let stats = store.stats_for_app("app-x", &jan2025()).unwrap().unwrap();
        let avg = stats.avg_session_duration_secs.unwrap();
        // Média de 1s e 3s = 2s
        assert!(
            (avg - 2.0).abs() < 0.1,
            "avg_duration esperado ~2s, obtido {avg:.3}s"
        );
    }

    #[test]
    fn stats_no_double_counting_with_multiple_close_events() {
        let (_dir, store) = open_tmp();
        let t0 = ts(0);
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                t0,
            ))
            .unwrap();
        // Dois AppClosed na mesma sessão (ex: crash + cleanup): só o primeiro deve contar.
        store
            .record(&event(
                "e2",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppClosed,
                t0 + Duration::seconds(2),
            ))
            .unwrap();
        store
            .record(&event(
                "e3",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppClosed,
                t0 + Duration::seconds(10),
            ))
            .unwrap();

        let stats = store.stats_for_app("app-x", &jan2025()).unwrap().unwrap();
        let avg = stats.avg_session_duration_secs.unwrap();
        // Só o primeiro close (2s) deve ser usado — não a média de 2s e 10s (=6s).
        assert!(
            (avg - 2.0).abs() < 0.1,
            "avg_duration esperado ~2s, obtido {avg:.3}s"
        );
    }

    #[test]
    fn stats_no_complete_sessions_duration_is_none() {
        let (_dir, store) = open_tmp();
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();

        let stats = store.stats_for_app("app-x", &jan2025()).unwrap().unwrap();
        assert!(stats.avg_session_duration_secs.is_none());
    }

    #[test]
    fn stats_out_of_period_ignored() {
        let (_dir, store) = open_tmp();
        let feb = chrono::DateTime::parse_from_rfc3339("2025-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                feb,
            ))
            .unwrap();

        assert!(store.stats_for_app("app-x", &jan2025()).unwrap().is_none());
    }

    #[test]
    fn top_apps_sorted_by_opens() {
        let (_dir, store) = open_tmp();
        for i in 0..3 {
            store
                .record(&event(
                    &format!("a{i}"),
                    "app-a",
                    &format!("sa{i}"),
                    "u1",
                    UsageEventType::AppOpened,
                    ts(i * 100),
                ))
                .unwrap();
        }
        store
            .record(&event(
                "b1",
                "app-b",
                "sb1",
                "u1",
                UsageEventType::AppOpened,
                ts(999),
            ))
            .unwrap();
        for i in 0..2 {
            store
                .record(&event(
                    &format!("c{i}"),
                    "app-c",
                    &format!("sc{i}"),
                    "u1",
                    UsageEventType::AppOpened,
                    ts(i * 100 + 5000),
                ))
                .unwrap();
        }

        let top = store.top_apps(&jan2025(), 10).unwrap();
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].app_id, "app-a");
        assert_eq!(top[0].opened_count, 3);
        assert_eq!(top[1].app_id, "app-c");
        assert_eq!(top[2].app_id, "app-b");
    }

    #[test]
    fn top_apps_includes_session_count() {
        let (_dir, store) = open_tmp();
        // 3 aberturas em 2 sessões distintas (uma sessão aberta 2 vezes — improvável mas válido)
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                ts(0),
            ))
            .unwrap();
        store
            .record(&event(
                "e2",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                ts(100),
            ))
            .unwrap();
        store
            .record(&event(
                "e3",
                "app-x",
                "s2",
                "u2",
                UsageEventType::AppOpened,
                ts(200),
            ))
            .unwrap();

        let top = store.top_apps(&jan2025(), 10).unwrap();
        assert_eq!(top[0].opened_count, 3);
        assert_eq!(top[0].session_count, 2, "sessões únicas por session_id");
    }

    #[test]
    fn metadata_roundtrip() {
        let (_dir, store) = open_tmp();
        let mut meta = HashMap::new();
        meta.insert("action".to_owned(), "criar_documento".to_owned());
        meta.insert("tipo_doc".to_owned(), "oficio".to_owned());

        let mut e = event(
            "ev-m",
            "app-x",
            "s1",
            "u1",
            UsageEventType::ActionCompleted,
            ts(0),
        );
        e.metadata = meta;
        store.record(&e).unwrap();

        let result = store
            .query_events(&UsageEventFilter::default(), 10, 0)
            .unwrap();
        assert_eq!(
            result[0].metadata.get("action").map(String::as_str),
            Some("criar_documento")
        );
        assert_eq!(
            result[0].metadata.get("tipo_doc").map(String::as_str),
            Some("oficio")
        );
    }

    #[test]
    fn service_validates_empty_app_id() {
        let (_dir, store) = open_tmp();
        let svc = TelemetryService::new(store);
        let mut e = event(
            "ev-1",
            "app-x",
            "s1",
            "u1",
            UsageEventType::AppOpened,
            ts(0),
        );
        e.app_id = String::new();
        assert!(svc
            .record_event(e)
            .unwrap_err()
            .to_string()
            .contains("app_id"));
    }

    #[test]
    fn top_apps_limit_respected() {
        let (_dir, store) = open_tmp();
        for i in 0..5i64 {
            store
                .record(&event(
                    &format!("ev-{i}"),
                    &format!("app-{i}"),
                    &format!("s-{i}"),
                    "u1",
                    UsageEventType::AppOpened,
                    ts(i * 100),
                ))
                .unwrap();
        }
        assert_eq!(store.top_apps(&jan2025(), 3).unwrap().len(), 3);
    }

    #[test]
    fn stats_day_period() {
        let (_dir, store) = open_tmp();
        let day_ts = chrono::DateTime::parse_from_rfc3339("2025-01-15T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store
            .record(&event(
                "e1",
                "app-x",
                "s1",
                "u1",
                UsageEventType::AppOpened,
                day_ts,
            ))
            .unwrap();

        let stats = store
            .stats_for_app(
                "app-x",
                &UsagePeriod::Day(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            )
            .unwrap()
            .unwrap();
        assert_eq!(stats.opened_count, 1);

        assert!(store
            .stats_for_app(
                "app-x",
                &UsagePeriod::Day(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap())
            )
            .unwrap()
            .is_none());
    }
}

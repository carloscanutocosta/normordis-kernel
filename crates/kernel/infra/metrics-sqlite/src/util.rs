use chrono::{DateTime, Utc};
use core_metrics::ListOptions;

use crate::error::MetricsSqliteError;

pub fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

pub fn str_to_dt(s: &str) -> Result<DateTime<Utc>, MetricsSqliteError> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3fZ"))
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| MetricsSqliteError::InvalidDateTime(s.to_string()))
}

pub fn limit_offset(opts: &ListOptions) -> String {
    if opts.limit == 0 {
        format!(" LIMIT -1 OFFSET {}", opts.offset)
    } else {
        format!(" LIMIT {} OFFSET {}", opts.limit, opts.offset)
    }
}

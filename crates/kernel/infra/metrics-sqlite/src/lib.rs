#![allow(clippy::redundant_closure, clippy::result_large_err)]

mod error;
mod schema;
mod store_changelog;
mod store_cycles;
mod store_definitions;
mod store_events;
mod store_instances;
mod store_results;
mod store_targets;
mod store_versions;
mod util;

pub use error::MetricsSqliteError;
pub use schema::METRICS_SQLITE_MIGRATIONS;

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use rusqlite::Connection;
use std::sync::{Mutex, MutexGuard};

/// Façade que agrega todos os stores de métricas numa única conexão SQLite.
///
/// Implements:
/// - `MetricStore` (eventos operacionais)
/// - `MetricDefinitionStore`
/// - `MetricVersionStore`
/// - `TargetDefinitionStore`
/// - `EvaluationCycleStore`
/// - `IndicatorInstanceStore`
/// - `MeasurementResultStore`
/// - `GovernanceChangeLog`
#[derive(Debug)]
pub struct MetricsSqliteStore {
    conn: Mutex<Connection>,
}

impl MetricsSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, MetricsSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, MetricsSqliteError> {
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), MetricsSqliteError> {
        let conn = self.db();
        run_relational_migrations(&conn, METRICS_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    pub(crate) fn db(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("metrics-sqlite mutex poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use tempfile::NamedTempFile;

    fn test_store() -> MetricsSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        MetricsSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    #[test]
    fn migrations_run_without_error() {
        let _store = test_store();
    }
}

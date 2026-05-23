//! Adapters de infraestrutura para exportacao tabular do Mini-Kernel RS.
//!
//! Este crate materializa pedidos tecnicos de export em SQLite, CSV, XML e
//! XLSX. A semantica institucional do snapshot continua em `core-exports`;
//! aqui ficam apenas os formatos concretos e o runtime headless.

mod config;
mod csv;
mod error;
mod model;
mod runtime;
mod sqlite;
mod xlsx;
mod xml;

pub use config::{Config, Provider};
pub use csv::CsvExportAdapter;
pub use error::{ExportAdapterError, Result};
pub use model::{
    build_single_artefact_plan, payload_bytes, sha256_hex, Artefact, ExportRequest, ExportResult,
    ExportRow, Plan,
};
pub use runtime::{Exporter, RuntimeBinding, RuntimeExporter};
pub use sqlite::SqliteExportAdapter;
pub use xlsx::XlsxExportAdapter;
pub use xml::XmlExportAdapter;

#[cfg(test)]
mod tests;

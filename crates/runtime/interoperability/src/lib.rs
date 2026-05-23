//! Orquestracao headless substituivel para interoperabilidade institucional.
//!
//! Este crate constroi pedidos de `core-exports`, aplica autorizacao injetavel e
//! chama um `ExportMaterializerPort`. Nao conhece filesystem, Tauri, SQLite,
//! XLSX, UI ou adapters concretos.

mod authorization;
mod builder;
mod error;
mod service;

pub use authorization::{
    AllowAllExportAuthorization, DenyAllExportAuthorization, ExportAuthorizationContext,
    ExportAuthorizationPolicy,
};
pub use builder::ExportRequestBuilder;
pub use error::{InteroperabilityError, Result};
pub use service::InteroperabilityExportService;

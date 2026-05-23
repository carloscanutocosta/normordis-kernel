pub mod error;
pub mod ports;
pub mod types;

pub use error::MefError;
pub use ports::MefRepository;
pub use types::{DiplomaRef, MefCode, MefEntry, UpsertMefEntryRequest};

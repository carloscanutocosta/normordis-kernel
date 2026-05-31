pub mod error;
pub mod ports;
pub mod service;
pub mod types;

pub use error::RegistryError;
pub use ports::AppRegistryRepository;
pub use service::AppRegistryService;
pub use types::{
    AppId, AppRegistration, AppRegistryFilter, AppState, AppStateTransition, AppVisibility,
    RegisterAppRequest, RoleId, TransitionStateRequest, UpdateAppMetadataRequest,
};

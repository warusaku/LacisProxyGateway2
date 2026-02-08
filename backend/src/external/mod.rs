//! External device integration module
//!
//! - `mercury`: Mercury AC HTTP JSON client
//! - `manager`: Multi-device lifecycle management
//! - `sync`: Background polling synchronization

pub mod manager;
pub mod mercury;
pub mod sync;

pub use manager::ExternalDeviceManager;
pub use sync::ExternalSyncer;

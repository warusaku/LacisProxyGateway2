//! OpenWrt/AsusWrt SSH integration module
//!
//! - `client`: SSH-based router data retrieval
//! - `manager`: Multi-router lifecycle management
//! - `sync`: Background polling synchronization

pub mod client;
pub mod manager;
pub mod sync;

pub use manager::OpenWrtManager;
pub use sync::OpenWrtSyncer;

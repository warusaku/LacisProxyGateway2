//! Omada OpenAPI integration module
//!
//! - `client`: Low-level API client (token management, HTTP requests)
//! - `manager`: Multi-controller lifecycle management
//! - `sync`: Background data synchronization

pub mod client;
pub mod manager;
pub mod sync;

pub use client::OmadaClient;
pub use manager::OmadaManager;
pub use sync::OmadaSyncer;

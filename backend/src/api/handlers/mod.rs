//! HTTP handlers module

pub mod agent;
pub mod auth;
pub mod aranea;
mod audit;
mod dashboard;
mod ddns;
pub mod external;
mod lacis_id;
mod diagnostics;
mod nginx;
mod omada;
pub mod openwrt;
mod routes;
mod security;
mod settings;
mod tools;
mod topology;
pub mod wireguard;

pub use self::agent::*;
pub use self::aranea::*;
pub use self::audit::*;
pub use self::dashboard::*;
pub use self::ddns::*;
pub use self::diagnostics::*;
pub use self::lacis_id::*;
pub use self::nginx::*;
pub use self::omada::*;
pub use self::routes::*;
pub use self::security::*;
pub use self::settings::*;
pub use self::tools::*;
pub use self::topology::*;

use axum::{response::IntoResponse, Json};
use serde::Serialize;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Health check handler
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "LacisProxyGateway2".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Generic success response
#[derive(Serialize)]
pub struct SuccessResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
}

impl SuccessResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            id: None,
        }
    }

    pub fn with_id(message: impl Into<String>, id: i32) -> Self {
        Self {
            message: message.into(),
            id: Some(id),
        }
    }
}

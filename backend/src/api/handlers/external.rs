//! External device API handlers
//!
//! Device management (CRUD), client viewing, connection testing, manual polling.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::external::ExternalDeviceManager;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
use crate::proxy::ProxyState;

// ============================================================================
// Request types
// ============================================================================

#[derive(Deserialize)]
pub struct RegisterDeviceRequest {
    pub display_name: String,
    pub mac: String,
    pub ip: String,
    pub protocol: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct TestDeviceRequest {
    pub ip: String,
    pub protocol: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ExternalClientQuery {
    pub device_id: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/external/devices - Register a new device (admin: permission >= 80)
pub async fn register_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    match state
        .external_manager
        .register_device(
            &req.display_name,
            &req.mac,
            &req.ip,
            &req.protocol,
            req.username.as_deref(),
            req.password.as_deref(),
        )
        .await
    {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "device": doc,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// GET /api/external/devices - List all devices
pub async fn list_devices(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.list_external_devices().await {
        Ok(devices) => Json(serde_json::json!({
            "ok": true,
            "devices": devices,
            "total": devices.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/external/devices/:id - Get a single device
pub async fn get_device(
    State(state): State<ProxyState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_external_device(&id).await {
        Ok(Some(device)) => Json(serde_json::json!({
            "ok": true,
            "device": device,
        })),
        Ok(None) => Json(serde_json::json!({
            "ok": false,
            "error": "Device not found",
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// DELETE /api/external/devices/:id - Remove a device
pub async fn delete_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Confirm guard
    if !confirm.confirm {
        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_device".to_string(),
            target: format!("external device {}", id),
            warning: "This will remove the external device and all synced client data.".to_string(),
            confirm_required: true,
        })));
    }

    match state.external_manager.remove_device(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Device {} removed", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// POST /api/external/devices/test - Test device connection
pub async fn test_device_connection(Json(req): Json<TestDeviceRequest>) -> Json<serde_json::Value> {
    match ExternalDeviceManager::test_connection(
        &req.ip,
        &req.protocol,
        req.username.as_deref(),
        req.password.as_deref(),
    )
    .await
    {
        Ok(result) => Json(result),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e,
        })),
    }
}

/// POST /api/external/devices/:id/poll - Manual poll (operate: permission >= 50)
pub async fn poll_device(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let syncer = crate::external::ExternalSyncer::new(
        state.external_manager.clone(),
        state.app_state.mongo.clone(),
    );

    match syncer.poll_one(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Device {} polled", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// GET /api/external/clients - All clients
pub async fn get_external_clients(
    State(state): State<ProxyState>,
    Query(q): Query<ExternalClientQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_external_clients(q.device_id.as_deref())
        .await
    {
        Ok(clients) => Json(serde_json::json!({
            "ok": true,
            "clients": clients,
            "total": clients.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/external/summary - Summary statistics
pub async fn get_external_summary(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_external_summary().await {
        Ok(summary) => Json(serde_json::json!({
            "ok": true,
            "summary": summary,
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

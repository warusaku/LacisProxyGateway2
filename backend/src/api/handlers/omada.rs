//! Omada API handlers
//!
//! Controller management (CRUD), data viewing, sync triggers, and legacy compatibility.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
use crate::omada::manager::OmadaManager;
use crate::omada::OmadaClient;
use crate::proxy::ProxyState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Deserialize)]
pub struct RegisterControllerRequest {
    pub display_name: String,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize)]
pub struct TestConnectionRequest {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize, Default)]
pub struct OmadaDeviceQuery {
    pub controller_id: Option<String>,
    pub site_id: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct OmadaClientQuery {
    pub controller_id: Option<String>,
    pub site_id: Option<String>,
    pub active: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct OmadaWgQuery {
    pub controller_id: Option<String>,
    pub site_id: Option<String>,
}

// ============================================================================
// Controller management
// ============================================================================

/// POST /api/omada/controllers - Register a new controller (admin: permission >= 80)
pub async fn register_controller(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<RegisterControllerRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    match state
        .omada_manager
        .register_controller(
            &req.display_name,
            &req.base_url,
            &req.client_id,
            &req.client_secret,
        )
        .await
    {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "controller": doc,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// GET /api/omada/controllers - List all controllers
pub async fn list_controllers(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.list_omada_controllers().await {
        Ok(controllers) => Json(serde_json::json!({
            "ok": true,
            "controllers": controllers,
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/omada/controllers/:id - Get a single controller
pub async fn get_controller(
    State(state): State<ProxyState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_omada_controller(&id).await {
        Ok(Some(ctrl)) => Json(serde_json::json!({
            "ok": true,
            "controller": ctrl,
        })),
        Ok(None) => Json(serde_json::json!({
            "ok": false,
            "error": "Controller not found",
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// DELETE /api/omada/controllers/:id - Remove a controller (dangerous: permission == 100, confirm required)
pub async fn delete_controller(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Confirm guard
    if !confirm.confirm {
        let ctrl = state
            .app_state
            .mongo
            .get_omada_controller(&id)
            .await
            .ok()
            .flatten();
        let target_info = ctrl
            .map(|c| format!("Omada controller '{}' ({})", c.display_name, id))
            .unwrap_or_else(|| format!("Omada controller {}", id));

        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_controller".to_string(),
            target: target_info,
            warning: "This will remove the Omada controller and all synced device/client data."
                .to_string(),
            confirm_required: true,
        })));
    }

    match state.omada_manager.remove_controller(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Controller {} removed", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// POST /api/omada/controllers/test - Test connection (pre-registration)
pub async fn test_controller_connection(
    Json(req): Json<TestConnectionRequest>,
) -> Json<serde_json::Value> {
    let result =
        OmadaManager::test_connection(&req.base_url, &req.client_id, &req.client_secret).await;
    Json(serde_json::to_value(result).unwrap_or_default())
}

/// POST /api/omada/controllers/:id/sync - Manual sync trigger (operate: permission >= 50)
pub async fn sync_controller(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let syncer =
        crate::omada::OmadaSyncer::new(state.omada_manager.clone(), state.app_state.mongo.clone());

    match syncer.sync_one(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Controller {} synced", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

// ============================================================================
// Data viewing (from MongoDB)
// ============================================================================

/// GET /api/omada/devices - All devices
pub async fn get_omada_devices(
    State(state): State<ProxyState>,
    Query(q): Query<OmadaDeviceQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_omada_devices(q.controller_id.as_deref(), q.site_id.as_deref())
        .await
    {
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

/// GET /api/omada/clients - All clients
pub async fn get_omada_clients(
    State(state): State<ProxyState>,
    Query(q): Query<OmadaClientQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_omada_clients(q.controller_id.as_deref(), q.site_id.as_deref(), q.active)
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

/// GET /api/omada/wireguard - All WireGuard peers
pub async fn get_omada_wireguard(
    State(state): State<ProxyState>,
    Query(q): Query<OmadaWgQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_omada_wg_peers(q.controller_id.as_deref(), q.site_id.as_deref())
        .await
    {
        Ok(peers) => Json(serde_json::json!({
            "ok": true,
            "peers": peers,
            "total": peers.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/omada/summary - Aggregated summary
pub async fn get_omada_summary(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_omada_summary().await {
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

// ============================================================================
// Legacy compatibility (backward compatible with existing frontend)
// ============================================================================

/// GET /api/omada/status - Legacy: first controller's network status
pub async fn get_network_status(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    // Use first registered controller, or fall back to MySQL-based client
    let ids = state.omada_manager.list_controller_ids().await;

    if let Some(first_id) = ids.first() {
        if let Some(client) = state.omada_manager.get_client(first_id).await {
            let status = client.get_network_status().await;
            return Json(serde_json::to_value(status).unwrap_or_default());
        }
    }

    // Fallback: legacy MySQL-based client
    let client = OmadaClient::new(state.app_state.mysql.clone());
    let status = client.get_network_status().await;
    Json(serde_json::to_value(status).unwrap_or_default())
}

/// POST /api/omada/test - Legacy: first controller's connection test
pub async fn test_connection(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    let client = OmadaClient::new(state.app_state.mysql.clone());

    match client.load_config().await {
        Ok(true) => match client.get_devices().await {
            Ok(devices) => Json(serde_json::json!({
                "success": true,
                "message": format!("Connected! Found {} devices", devices.len()),
                "devices": devices.len()
            })),
            Err(e) => Json(serde_json::json!({
                "success": false,
                "message": format!("Connection failed: {}", e)
            })),
        },
        Ok(false) => Json(serde_json::json!({
            "success": false,
            "message": "Omada not configured"
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": format!("Config load failed: {}", e)
        })),
    }
}

//! OpenWrt API handlers
//!
//! Router management (CRUD), client viewing, SSH connection testing, manual polling.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired};
use crate::openwrt::OpenWrtManager;
use crate::proxy::ProxyState;

// ============================================================================
// Request types
// ============================================================================

#[derive(Deserialize)]
pub struct RegisterRouterRequest {
    pub display_name: String,
    pub mac: String,
    pub ip: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
    pub firmware: String,
}

#[derive(Deserialize)]
pub struct TestRouterRequest {
    pub ip: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
    pub firmware: String,
}

#[derive(Deserialize, Default)]
pub struct OpenWrtClientQuery {
    pub router_id: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/openwrt/routers - Register a new router (admin: permission >= 80)
pub async fn register_router(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<RegisterRouterRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    match state
        .openwrt_manager
        .register_router(
            &req.display_name,
            &req.mac,
            &req.ip,
            req.port.unwrap_or(22),
            &req.username,
            &req.password,
            &req.firmware,
        )
        .await
    {
        Ok(doc) => Ok(Json(serde_json::json!({
            "ok": true,
            "router": doc,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// GET /api/openwrt/routers - List all routers
pub async fn list_routers(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.list_openwrt_routers().await {
        Ok(routers) => Json(serde_json::json!({
            "ok": true,
            "routers": routers,
            "total": routers.len(),
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// GET /api/openwrt/routers/:id - Get a single router
pub async fn get_router(
    State(state): State<ProxyState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_openwrt_router(&id).await {
        Ok(Some(router)) => Json(serde_json::json!({
            "ok": true,
            "router": router,
        })),
        Ok(None) => Json(serde_json::json!({
            "ok": false,
            "error": "Router not found",
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e,
        })),
    }
}

/// DELETE /api/openwrt/routers/:id - Remove a router
pub async fn delete_router(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Confirm guard
    if !confirm.confirm {
        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_router".to_string(),
            target: format!("OpenWrt router {}", id),
            warning: "This will remove the OpenWrt router and all synced client data.".to_string(),
            confirm_required: true,
        })));
    }

    match state.openwrt_manager.remove_router(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Router {} removed", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// POST /api/openwrt/routers/test - Test SSH connection
pub async fn test_router_connection(Json(req): Json<TestRouterRequest>) -> Json<serde_json::Value> {
    match OpenWrtManager::test_connection(
        &req.ip,
        req.port.unwrap_or(22),
        &req.username,
        &req.password,
        &req.firmware,
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

/// POST /api/openwrt/routers/:id/poll - Manual poll (operate: permission >= 50)
pub async fn poll_router(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let syncer = crate::openwrt::OpenWrtSyncer::new(
        state.openwrt_manager.clone(),
        state.app_state.mongo.clone(),
    );

    match syncer.poll_one(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "ok": true,
            "message": format!("Router {} polled", id),
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "ok": false,
            "error": e,
        }))),
    }
}

/// GET /api/openwrt/clients - All clients
pub async fn get_openwrt_clients(
    State(state): State<ProxyState>,
    Query(q): Query<OpenWrtClientQuery>,
) -> Json<serde_json::Value> {
    match state
        .app_state
        .mongo
        .get_openwrt_clients(q.router_id.as_deref())
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

/// GET /api/openwrt/summary - Summary statistics
pub async fn get_openwrt_summary(State(state): State<ProxyState>) -> Json<serde_json::Value> {
    match state.app_state.mongo.get_openwrt_summary().await {
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

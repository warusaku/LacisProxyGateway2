//! Operation tools API handlers - sync triggers, network diagnostics

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::db::mongo::{OperationLogQuery, OperatorInfo};
use crate::error::AppError;
use crate::models::AuthUser;
use crate::proxy::ProxyState;

/// Helper to build OperatorInfo from AuthUser
fn operator_from(user: &AuthUser) -> OperatorInfo {
    OperatorInfo {
        sub: user.sub.clone(),
        auth_method: user.auth_method.clone(),
        permission: user.permission,
    }
}

/// POST /api/tools/sync/omada - Manual Omada sync trigger (operate: permission >= 50)
pub async fn tool_sync_omada(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let op_id = state
        .app_state
        .mongo
        .start_operation_log_with_operator("sync_omada", "api", None, Some(operator_from(&user)))
        .await
        .unwrap_or_default();

    let start = std::time::Instant::now();

    let syncer = crate::omada::OmadaSyncer::new(
        state.omada_manager.clone(),
        state.app_state.mongo.clone(),
    );

    let controller_ids = state.omada_manager.list_controller_ids().await;
    let mut results = Vec::new();

    for ctrl_id in &controller_ids {
        match syncer.sync_one(ctrl_id).await {
            Ok(()) => results.push(serde_json::json!({ "controller_id": ctrl_id, "status": "ok" })),
            Err(e) => results.push(serde_json::json!({ "controller_id": ctrl_id, "status": "error", "error": e })),
        }
    }

    let duration = start.elapsed().as_millis() as u64;

    if !op_id.is_empty() {
        let _ = state
            .app_state
            .mongo
            .complete_operation_log(&op_id, Some(&serde_json::json!({ "results": results })), duration)
            .await;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "controllers_synced": controller_ids.len(),
        "results": results,
        "duration_ms": duration,
    })))
}

/// POST /api/tools/sync/openwrt - Manual OpenWrt sync trigger (operate: permission >= 50)
pub async fn tool_sync_openwrt(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let op_id = state
        .app_state
        .mongo
        .start_operation_log_with_operator("sync_openwrt", "api", None, Some(operator_from(&user)))
        .await
        .unwrap_or_default();

    let start = std::time::Instant::now();

    let syncer = crate::openwrt::OpenWrtSyncer::new(
        state.openwrt_manager.clone(),
        state.app_state.mongo.clone(),
    );

    let router_ids = state.openwrt_manager.list_router_ids().await;
    let mut ok_count = 0u32;
    let mut err_count = 0u32;

    for rid in &router_ids {
        match syncer.poll_one(rid).await {
            Ok(()) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    if !op_id.is_empty() {
        let _ = state.app_state.mongo.complete_operation_log(
            &op_id,
            Some(&serde_json::json!({ "ok": ok_count, "errors": err_count })),
            duration,
        ).await;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "polled": ok_count,
        "errors": err_count,
        "duration_ms": duration,
    })))
}

/// POST /api/tools/sync/external - Manual External sync trigger (operate: permission >= 50)
pub async fn tool_sync_external(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let op_id = state
        .app_state
        .mongo
        .start_operation_log_with_operator("sync_external", "api", None, Some(operator_from(&user)))
        .await
        .unwrap_or_default();

    let start = std::time::Instant::now();

    let syncer = crate::external::ExternalSyncer::new(
        state.external_manager.clone(),
        state.app_state.mongo.clone(),
    );

    let device_ids = state.external_manager.list_device_ids().await;
    let mut ok_count = 0u32;
    let mut err_count = 0u32;

    for did in &device_ids {
        match syncer.poll_one(did).await {
            Ok(()) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    if !op_id.is_empty() {
        let _ = state.app_state.mongo.complete_operation_log(
            &op_id,
            Some(&serde_json::json!({ "ok": ok_count, "errors": err_count })),
            duration,
        ).await;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "polled": ok_count,
        "errors": err_count,
        "duration_ms": duration,
    })))
}

/// POST /api/tools/ddns/update-all - Manual DDNS update for all configs (operate: permission >= 50)
pub async fn tool_ddns_update_all(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let op_id = state
        .app_state
        .mongo
        .start_operation_log_with_operator("ddns_update_all", "api", None, Some(operator_from(&user)))
        .await
        .unwrap_or_default();

    let start = std::time::Instant::now();

    let configs = state.app_state.mysql.list_active_ddns().await
        .map_err(|e| AppError::InternalError(format!("Failed to list DDNS configs: {}", e)))?;

    let mut ok_count = 0u32;
    let mut err_count = 0u32;

    for config in &configs {
        match state.ddns_updater.update_single(config.id).await {
            Ok(()) => ok_count += 1,
            Err(_) => err_count += 1,
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    if !op_id.is_empty() {
        let _ = state.app_state.mongo.complete_operation_log(
            &op_id,
            Some(&serde_json::json!({ "ok": ok_count, "errors": err_count, "total": configs.len() })),
            duration,
        ).await;
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "updated": ok_count,
        "errors": err_count,
        "total": configs.len(),
        "duration_ms": duration,
    })))
}

#[derive(Debug, Deserialize)]
pub struct PingRequest {
    pub host: String,
}

/// POST /api/tools/network/ping - Ping a host from server (operate: permission >= 50)
pub async fn tool_network_ping(
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<PingRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    // Validate hostname (prevent command injection)
    if payload.host.contains(';') || payload.host.contains('|') || payload.host.contains('&')
        || payload.host.contains('$') || payload.host.contains('`') || payload.host.contains('\n')
    {
        return Err(AppError::BadRequest("Invalid hostname".to_string()));
    }

    let output = tokio::process::Command::new("ping")
        .args(["-c", "3", "-W", "5", &payload.host])
        .output()
        .await
        .map_err(|e| AppError::InternalError(format!("Ping failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(Json(serde_json::json!({
        "host": payload.host,
        "success": output.status.success(),
        "stdout": stdout,
        "stderr": stderr,
    })))
}

#[derive(Debug, Deserialize)]
pub struct DnsRequest {
    pub hostname: String,
}

/// POST /api/tools/network/dns - DNS lookup (operate: permission >= 50)
pub async fn tool_network_dns(
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<DnsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    match tokio::net::lookup_host(format!("{}:0", payload.hostname)).await {
        Ok(addrs) => {
            let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
            Ok(Json(serde_json::json!({
                "hostname": payload.hostname,
                "resolved": true,
                "addresses": ips,
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "hostname": payload.hostname,
            "resolved": false,
            "error": e.to_string(),
        }))),
    }
}

/// GET /api/logs/operations - List operation logs
pub async fn list_operation_logs(
    State(state): State<ProxyState>,
    Query(query): Query<OperationLogQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = state
        .app_state
        .mongo
        .query_operation_logs(&query)
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(logs))
}

/// GET /api/logs/operations/summary - Operation logs summary
pub async fn get_operation_logs_summary(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let summary = state
        .app_state
        .mongo
        .get_operation_log_summary()
        .await
        .map_err(|e| AppError::InternalError(e))?;

    Ok(Json(summary))
}

//! Security handlers (blocked IPs, security events)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, BlockIpRequest, ConfirmQuery, ConfirmRequired, SecurityEventSearchQuery};
use crate::proxy::ProxyState;

use super::SuccessResponse;

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/security/blocked-ips - List all blocked IPs
pub async fn list_blocked_ips(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let ips = state.app_state.mysql.list_blocked_ips().await?;
    Ok(Json(ips))
}

/// POST /api/security/blocked-ips - Block an IP address (admin: permission >= 80)
pub async fn block_ip(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<BlockIpRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Validate IP format (basic check)
    if payload.ip.is_empty() {
        return Err(AppError::BadRequest("IP address is required".to_string()));
    }

    // Check if already blocked
    if state.app_state.mysql.is_ip_blocked(&payload.ip).await? {
        return Err(AppError::BadRequest(format!(
            "IP {} is already blocked",
            payload.ip
        )));
    }

    let id = state.app_state.mysql.block_ip(&payload, "manual").await?;

    // Log security event
    state
        .app_state
        .mongo
        .log_ip_blocked(
            &payload.ip,
            payload.reason.as_deref().unwrap_or("Manual block"),
            crate::models::Severity::Medium,
        )
        .await?;

    tracing::warn!("Blocked IP: {}", payload.ip);

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::with_id("IP blocked", id)),
    ))
}

/// DELETE /api/security/blocked-ips/:id - Unblock an IP (dangerous: permission == 100, confirm required)
pub async fn unblock_ip(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Get the IP before deleting for logging
    let blocked = state.app_state.mysql.get_blocked_ip(id).await?;

    // Confirm guard
    if !confirm.confirm {
        let target_info = blocked
            .as_ref()
            .map(|b| format!("blocked IP #{} ({})", id, b.ip))
            .unwrap_or_else(|| format!("blocked IP #{}", id));

        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "unblock_ip".to_string(),
            target: target_info,
            warning: "This will unblock the IP address, allowing it to access the system again.".to_string(),
            confirm_required: true,
        })));
    }

    let deleted = state.app_state.mysql.unblock_ip(id).await?;

    if deleted {
        if let Some(b) = &blocked {
            tracing::info!("Unblocked IP: {}", b.ip);
        }
        Ok(Json(serde_json::json!(SuccessResponse::new("IP unblocked"))))
    } else {
        Err(AppError::NotFound(format!("Blocked IP {} not found", id)))
    }
}

/// GET /api/security/events - List security events
pub async fn list_security_events(
    State(state): State<ProxyState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let events = state
        .app_state
        .mongo
        .get_security_events(pagination.limit, pagination.offset)
        .await?;

    Ok(Json(events))
}

/// GET /api/security/events/ip/:ip - Get security events for an IP
pub async fn get_security_events_by_ip(
    State(state): State<ProxyState>,
    Path(ip): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let events = state
        .app_state
        .mongo
        .get_security_events_by_ip(&ip, 100)
        .await?;
    Ok(Json(events))
}

/// GET /api/security/events/search - Advanced security event search
pub async fn search_security_events(
    State(state): State<ProxyState>,
    Query(query): Query<SecurityEventSearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let events = state
        .app_state
        .mongo
        .search_security_events(&query)
        .await?;
    Ok(Json(events))
}

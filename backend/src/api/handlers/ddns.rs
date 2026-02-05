//! DDNS configuration handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::error::AppError;
use crate::models::{CreateDdnsRequest, DdnsProvider, UpdateDdnsRequest};
use crate::proxy::ProxyState;

use super::SuccessResponse;

/// GET /api/ddns - List all DDNS configurations
pub async fn list_ddns(State(state): State<ProxyState>) -> Result<impl IntoResponse, AppError> {
    let configs = state.app_state.mysql.list_ddns().await?;

    // Mask sensitive fields for response
    let masked: Vec<_> = configs
        .into_iter()
        .map(|mut c| {
            c.password = c.password.as_ref().map(|_| "********".to_string());
            c.api_token = c.api_token.as_ref().map(|_| "********".to_string());
            c
        })
        .collect();

    Ok(Json(masked))
}

/// GET /api/ddns/:id - Get a single DDNS configuration
pub async fn get_ddns(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let mut config = state
        .app_state
        .mysql
        .get_ddns(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("DDNS config {} not found", id)))?;

    // Mask sensitive fields
    config.password = config.password.as_ref().map(|_| "********".to_string());
    config.api_token = config.api_token.as_ref().map(|_| "********".to_string());

    Ok(Json(config))
}

/// POST /api/ddns - Create a new DDNS configuration
pub async fn create_ddns(
    State(state): State<ProxyState>,
    Json(payload): Json<CreateDdnsRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate based on provider
    match payload.provider {
        DdnsProvider::DynDns | DdnsProvider::NoIp => {
            if payload.username.is_none() || payload.password.is_none() {
                return Err(AppError::BadRequest(
                    "Username and password required for DynDNS/No-IP".to_string(),
                ));
            }
        }
        DdnsProvider::Cloudflare => {
            if payload.api_token.is_none() || payload.zone_id.is_none() {
                return Err(AppError::BadRequest(
                    "API token and zone ID required for Cloudflare".to_string(),
                ));
            }
        }
    }

    let id = state.app_state.mysql.create_ddns(&payload).await?;

    tracing::info!(
        "Created DDNS config: {} ({:?})",
        payload.hostname,
        payload.provider
    );

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::with_id("DDNS configuration created", id)),
    ))
}

/// PUT /api/ddns/:id - Update a DDNS configuration
pub async fn update_ddns(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDdnsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let updated = state.app_state.mysql.update_ddns(id, &payload).await?;

    if updated {
        tracing::info!("Updated DDNS config {}", id);
        Ok(Json(SuccessResponse::new("DDNS configuration updated")))
    } else {
        Err(AppError::NotFound(format!("DDNS config {} not found", id)))
    }
}

/// DELETE /api/ddns/:id - Delete a DDNS configuration
pub async fn delete_ddns(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = state.app_state.mysql.delete_ddns(id).await?;

    if deleted {
        tracing::info!("Deleted DDNS config {}", id);
        Ok(Json(SuccessResponse::new("DDNS configuration deleted")))
    } else {
        Err(AppError::NotFound(format!("DDNS config {} not found", id)))
    }
}

/// POST /api/ddns/:id/update - Trigger manual DDNS update
pub async fn trigger_ddns_update(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let config = state
        .app_state
        .mysql
        .get_ddns(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("DDNS config {} not found", id)))?;

    // Trigger actual DDNS update via ddns module
    tracing::info!("Manual DDNS update triggered for {}", config.hostname);

    state
        .ddns_updater
        .update_single(id)
        .await
        .map_err(|e| AppError::InternalError(format!("DDNS update failed: {}", e)))?;

    Ok(Json(SuccessResponse::new("DDNS update completed successfully")))
}

//! DDNS configuration handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired, CreateDdnsRequest, DdnsProvider, LinkOmadaRequest, UpdateDdnsRequest};
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

/// POST /api/ddns - Create a new DDNS configuration (admin: permission >= 80)
pub async fn create_ddns(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<CreateDdnsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

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

/// PUT /api/ddns/:id - Update a DDNS configuration (admin: permission >= 80)
pub async fn update_ddns(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDdnsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let updated = state.app_state.mysql.update_ddns(id, &payload).await?;

    if updated {
        tracing::info!("Updated DDNS config {}", id);
        Ok(Json(SuccessResponse::new("DDNS configuration updated")))
    } else {
        Err(AppError::NotFound(format!("DDNS config {} not found", id)))
    }
}

/// DELETE /api/ddns/:id - Delete a DDNS configuration (dangerous: permission == 100, confirm required)
pub async fn delete_ddns(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Confirm guard
    if !confirm.confirm {
        let config = state.app_state.mysql.get_ddns(id).await?;
        let target_info = config
            .map(|c| format!("DDNS #{} ({})", id, c.hostname))
            .unwrap_or_else(|| format!("DDNS #{}", id));

        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_ddns".to_string(),
            target: target_info,
            warning: "This will remove the DDNS configuration. DNS updates will stop.".to_string(),
            confirm_required: true,
        })));
    }

    let deleted = state.app_state.mysql.delete_ddns(id).await?;

    if deleted {
        tracing::info!("Deleted DDNS config {}", id);
        Ok(Json(serde_json::json!(SuccessResponse::new("DDNS configuration deleted"))))
    } else {
        Err(AppError::NotFound(format!("DDNS config {} not found", id)))
    }
}

/// POST /api/ddns/:id/update - Trigger manual DDNS update (operate: permission >= 50)
pub async fn trigger_ddns_update(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

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

/// GET /api/ddns/integrated - List DDNS configs with Omada WAN IP comparison
pub async fn list_ddns_integrated(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let configs = state.app_state.mysql.list_ddns().await?;
    let controllers = state.app_state.mongo.list_omada_controllers().await.unwrap_or_default();

    let mut results = Vec::new();

    for config in configs {
        let mut omada_wan_ip: Option<String> = None;
        let mut port_forwarding: Vec<serde_json::Value> = Vec::new();
        let mut linked_controller: Option<String> = None;

        // If linked to an Omada controller, fetch WAN IP
        if let (Some(ref ctrl_id), Some(ref _site_id)) =
            (&config.omada_controller_id, &config.omada_site_id)
        {
            // Find controller name
            if let Some(ctrl) = controllers.iter().find(|c| &c.controller_id == ctrl_id) {
                linked_controller = Some(ctrl.display_name.clone());
            }

            // Get WAN IP via OmadaManager
            if let Some(client) = state.omada_manager.get_client(ctrl_id).await {
                omada_wan_ip = client.get_gateway_wan_status().await.ok().flatten();

                // Get port forwarding rules
                if let Ok(rules) = client.get_port_forwarding().await {
                    port_forwarding = rules
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "name": r.name,
                                "external_port": r.external_port,
                                "internal_port": r.internal_port,
                                "internal_ip": r.internal_ip,
                                "protocol": r.protocol,
                            })
                        })
                        .collect();
                }
            }
        }

        // DNS resolve for the hostname
        let resolved_ip = resolve_hostname(&config.hostname).await;

        // Check mismatch
        let ip_mismatch = match (&omada_wan_ip, &resolved_ip) {
            (Some(wan), Some(dns)) => wan != dns,
            _ => false,
        };

        results.push(serde_json::json!({
            "config": {
                "id": config.id,
                "provider": config.provider,
                "hostname": config.hostname,
                "username": config.username.as_ref().map(|_| "********"),
                "password": config.password.as_ref().map(|_| "********"),
                "api_token": config.api_token.as_ref().map(|_| "********"),
                "zone_id": config.zone_id,
                "update_interval_sec": config.update_interval_sec,
                "last_ip": config.last_ip,
                "last_update": config.last_update,
                "last_error": config.last_error,
                "status": config.status,
                "omada_controller_id": config.omada_controller_id,
                "omada_site_id": config.omada_site_id,
                "created_at": config.created_at,
                "updated_at": config.updated_at,
            },
            "omada_wan_ip": omada_wan_ip,
            "resolved_ip": resolved_ip,
            "ip_mismatch": ip_mismatch,
            "port_forwarding": port_forwarding,
            "linked_controller": linked_controller,
        }));
    }

    Ok(Json(results))
}

/// PUT /api/ddns/:id/link-omada - Link DDNS config to Omada controller/site (admin: permission >= 80)
pub async fn link_ddns_omada(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(payload): Json<LinkOmadaRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Verify DDNS config exists
    let _config = state
        .app_state
        .mysql
        .get_ddns(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("DDNS config {} not found", id)))?;

    // If linking, verify the controller exists
    if let Some(ref ctrl_id) = payload.omada_controller_id {
        let controllers = state
            .app_state
            .mongo
            .list_omada_controllers()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to list controllers: {}", e)))?;

        if !controllers.iter().any(|c| &c.controller_id == ctrl_id) {
            return Err(AppError::NotFound(format!(
                "Omada controller {} not found",
                ctrl_id
            )));
        }
    }

    let updated = state
        .app_state
        .mysql
        .link_ddns_omada(
            id,
            payload.omada_controller_id.as_deref(),
            payload.omada_site_id.as_deref(),
        )
        .await?;

    if updated {
        tracing::info!(
            "Linked DDNS {} to Omada controller {:?} site {:?}",
            id,
            payload.omada_controller_id,
            payload.omada_site_id
        );
        Ok(Json(SuccessResponse::new("DDNS Omada link updated")))
    } else {
        Err(AppError::NotFound(format!("DDNS config {} not found", id)))
    }
}

/// GET /api/ddns/:id/port-forwards - Get port forwarding rules for linked Omada controller
pub async fn get_ddns_port_forwards(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let config = state
        .app_state
        .mysql
        .get_ddns(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("DDNS config {} not found", id)))?;

    let ctrl_id = config.omada_controller_id.as_deref().ok_or_else(|| {
        AppError::BadRequest("DDNS config is not linked to an Omada controller".to_string())
    })?;

    let client = state.omada_manager.get_client(ctrl_id).await.ok_or_else(|| {
        AppError::NotFound(format!("Omada client for controller {} not found", ctrl_id))
    })?;

    let rules = client
        .get_port_forwarding()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get port forwarding: {}", e)))?;

    Ok(Json(rules))
}

/// Resolve hostname to IP address via DNS lookup
async fn resolve_hostname(hostname: &str) -> Option<String> {
    match tokio::net::lookup_host(format!("{}:0", hostname)).await {
        Ok(mut addrs) => addrs.next().map(|addr| addr.ip().to_string()),
        Err(_) => None,
    }
}

//! Proxy routes handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};

use crate::api::auth_middleware::require_permission;
use crate::error::AppError;
use crate::models::{AuthUser, ConfirmQuery, ConfirmRequired, CreateRouteRequest, UpdateRouteRequest};
use crate::proxy::ProxyState;

use super::SuccessResponse;

/// GET /api/server-routes - List routes with subnet matching info
pub async fn list_server_routes(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let routes = state.app_state.mysql.list_routes().await?;

    // Get Omada device data for subnet matching
    let omada_devices = state
        .app_state
        .mongo
        .get_omada_devices(None, None)
        .await
        .unwrap_or_default();

    let omada_controllers = state
        .app_state
        .mongo
        .list_omada_controllers()
        .await
        .unwrap_or_default();

    // Build subnet info from gateway devices
    let mut server_routes = Vec::new();

    for route in &routes {
        // Parse target IP from the route's target URL
        let target_ip = url::Url::parse(&route.target)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()));

        let mut subnet_info: Option<serde_json::Value> = None;
        let mut fid: Option<String> = None;
        let mut tid: Option<String> = None;

        if let Some(ref ip_str) = target_ip {
            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                // Check each gateway device's LAN network
                for dev in &omada_devices {
                    if dev.device_type == "gateway" {
                        if let Some(ref dev_ip) = dev.ip {
                            // Check common subnet prefixes
                            if let Ok(dev_addr) = dev_ip.parse::<std::net::IpAddr>() {
                                // Simple /24 subnet check
                                let net = ipnetwork::IpNetwork::new(dev_addr, 24);
                                if let Ok(network) = net {
                                    if network.contains(ip) {
                                        subnet_info = Some(serde_json::json!({
                                            "network": network.to_string(),
                                            "gateway": dev_ip,
                                            "controller_id": &dev.controller_id,
                                            "site_id": &dev.site_id,
                                        }));

                                        // Find fid/tid from controller sites
                                        if let Some(ctrl) = omada_controllers.iter().find(|c| c.controller_id == dev.controller_id) {
                                            if let Some(site) = ctrl.sites.iter().find(|s| s.site_id == dev.site_id) {
                                                fid = site.fid.clone();
                                                tid = site.tid.clone();
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        server_routes.push(serde_json::json!({
            "id": route.id,
            "path": route.path,
            "target": route.target,
            "active": route.active,
            "strip_prefix": route.strip_prefix,
            "preserve_host": route.preserve_host,
            "priority": route.priority,
            "timeout_ms": route.timeout_ms,
            "websocket_support": route.websocket_support,
            "ddns_config_id": route.ddns_config_id,
            "subnet": subnet_info,
            "fid": fid,
            "tid": tid,
        }));
    }

    Ok(Json(server_routes))
}

/// GET /api/routes - List all proxy routes
pub async fn list_routes(State(state): State<ProxyState>) -> Result<impl IntoResponse, AppError> {
    let routes = state.app_state.mysql.list_routes().await?;
    Ok(Json(routes))
}

/// GET /api/routes/:id - Get a single route
pub async fn get_route(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let route = state
        .app_state
        .mysql
        .get_route(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Route {} not found", id)))?;

    Ok(Json(route))
}

/// POST /api/routes - Create a new route (admin: permission >= 80)
pub async fn create_route(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<CreateRouteRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Validate path format
    if !payload.path.starts_with('/') {
        return Err(AppError::BadRequest("Path must start with /".to_string()));
    }

    // Validate target URL
    if !payload.target.starts_with("http://") && !payload.target.starts_with("https://") {
        return Err(AppError::BadRequest(
            "Target must be a valid HTTP(S) URL".to_string(),
        ));
    }

    let id = state.app_state.mysql.create_route(&payload).await?;

    // Log audit
    let _ = state
        .app_state
        .mysql
        .log_audit(
            "route",
            Some(id),
            "create",
            None,
            None,
            Some(&format!("{} -> {}", payload.path, payload.target)),
            "api",
            None,
        )
        .await;

    // Send Discord notification
    state.notifier.notify_config_change(
        "Route Created",
        &format!("New route added: `{}` → `{}`", payload.path, payload.target),
    ).await;

    // Reload proxy routes
    if let Err(e) = state.reload_routes().await {
        tracing::error!("Failed to reload routes after create: {}", e);
    }

    tracing::info!("Created route {} -> {}", payload.path, payload.target);

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::with_id("Route created", id)),
    ))
}

/// PUT /api/routes/:id - Update a route (admin: permission >= 80)
pub async fn update_route(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRouteRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Get old route for audit log
    let old_route = state.app_state.mysql.get_route(id).await?;

    // Validate path format if provided
    if let Some(ref path) = payload.path {
        if !path.starts_with('/') {
            return Err(AppError::BadRequest("Path must start with /".to_string()));
        }
    }

    // Validate target URL if provided
    if let Some(ref target) = payload.target {
        if !target.starts_with("http://") && !target.starts_with("https://") {
            return Err(AppError::BadRequest(
                "Target must be a valid HTTP(S) URL".to_string(),
            ));
        }
    }

    let updated = state.app_state.mysql.update_route(id, &payload).await?;

    if updated {
        // Log audit for each changed field
        if let Some(ref old) = old_route {
            let mut changes = Vec::new();

            if let Some(ref new_path) = payload.path {
                if &old.path != new_path {
                    let _ = state.app_state.mysql.log_audit(
                        "route", Some(id), "update", Some("path"),
                        Some(&old.path), Some(new_path), "api", None,
                    ).await;
                    changes.push(format!("path: `{}` → `{}`", old.path, new_path));
                }
            }

            if let Some(ref new_target) = payload.target {
                if &old.target != new_target {
                    let _ = state.app_state.mysql.log_audit(
                        "route", Some(id), "update", Some("target"),
                        Some(&old.target), Some(new_target), "api", None,
                    ).await;
                    changes.push(format!("target: `{}` → `{}`", old.target, new_target));
                }
            }

            if let Some(new_active) = payload.active {
                if old.active != new_active {
                    let _ = state.app_state.mysql.log_audit(
                        "route", Some(id), "update", Some("active"),
                        Some(&old.active.to_string()), Some(&new_active.to_string()), "api", None,
                    ).await;
                    changes.push(format!("active: `{}` → `{}`", old.active, new_active));
                }
            }

            if let Some(new_ws) = payload.websocket_support {
                if old.websocket_support != new_ws {
                    let _ = state.app_state.mysql.log_audit(
                        "route", Some(id), "update", Some("websocket_support"),
                        Some(&old.websocket_support.to_string()), Some(&new_ws.to_string()), "api", None,
                    ).await;
                    changes.push(format!("websocket_support: `{}` → `{}`", old.websocket_support, new_ws));
                }
            }

            // Send Discord notification if there were changes
            if !changes.is_empty() {
                state.notifier.notify_config_change(
                    "Route Updated",
                    &format!("Route `{}` (ID: {}) modified:\n{}", old.path, id, changes.join("\n")),
                ).await;
            }
        }

        // Reload proxy routes
        if let Err(e) = state.reload_routes().await {
            tracing::error!("Failed to reload routes after update: {}", e);
        }

        tracing::info!("Updated route {}", id);
        Ok(Json(SuccessResponse::new("Route updated")))
    } else {
        Err(AppError::NotFound(format!("Route {} not found", id)))
    }
}

/// DELETE /api/routes/:id - Delete a route (dangerous: permission == 100, confirm required)
pub async fn delete_route(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<i32>,
    Query(confirm): Query<ConfirmQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 100)?;

    // Get route info before deletion for audit log
    let route = state.app_state.mysql.get_route(id).await?;

    // Confirm guard: return impact info if confirm is not set
    if !confirm.confirm {
        let target_info = route
            .as_ref()
            .map(|r| format!("route #{} ({} → {})", id, r.path, r.target))
            .unwrap_or_else(|| format!("route #{}", id));

        return Ok(Json(serde_json::json!(ConfirmRequired {
            action: "delete_route".to_string(),
            target: target_info,
            warning: "This will remove the proxy route. Active connections will be dropped.".to_string(),
            confirm_required: true,
        })));
    }

    let deleted = state.app_state.mysql.delete_route(id).await?;

    if deleted {
        // Log audit
        if let Some(ref r) = route {
            let _ = state.app_state.mysql.log_audit(
                "route", Some(id), "delete", None,
                Some(&format!("{} -> {}", r.path, r.target)), None, "api", None,
            ).await;

            // Send Discord notification
            state.notifier.notify_config_change(
                "Route Deleted",
                &format!("Route removed: `{}` → `{}`", r.path, r.target),
            ).await;
        }

        // Reload proxy routes
        if let Err(e) = state.reload_routes().await {
            tracing::error!("Failed to reload routes after delete: {}", e);
        }

        tracing::info!("Deleted route {}", id);
        Ok(Json(serde_json::json!(SuccessResponse::new("Route deleted"))))
    } else {
        Err(AppError::NotFound(format!("Route {} not found", id)))
    }
}

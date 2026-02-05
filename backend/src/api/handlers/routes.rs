//! Proxy routes handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::error::AppError;
use crate::models::{CreateRouteRequest, UpdateRouteRequest};
use crate::proxy::ProxyState;

use super::SuccessResponse;

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

/// POST /api/routes - Create a new route
pub async fn create_route(
    State(state): State<ProxyState>,
    Json(payload): Json<CreateRouteRequest>,
) -> Result<impl IntoResponse, AppError> {
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

    tracing::info!("Created route {} -> {}", payload.path, payload.target);

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::with_id("Route created", id)),
    ))
}

/// PUT /api/routes/:id - Update a route
pub async fn update_route(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRouteRequest>,
) -> Result<impl IntoResponse, AppError> {
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

            // Send Discord notification if there were changes
            if !changes.is_empty() {
                state.notifier.notify_config_change(
                    "Route Updated",
                    &format!("Route `{}` (ID: {}) modified:\n{}", old.path, id, changes.join("\n")),
                ).await;
            }
        }

        tracing::info!("Updated route {}", id);
        Ok(Json(SuccessResponse::new("Route updated")))
    } else {
        Err(AppError::NotFound(format!("Route {} not found", id)))
    }
}

/// DELETE /api/routes/:id - Delete a route
pub async fn delete_route(
    State(state): State<ProxyState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    // Get route info before deletion for audit log
    let route = state.app_state.mysql.get_route(id).await?;

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

        tracing::info!("Deleted route {}", id);
        Ok(Json(SuccessResponse::new("Route deleted")))
    } else {
        Err(AppError::NotFound(format!("Route {} not found", id)))
    }
}

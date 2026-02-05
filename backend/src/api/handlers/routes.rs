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
    let deleted = state.app_state.mysql.delete_route(id).await?;

    if deleted {
        tracing::info!("Deleted route {}", id);
        Ok(Json(SuccessResponse::new("Route deleted")))
    } else {
        Err(AppError::NotFound(format!("Route {} not found", id)))
    }
}

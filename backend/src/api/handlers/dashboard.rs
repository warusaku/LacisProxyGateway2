//! Dashboard handlers

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{DashboardStats, RouteHealth};
use crate::proxy::ProxyState;

use super::security::PaginationQuery;

/// GET /api/dashboard/stats - Get dashboard statistics
pub async fn get_dashboard_stats(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let total_requests_today = state
        .app_state
        .mongo
        .get_today_request_count()
        .await
        .unwrap_or(0);
    let active_routes = state
        .app_state
        .mysql
        .count_active_routes()
        .await
        .unwrap_or(0);
    let active_ddns = state.app_state.mysql.count_active_ddns().await.unwrap_or(0);
    let blocked_ips = state.app_state.mysql.count_blocked_ips().await.unwrap_or(0);

    // Determine overall health based on latest health checks
    let health_checks = state
        .app_state
        .mongo
        .get_latest_health_status()
        .await
        .unwrap_or_default();
    let unhealthy_count = health_checks.iter().filter(|c| !c.healthy).count();
    let server_health = if unhealthy_count == 0 {
        "healthy"
    } else if unhealthy_count < health_checks.len() / 2 {
        "degraded"
    } else {
        "unhealthy"
    };

    Ok(Json(DashboardStats {
        total_requests_today,
        active_routes,
        active_ddns,
        blocked_ips,
        server_health: server_health.to_string(),
        uptime_seconds: state.app_state.uptime_seconds(),
    }))
}

/// GET /api/dashboard/access-log - Get recent access logs
pub async fn get_access_log(
    State(state): State<ProxyState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = state
        .app_state
        .mongo
        .get_access_logs(pagination.limit, pagination.offset)
        .await?;

    Ok(Json(logs))
}

/// GET /api/dashboard/health - Get health status for all routes
pub async fn get_health_status(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let routes = state.app_state.mysql.list_active_routes().await?;
    let health_checks = state.app_state.mongo.get_latest_health_status().await?;

    let mut route_health: Vec<RouteHealth> = Vec::new();

    for route in routes {
        let check = health_checks.iter().find(|c| c.route_id == route.id);
        let consecutive_failures = state
            .app_state
            .mongo
            .count_consecutive_failures(route.id)
            .await
            .unwrap_or(0);

        route_health.push(RouteHealth {
            route_id: route.id,
            path: route.path,
            target: route.target,
            healthy: check.map(|c| c.healthy).unwrap_or(true),
            last_check: check.map(|c| c.timestamp),
            consecutive_failures,
        });
    }

    Ok(Json(route_health))
}

#[derive(Debug, Serialize)]
pub struct StatusDistribution {
    pub status: i32,
    pub count: u64,
}

/// GET /api/dashboard/status-distribution - Get request status code distribution
pub async fn get_status_distribution(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let distribution = state
        .app_state
        .mongo
        .get_today_status_distribution()
        .await?;

    let result: Vec<StatusDistribution> = distribution
        .into_iter()
        .map(|(status, count)| StatusDistribution { status, count })
        .collect();

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct LogFilterQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub path: Option<String>,
    pub ip: Option<String>,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/dashboard/access-log/filter - Get filtered access logs
pub async fn get_filtered_access_log(
    State(state): State<ProxyState>,
    Query(filter): Query<LogFilterQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = if let Some(ref path) = filter.path {
        state
            .app_state
            .mongo
            .get_access_logs_by_path(path, filter.limit)
            .await?
    } else if let Some(ref ip) = filter.ip {
        state
            .app_state
            .mongo
            .get_access_logs_by_ip(ip, filter.limit)
            .await?
    } else {
        state
            .app_state
            .mongo
            .get_access_logs(filter.limit, 0)
            .await?
    };

    Ok(Json(logs))
}

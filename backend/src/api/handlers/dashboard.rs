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

/// SSL Certificate Status
#[derive(Debug, Serialize)]
pub struct SslStatus {
    pub enabled: bool,
    pub domain: Option<String>,
    pub issuer: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub days_remaining: Option<i64>,
    pub auto_renew: bool,
    pub last_renewal: Option<String>,
    pub next_renewal_attempt: Option<String>,
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

/// GET /api/dashboard/ssl-status - Get SSL certificate status
pub async fn get_ssl_status() -> impl IntoResponse {
    let cert_path = "/etc/letsencrypt/live/akbdevs.dnsalias.com/fullchain.pem";
    let renewal_conf_path = "/etc/letsencrypt/renewal/akbdevs.dnsalias.com.conf";

    // Check if SSL is configured
    let ssl_enabled = std::path::Path::new(cert_path).exists();

    if !ssl_enabled {
        return Json(SslStatus {
            enabled: false,
            domain: None,
            issuer: None,
            valid_from: None,
            valid_until: None,
            days_remaining: None,
            auto_renew: false,
            last_renewal: None,
            next_renewal_attempt: None,
        });
    }

    // Parse certificate using openssl command
    let cert_info = std::process::Command::new("openssl")
        .args(["x509", "-in", cert_path, "-noout", "-dates", "-issuer", "-subject"])
        .output();

    let mut domain = Some("akbdevs.dnsalias.com".to_string());
    let mut issuer = None;
    let mut valid_from = None;
    let mut valid_until = None;
    let mut days_remaining = None;

    if let Ok(output) = cert_info {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("notBefore=") {
                valid_from = Some(line.replace("notBefore=", ""));
            } else if line.starts_with("notAfter=") {
                valid_until = Some(line.replace("notAfter=", ""));
            } else if line.starts_with("issuer=") {
                // Extract CN from issuer
                if let Some(cn_part) = line.split("CN = ").nth(1) {
                    issuer = Some(cn_part.split(',').next().unwrap_or(cn_part).to_string());
                }
            } else if line.starts_with("subject=") {
                // Extract CN from subject for domain
                if let Some(cn_part) = line.split("CN = ").nth(1) {
                    domain = Some(cn_part.split(',').next().unwrap_or(cn_part).to_string());
                }
            }
        }
    }

    // Calculate days remaining
    if let Some(ref until) = valid_until {
        let end_date = std::process::Command::new("date")
            .args(["-d", until, "+%s"])
            .output();
        let now = std::process::Command::new("date").args(["+%s"]).output();

        if let (Ok(end_out), Ok(now_out)) = (end_date, now) {
            let end_secs: i64 = String::from_utf8_lossy(&end_out.stdout)
                .trim()
                .parse()
                .unwrap_or(0);
            let now_secs: i64 = String::from_utf8_lossy(&now_out.stdout)
                .trim()
                .parse()
                .unwrap_or(0);
            if end_secs > 0 && now_secs > 0 {
                days_remaining = Some((end_secs - now_secs) / 86400);
            }
        }
    }

    // Check auto-renew status (certbot timer)
    let auto_renew = std::process::Command::new("systemctl")
        .args(["is-active", "certbot.timer"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);

    // Get last renewal from renewal conf modification time
    let last_renewal = std::fs::metadata(renewal_conf_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        });

    // Next renewal attempt (certbot renews 30 days before expiry)
    let next_renewal_attempt = days_remaining.map(|days| {
        if days > 30 {
            format!("{} days until renewal window", days - 30)
        } else {
            "In renewal window (will renew on next check)".to_string()
        }
    });

    Json(SslStatus {
        enabled: true,
        domain,
        issuer,
        valid_from,
        valid_until,
        days_remaining,
        auto_renew,
        last_renewal,
        next_renewal_attempt,
    })
}

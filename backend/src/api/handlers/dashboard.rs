//! Dashboard handlers

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::api::admin_guard::extract_client_ip;
use crate::error::AppError;
use crate::models::{AccessLogSearchQuery, DashboardStats, RouteHealth};
use crate::proxy::ProxyState;

use super::security::PaginationQuery;

/// GET /api/my-ip - Get the client's IP address, server's global IP, and IP history
///
/// Returns current IPs and ALL historical IPs (for exclusion filters).
/// Also records the client IP and server IP into ip_history collection.
pub async fn get_my_ip(
    State(state): State<ProxyState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, addr);

    // サーバーのグローバルIPをDDNS last_ipから動的に取得（ハードコード禁止）
    let server_ip = state
        .app_state
        .mysql
        .list_ddns()
        .await
        .ok()
        .and_then(|configs| {
            configs
                .into_iter()
                .find(|c| c.status == crate::models::DdnsStatus::Active)
                .and_then(|c| c.last_ip)
        });

    // IP履歴に記録（期間追跡のため: first_seen/last_seen を upsert）
    let _ = state.app_state.mongo.upsert_ip_history(&ip, "admin").await;
    if let Some(ref sip) = server_ip {
        let _ = state.app_state.mongo.upsert_ip_history(sip, "server").await;
    }

    // 全履歴IPを取得（IP変更後も旧IPでフィルタ可能にする）
    let server_ip_history = state
        .app_state
        .mongo
        .get_ip_history("server")
        .await
        .unwrap_or_default();
    let admin_ip_history = state
        .app_state
        .mongo
        .get_ip_history("admin")
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "ip": ip,
        "server_ip": server_ip,
        "server_ip_history": server_ip_history,
        "admin_ip_history": admin_ip_history,
    }))
}

/// Dashboard stats query with IP exclusion parameters
#[derive(Debug, Deserialize)]
pub struct DashboardStatsQuery {
    pub exclude_ips: Option<String>,
    pub exclude_lan: Option<bool>,
}

/// Dashboard pagination query with IP exclusion parameters
#[derive(Debug, Deserialize)]
pub struct DashboardPaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub exclude_ips: Option<String>,
    pub exclude_lan: Option<bool>,
}

/// GET /api/dashboard/stats - Get dashboard statistics
pub async fn get_dashboard_stats(
    State(state): State<ProxyState>,
    Query(query): Query<DashboardStatsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let total_requests_today = state
        .app_state
        .mongo
        .get_today_request_count(&query.exclude_ips, &query.exclude_lan)
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
    Query(pagination): Query<DashboardPaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = state
        .app_state
        .mongo
        .get_access_logs(pagination.limit, pagination.offset, &pagination.exclude_ips, &pagination.exclude_lan)
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

/// Detailed route status with metrics
#[derive(Debug, Serialize)]
pub struct RouteDetailedStatus {
    pub route_id: i32,
    pub path: String,
    pub target: String,
    pub active: bool,
    pub healthy: bool,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub consecutive_failures: u32,
    pub response_time_ms: Option<i32>,
    pub last_status_code: Option<i32>,
    pub requests_today: u64,
    pub requests_last_hour: u64,
    pub error_rate_percent: f64,
    pub avg_response_time_ms: f64,
}

/// GET /api/routes/status - Get detailed status for all routes
pub async fn get_all_routes_status(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let routes = state.app_state.mysql.list_routes().await?;
    let health_checks = state.app_state.mongo.get_latest_health_status().await?;

    let mut detailed_status: Vec<RouteDetailedStatus> = Vec::new();

    for route in routes {
        let check = health_checks.iter().find(|c| c.route_id == route.id);
        let consecutive_failures = state
            .app_state
            .mongo
            .count_consecutive_failures(route.id)
            .await
            .unwrap_or(0);

        // Get request stats for this route
        let stats = state
            .app_state
            .mongo
            .get_route_stats(&route.path)
            .await
            .unwrap_or_default();

        detailed_status.push(RouteDetailedStatus {
            route_id: route.id,
            path: route.path.clone(),
            target: route.target.clone(),
            active: route.active,
            healthy: check.map(|c| c.healthy).unwrap_or(true),
            last_check: check.map(|c| c.timestamp),
            consecutive_failures,
            response_time_ms: check.and_then(|c| c.response_time_ms),
            last_status_code: check.and_then(|c| c.status_code),
            requests_today: stats.requests_today,
            requests_last_hour: stats.requests_last_hour,
            error_rate_percent: stats.error_rate_percent,
            avg_response_time_ms: stats.avg_response_time_ms,
        });
    }

    Ok(Json(detailed_status))
}

/// GET /api/routes/:id/status - Get detailed status for a specific route
pub async fn get_route_status(
    State(state): State<ProxyState>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let route = state
        .app_state
        .mysql
        .get_route(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Route with id {} not found", id)))?;
    let health_checks = state.app_state.mongo.get_latest_health_status().await?;
    let check = health_checks.iter().find(|c| c.route_id == route.id);
    let consecutive_failures = state
        .app_state
        .mongo
        .count_consecutive_failures(route.id)
        .await
        .unwrap_or(0);

    let stats = state
        .app_state
        .mongo
        .get_route_stats(&route.path)
        .await
        .unwrap_or_default();

    let detailed_status = RouteDetailedStatus {
        route_id: route.id,
        path: route.path.clone(),
        target: route.target.clone(),
        active: route.active,
        healthy: check.map(|c| c.healthy).unwrap_or(true),
        last_check: check.map(|c| c.timestamp),
        consecutive_failures,
        response_time_ms: check.and_then(|c| c.response_time_ms),
        last_status_code: check.and_then(|c| c.status_code),
        requests_today: stats.requests_today,
        requests_last_hour: stats.requests_last_hour,
        error_rate_percent: stats.error_rate_percent,
        avg_response_time_ms: stats.avg_response_time_ms,
    };

    Ok(Json(detailed_status))
}

/// GET /api/routes/:id/logs - Get access logs for a specific route
pub async fn get_route_logs(
    State(state): State<ProxyState>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let route = state
        .app_state
        .mysql
        .get_route(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Route with id {} not found", id)))?;
    let logs = state
        .app_state
        .mongo
        .get_access_logs_by_path(&route.path, pagination.limit)
        .await?;

    Ok(Json(logs))
}

#[derive(Debug, Serialize)]
pub struct StatusDistribution {
    pub status: i32,
    pub count: u64,
}

/// GET /api/dashboard/status-distribution - Get request status code distribution
pub async fn get_status_distribution(
    State(state): State<ProxyState>,
    Query(query): Query<DashboardStatsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let distribution = state
        .app_state
        .mongo
        .get_today_status_distribution(&query.exclude_ips, &query.exclude_lan)
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
    let none_str: Option<String> = None;
    let none_bool: Option<bool> = None;
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
            .get_access_logs(filter.limit, 0, &none_str, &none_bool)
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

/// Server Health Metrics
#[derive(Debug, Serialize)]
pub struct ServerHealth {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub uptime: String,
    pub uptime_seconds: u64,
    pub load_average: LoadAverage,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub swap: SwapInfo,
    pub disk: Vec<DiskInfo>,
    pub network: NetworkInfo,
    pub processes: ProcessInfo,
}

#[derive(Debug, Serialize)]
pub struct LoadAverage {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
}

#[derive(Debug, Serialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub available_mb: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct SwapInfo {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct DiskInfo {
    pub mount_point: String,
    pub filesystem: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub connections: u32,
}

#[derive(Debug, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: Option<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub total: u32,
    pub running: u32,
    pub sleeping: u32,
}

fn run_command(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn parse_meminfo() -> (MemoryInfo, SwapInfo) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut mem_total = 0u64;
    let mut mem_free = 0u64;
    let mut mem_available = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value: u64 = parts[1].parse().unwrap_or(0);
            match parts[0] {
                "MemTotal:" => mem_total = value,
                "MemFree:" => mem_free = value,
                "MemAvailable:" => mem_available = value,
                "Buffers:" => buffers = value,
                "Cached:" => cached = value,
                "SwapTotal:" => swap_total = value,
                "SwapFree:" => swap_free = value,
                _ => {}
            }
        }
    }

    let mem_used = mem_total.saturating_sub(mem_free + buffers + cached);
    let swap_used = swap_total.saturating_sub(swap_free);

    let memory = MemoryInfo {
        total_mb: mem_total / 1024,
        used_mb: mem_used / 1024,
        free_mb: mem_free / 1024,
        available_mb: mem_available / 1024,
        usage_percent: if mem_total > 0 {
            (mem_used as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        },
    };

    let swap = SwapInfo {
        total_mb: swap_total / 1024,
        used_mb: swap_used / 1024,
        free_mb: swap_free / 1024,
        usage_percent: if swap_total > 0 {
            (swap_used as f64 / swap_total as f64) * 100.0
        } else {
            0.0
        },
    };

    (memory, swap)
}

fn get_cpu_usage() -> f64 {
    // Read /proc/stat twice with a small delay to calculate CPU usage
    let read_stat = || -> (u64, u64) {
        let content = std::fs::read_to_string("/proc/stat").unwrap_or_default();
        if let Some(line) = content.lines().next() {
            let parts: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 4 {
                let idle = parts[3];
                let total: u64 = parts.iter().sum();
                return (idle, total);
            }
        }
        (0, 0)
    };

    let (idle1, total1) = read_stat();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let (idle2, total2) = read_stat();

    let idle_delta = idle2.saturating_sub(idle1);
    let total_delta = total2.saturating_sub(total1);

    if total_delta > 0 {
        ((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0
    } else {
        0.0
    }
}

fn get_disk_info() -> Vec<DiskInfo> {
    let output = run_command("df", &["-BG", "--output=target,fstype,size,used,avail,pcent"]);
    let mut disks = Vec::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let mount = parts[0];
            // Skip virtual filesystems
            if mount.starts_with("/dev") || mount == "/" || mount.starts_with("/home") || mount.starts_with("/var") {
                let parse_gb = |s: &str| -> f64 {
                    s.trim_end_matches('G').parse().unwrap_or(0.0)
                };
                let parse_percent = |s: &str| -> f64 {
                    s.trim_end_matches('%').parse().unwrap_or(0.0)
                };

                disks.push(DiskInfo {
                    mount_point: mount.to_string(),
                    filesystem: parts[1].to_string(),
                    total_gb: parse_gb(parts[2]),
                    used_gb: parse_gb(parts[3]),
                    free_gb: parse_gb(parts[4]),
                    usage_percent: parse_percent(parts[5]),
                });
            }
        }
    }

    // If no disks found, try simpler approach
    if disks.is_empty() {
        let output = run_command("df", &["-h", "/"]);
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                disks.push(DiskInfo {
                    mount_point: "/".to_string(),
                    filesystem: parts[0].to_string(),
                    total_gb: 0.0,
                    used_gb: 0.0,
                    free_gb: 0.0,
                    usage_percent: parts[4].trim_end_matches('%').parse().unwrap_or(0.0),
                });
            }
        }
    }

    disks
}

fn get_network_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    let net_dev = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();

    for line in net_dev.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            let name = parts[0].trim_end_matches(':');
            if name != "lo" {
                let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
                let tx_bytes: u64 = parts[9].parse().unwrap_or(0);

                // Get IP address
                let ip_output = run_command("ip", &["-4", "addr", "show", name]);
                let ip = ip_output
                    .lines()
                    .find(|l| l.contains("inet "))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .map(|s| s.split('/').next().unwrap_or(s).to_string());

                interfaces.push(NetworkInterface {
                    name: name.to_string(),
                    ip,
                    rx_bytes,
                    tx_bytes,
                });
            }
        }
    }

    interfaces
}

/// GET /api/dashboard/server-health - Get detailed server health metrics
pub async fn get_server_health() -> impl IntoResponse {
    // Hostname
    let hostname = run_command("hostname", &[]);

    // OS info
    let os = run_command("lsb_release", &["-ds"])
        .trim_matches('"')
        .to_string();
    let os = if os.is_empty() {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|c| {
                c.lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.replace("PRETTY_NAME=", "").trim_matches('"').to_string())
            })
            .unwrap_or_else(|| "Linux".to_string())
    } else {
        os
    };

    // Kernel
    let kernel = run_command("uname", &["-r"]);

    // Uptime
    let uptime_content = std::fs::read_to_string("/proc/uptime").unwrap_or_default();
    let uptime_seconds: u64 = uptime_content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0);

    let days = uptime_seconds / 86400;
    let hours = (uptime_seconds % 86400) / 3600;
    let mins = (uptime_seconds % 3600) / 60;
    let uptime = if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    };

    // Load average
    let loadavg = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let load_parts: Vec<f64> = loadavg
        .split_whitespace()
        .take(3)
        .filter_map(|s| s.parse().ok())
        .collect();
    let load_average = LoadAverage {
        one_min: load_parts.first().copied().unwrap_or(0.0),
        five_min: load_parts.get(1).copied().unwrap_or(0.0),
        fifteen_min: load_parts.get(2).copied().unwrap_or(0.0),
    };

    // CPU info
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let cpu_model = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let cpu_cores = cpuinfo.lines().filter(|l| l.starts_with("processor")).count() as u32;
    let cpu_usage = get_cpu_usage();

    let cpu = CpuInfo {
        model: cpu_model,
        cores: cpu_cores,
        usage_percent: (cpu_usage * 10.0).round() / 10.0,
    };

    // Memory and Swap
    let (memory, swap) = parse_meminfo();

    // Disk
    let disk = get_disk_info();

    // Network
    let interfaces = get_network_interfaces();
    let connections: u32 = run_command("ss", &["-tun"])
        .lines()
        .count()
        .saturating_sub(1) as u32;

    let network = NetworkInfo {
        interfaces,
        connections,
    };

    // Processes
    let proc_output = run_command("ps", &["aux"]);
    let total_procs = proc_output.lines().count().saturating_sub(1) as u32;
    let running = proc_output
        .lines()
        .filter(|l| l.contains(" R ") || l.contains(" R+ "))
        .count() as u32;

    let processes = ProcessInfo {
        total: total_procs,
        running,
        sleeping: total_procs.saturating_sub(running),
    };

    Json(ServerHealth {
        hostname,
        os,
        kernel,
        uptime,
        uptime_seconds,
        load_average,
        cpu,
        memory,
        swap,
        disk,
        network,
        processes,
    })
}

// ============================================================================
// Advanced search & analytics endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct TimeRangeQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub exclude_ips: Option<String>,
    pub exclude_lan: Option<bool>,
}

/// GET /api/dashboard/access-log/search - Advanced log search
pub async fn search_access_log(
    State(state): State<ProxyState>,
    Query(query): Query<AccessLogSearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let result = state
        .app_state
        .mongo
        .search_access_logs(&query)
        .await?;

    Ok(Json(result))
}

/// GET /api/dashboard/hourly-stats - Hourly aggregation
pub async fn get_hourly_stats(
    State(state): State<ProxyState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let from = query
        .from
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query
        .to
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);

    let stats = state
        .app_state
        .mongo
        .get_hourly_stats(from, to, &query.exclude_ips, &query.exclude_lan)
        .await?;

    Ok(Json(stats))
}

/// GET /api/dashboard/top-ips - Top IPs by request count
pub async fn get_top_ips(
    State(state): State<ProxyState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let from = query
        .from
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query
        .to
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);
    let limit = query.limit.unwrap_or(20);

    let entries = state
        .app_state
        .mongo
        .get_top_ips(from, to, limit, &query.exclude_ips, &query.exclude_lan)
        .await?;

    Ok(Json(entries))
}

/// GET /api/dashboard/top-paths - Top paths by request count
pub async fn get_top_paths(
    State(state): State<ProxyState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let from = query
        .from
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query
        .to
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);
    let limit = query.limit.unwrap_or(20);

    let entries = state
        .app_state
        .mongo
        .get_top_paths(from, to, limit, &query.exclude_ips, &query.exclude_lan)
        .await?;

    Ok(Json(entries))
}

/// GET /api/dashboard/error-summary - Error grouping summary
pub async fn get_error_summary(
    State(state): State<ProxyState>,
    Query(query): Query<TimeRangeQuery>,
) -> Result<impl IntoResponse, AppError> {
    let from = query
        .from
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
    let to = query
        .to
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);

    let summary = state
        .app_state
        .mongo
        .get_error_summary(from, to, &query.exclude_ips, &query.exclude_lan)
        .await?;

    Ok(Json(summary))
}

/// GET /api/dashboard/access-log/export - CSV export
pub async fn export_access_log(
    State(state): State<ProxyState>,
    Query(query): Query<AccessLogSearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Limit to 10000 for export
    let mut export_query = AccessLogSearchQuery {
        from: query.from,
        to: query.to,
        method: query.method,
        status_min: query.status_min,
        status_max: query.status_max,
        ip: query.ip,
        path: query.path,
        limit: query.limit.min(10000),
        offset: 0,
        exclude_ips: query.exclude_ips,
        exclude_lan: query.exclude_lan,
    };
    if export_query.limit == 0 {
        export_query.limit = 10000;
    }

    let result = state
        .app_state
        .mongo
        .search_access_logs(&export_query)
        .await?;

    // Build CSV
    let mut csv = String::from("timestamp,ip,method,path,status,response_time_ms,user_agent,referer\n");
    for log in &result.logs {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            log.timestamp.to_rfc3339(),
            csv_escape(&log.ip),
            csv_escape(&log.method),
            csv_escape(&log.path),
            log.status,
            log.response_time_ms,
            csv_escape(log.user_agent.as_deref().unwrap_or("")),
            csv_escape(log.referer.as_deref().unwrap_or("")),
        ));
    }

    let headers = [
        (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"access_logs.csv\"",
        ),
    ];

    Ok((StatusCode::OK, headers, csv))
}

/// Escape a field for CSV (wrap in quotes if it contains comma, quote, or newline)
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

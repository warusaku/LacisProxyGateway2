//! Diagnostics API handler - System integration test endpoint
//!
//! POST /api/tools/diagnostics
//! Runs diagnostic checks across all 10 subsystem categories.

use axum::{
    extract::State,
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::api::auth_middleware::require_permission;
use crate::db::mongo::OperatorInfo;
use crate::error::AppError;
use crate::models::AuthUser;
use crate::proxy::ProxyState;

// Re-use nginx helper functions (pub(crate) in nginx.rs)
use super::nginx::{check_nginx_running, test_nginx_config};

// ============================================================================
// Request / Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DiagnosticsRequest {
    /// Filter by categories (omit = run all)
    pub categories: Option<Vec<String>>,
    /// Include device connectivity tests (default: false, network I/O)
    pub include_device_tests: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticsResponse {
    pub checks: Vec<DiagnosticCheck>,
    pub summary: DiagnosticSummary,
    pub operation_id: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticCheck {
    pub category: String,
    pub name: String,
    /// "ok" | "warning" | "error"
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticSummary {
    pub total: usize,
    pub ok: usize,
    pub warning: usize,
    pub error: usize,
}

// ============================================================================
// All valid category names (MECE)
// ============================================================================

const ALL_CATEGORIES: &[&str] = &[
    "database", "nginx", "proxy_routes", "ddns", "omada",
    "openwrt", "external", "aranea", "geoip", "system",
];

// ============================================================================
// Main handler
// ============================================================================

/// POST /api/tools/diagnostics (operate: permission >= 50)
pub async fn run_diagnostics(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<DiagnosticsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 50)?;

    let overall_start = Instant::now();

    // Start operation log with operator info
    let operator = OperatorInfo {
        sub: user.sub.clone(),
        auth_method: user.auth_method.clone(),
        permission: user.permission,
    };
    let op_id = state
        .app_state
        .mongo
        .start_operation_log_with_operator("diagnostics", "api", None, Some(operator))
        .await
        .unwrap_or_default();

    let include_device_tests = payload.include_device_tests.unwrap_or(false);

    // Determine which categories to run
    let selected: Vec<&str> = match &payload.categories {
        Some(cats) => cats
            .iter()
            .filter_map(|c| {
                let s = c.as_str();
                if ALL_CATEGORIES.contains(&s) { Some(s) } else { None }
            })
            .collect(),
        None => ALL_CATEGORIES.to_vec(),
    };

    let mut checks: Vec<DiagnosticCheck> = Vec::new();

    // Run each selected category
    for cat in &selected {
        match *cat {
            "database"     => checks.extend(check_database(&state).await),
            "nginx"        => checks.extend(check_nginx().await),
            "proxy_routes" => checks.extend(check_proxy_routes(&state, include_device_tests).await),
            "ddns"         => checks.extend(check_ddns(&state, include_device_tests).await),
            "omada"        => checks.extend(check_omada(&state, include_device_tests).await),
            "openwrt"      => checks.extend(check_openwrt(&state, include_device_tests).await),
            "external"     => checks.extend(check_external(&state, include_device_tests).await),
            "aranea"       => checks.extend(check_aranea(&state).await),
            "geoip"        => checks.extend(check_geoip(&state).await),
            "system"       => checks.extend(check_system(&state).await),
            _ => {}
        }
    }

    // Build summary
    let ok_count = checks.iter().filter(|c| c.status == "ok").count();
    let warn_count = checks.iter().filter(|c| c.status == "warning").count();
    let err_count = checks.iter().filter(|c| c.status == "error").count();

    let summary = DiagnosticSummary {
        total: checks.len(),
        ok: ok_count,
        warning: warn_count,
        error: err_count,
    };

    let duration = overall_start.elapsed().as_millis() as u64;

    // Complete operation log
    if !op_id.is_empty() {
        let _ = state
            .app_state
            .mongo
            .complete_operation_log(
                &op_id,
                Some(&serde_json::json!({
                    "summary": { "total": summary.total, "ok": summary.ok, "warning": summary.warning, "error": summary.error },
                    "categories": selected,
                    "include_device_tests": include_device_tests,
                })),
                duration,
            )
            .await;
    }

    Ok(Json(DiagnosticsResponse {
        checks,
        summary,
        operation_id: op_id,
        duration_ms: duration,
    }))
}

// ============================================================================
// Category 1: database
// ============================================================================

async fn check_database(state: &ProxyState) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // MySQL connection
    {
        let start = Instant::now();
        let (status, message, details) = match state.app_state.mysql.pool().acquire().await {
            Ok(_conn) => ("ok", "MySQL connection acquired successfully".to_string(), None),
            Err(e) => ("error", format!("MySQL connection failed: {}", e), None),
        };
        checks.push(DiagnosticCheck {
            category: "database".into(),
            name: "mysql_connection".into(),
            status: status.into(),
            message,
            details,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // MongoDB connection
    {
        let start = Instant::now();
        let (status, message, details) = match state
            .app_state
            .mongo
            .db()
            .run_command(mongodb::bson::doc! { "ping": 1 }, None)
            .await
        {
            Ok(_) => ("ok", "MongoDB ping successful".to_string(), None),
            Err(e) => ("error", format!("MongoDB ping failed: {}", e), None),
        };
        checks.push(DiagnosticCheck {
            category: "database".into(),
            name: "mongodb_connection".into(),
            status: status.into(),
            message,
            details,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    checks
}

// ============================================================================
// Category 2: nginx
// ============================================================================

async fn check_nginx() -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // nginx running
    {
        let start = Instant::now();
        let running = check_nginx_running().await;
        checks.push(DiagnosticCheck {
            category: "nginx".into(),
            name: "nginx_running".into(),
            status: if running { "ok" } else { "error" }.into(),
            message: if running { "nginx is running" } else { "nginx is NOT running" }.into(),
            details: None,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // nginx config valid
    {
        let start = Instant::now();
        let (valid, error) = test_nginx_config().await;
        checks.push(DiagnosticCheck {
            category: "nginx".into(),
            name: "nginx_config_valid".into(),
            status: if valid { "ok" } else { "error" }.into(),
            message: if valid {
                "nginx config test passed".into()
            } else {
                format!("nginx config test failed: {}", error.unwrap_or_default())
            },
            details: None,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    checks
}

// ============================================================================
// Category 3: proxy_routes
// ============================================================================

async fn check_proxy_routes(state: &ProxyState, include_device_tests: bool) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Count active routes
    {
        let start = Instant::now();
        match state.app_state.mysql.list_active_routes().await {
            Ok(routes) => {
                let count = routes.len();
                checks.push(DiagnosticCheck {
                    category: "proxy_routes".into(),
                    name: "active_routes_count".into(),
                    status: if count > 0 { "ok" } else { "warning" }.into(),
                    message: format!("{} active routes configured", count),
                    details: Some(serde_json::json!({ "count": count })),
                    duration_ms: start.elapsed().as_millis() as u64,
                });

                // Optional: reachability test for each route
                if include_device_tests {
                    for route in &routes {
                        let rt_start = Instant::now();
                        let result = state
                            .http_client
                            .head(&route.target)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await;
                        let (status, msg) = match result {
                            Ok(resp) => {
                                let code = resp.status().as_u16();
                                if code < 500 {
                                    ("ok", format!("Reachable (HTTP {})", code))
                                } else {
                                    ("warning", format!("Server error (HTTP {})", code))
                                }
                            }
                            Err(e) => ("error", format!("Unreachable: {}", e)),
                        };
                        checks.push(DiagnosticCheck {
                            category: "proxy_routes".into(),
                            name: format!("route_reachable_{}", route.id),
                            status: status.into(),
                            message: format!("{} -> {} : {}", route.path, route.target, msg),
                            details: Some(serde_json::json!({
                                "route_id": route.id,
                                "path": route.path,
                                "target": route.target,
                            })),
                            duration_ms: rt_start.elapsed().as_millis() as u64,
                        });
                    }
                }
            }
            Err(e) => {
                checks.push(DiagnosticCheck {
                    category: "proxy_routes".into(),
                    name: "active_routes_count".into(),
                    status: "error".into(),
                    message: format!("Failed to list routes: {}", e),
                    details: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    checks
}

// ============================================================================
// Category 4: ddns
// ============================================================================

async fn check_ddns(state: &ProxyState, include_device_tests: bool) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    {
        let start = Instant::now();
        match state.app_state.mysql.list_active_ddns().await {
            Ok(configs) => {
                let count = configs.len();
                checks.push(DiagnosticCheck {
                    category: "ddns".into(),
                    name: "ddns_configs_count".into(),
                    status: if count > 0 { "ok" } else { "warning" }.into(),
                    message: format!("{} active DDNS configurations", count),
                    details: Some(serde_json::json!({ "count": count })),
                    duration_ms: start.elapsed().as_millis() as u64,
                });

                // Optional: DNS resolution test
                if include_device_tests {
                    for config in &configs {
                        let dns_start = Instant::now();
                        let (status, msg) = match tokio::net::lookup_host(format!("{}:0", config.hostname)).await {
                            Ok(addrs) => {
                                let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                                if ips.is_empty() {
                                    ("warning", "Resolved but no addresses returned".to_string())
                                } else {
                                    ("ok", format!("Resolved to: {}", ips.join(", ")))
                                }
                            }
                            Err(e) => ("error", format!("DNS resolution failed: {}", e)),
                        };
                        checks.push(DiagnosticCheck {
                            category: "ddns".into(),
                            name: format!("ddns_resolve_{}", config.id),
                            status: status.into(),
                            message: format!("{}: {}", config.hostname, msg),
                            details: Some(serde_json::json!({
                                "ddns_id": config.id,
                                "hostname": config.hostname,
                            })),
                            duration_ms: dns_start.elapsed().as_millis() as u64,
                        });
                    }
                }
            }
            Err(e) => {
                checks.push(DiagnosticCheck {
                    category: "ddns".into(),
                    name: "ddns_configs_count".into(),
                    status: "error".into(),
                    message: format!("Failed to list DDNS configs: {}", e),
                    details: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    checks
}

// ============================================================================
// Category 5: omada
// ============================================================================

async fn check_omada(state: &ProxyState, include_device_tests: bool) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Controller count
    {
        let start = Instant::now();
        let controller_ids = state.omada_manager.list_controller_ids().await;
        let count = controller_ids.len();
        checks.push(DiagnosticCheck {
            category: "omada".into(),
            name: "omada_controllers_count".into(),
            status: if count > 0 { "ok" } else { "warning" }.into(),
            message: format!("{} Omada controllers registered", count),
            details: Some(serde_json::json!({ "count": count, "ids": controller_ids })),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Device count from MongoDB
    {
        let start = Instant::now();
        match state.app_state.mongo.get_omada_devices(None, None).await {
            Ok(devices) => {
                let total = devices.len();
                let online = devices.iter().filter(|d| d.status == 1).count();
                checks.push(DiagnosticCheck {
                    category: "omada".into(),
                    name: "omada_device_count".into(),
                    status: "ok".into(),
                    message: format!("{} devices ({} online)", total, online),
                    details: Some(serde_json::json!({ "total": total, "online": online })),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                checks.push(DiagnosticCheck {
                    category: "omada".into(),
                    name: "omada_device_count".into(),
                    status: "error".into(),
                    message: format!("Failed to query Omada devices: {}", e),
                    details: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // Optional: connectivity test per controller
    if include_device_tests {
        let controller_ids = state.omada_manager.list_controller_ids().await;
        for ctrl_id in &controller_ids {
            let start = Instant::now();
            match state.app_state.mongo.get_omada_controller(ctrl_id).await {
                Ok(Some(ctrl)) => {
                    let (status, msg) = match &*ctrl.status {
                        "connected" => ("ok", format!("Controller '{}' connected", ctrl.display_name)),
                        "error" => ("error", format!("Controller '{}' has error: {}", ctrl.display_name, ctrl.last_error.as_deref().unwrap_or("unknown"))),
                        s => ("warning", format!("Controller '{}' status: {}", ctrl.display_name, s)),
                    };
                    checks.push(DiagnosticCheck {
                        category: "omada".into(),
                        name: format!("omada_connectivity_{}", ctrl_id),
                        status: status.into(),
                        message: msg,
                        details: Some(serde_json::json!({
                            "controller_id": ctrl_id,
                            "display_name": ctrl.display_name,
                            "last_synced_at": ctrl.last_synced_at,
                        })),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                Ok(None) => {
                    checks.push(DiagnosticCheck {
                        category: "omada".into(),
                        name: format!("omada_connectivity_{}", ctrl_id),
                        status: "warning".into(),
                        message: format!("Controller '{}' not found in DB", ctrl_id),
                        details: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    checks.push(DiagnosticCheck {
                        category: "omada".into(),
                        name: format!("omada_connectivity_{}", ctrl_id),
                        status: "error".into(),
                        message: format!("Query failed: {}", e),
                        details: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }
    }

    checks
}

// ============================================================================
// Category 6: openwrt
// ============================================================================

async fn check_openwrt(state: &ProxyState, include_device_tests: bool) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Router count
    {
        let start = Instant::now();
        let router_ids = state.openwrt_manager.list_router_ids().await;
        let count = router_ids.len();
        checks.push(DiagnosticCheck {
            category: "openwrt".into(),
            name: "openwrt_routers_count".into(),
            status: if count > 0 { "ok" } else { "warning" }.into(),
            message: format!("{} OpenWrt routers registered", count),
            details: Some(serde_json::json!({ "count": count, "ids": router_ids })),
            duration_ms: start.elapsed().as_millis() as u64,
        });

        // Optional: connectivity test per router
        if include_device_tests {
            match state.app_state.mongo.list_openwrt_routers().await {
                Ok(routers) => {
                    for router in &routers {
                        let rt_start = Instant::now();
                        let (status, msg) = match router.status.as_str() {
                            "connected" => ("ok", format!("Router '{}' connected", router.display_name)),
                            "error" => ("error", format!("Router '{}' error: {}", router.display_name, router.last_error.as_deref().unwrap_or("unknown"))),
                            s => ("warning", format!("Router '{}' status: {}", router.display_name, s)),
                        };
                        checks.push(DiagnosticCheck {
                            category: "openwrt".into(),
                            name: format!("openwrt_connectivity_{}", router.router_id),
                            status: status.into(),
                            message: msg,
                            details: Some(serde_json::json!({
                                "router_id": router.router_id,
                                "ip": router.ip,
                            })),
                            duration_ms: rt_start.elapsed().as_millis() as u64,
                        });
                    }
                }
                Err(e) => {
                    checks.push(DiagnosticCheck {
                        category: "openwrt".into(),
                        name: "openwrt_connectivity".into(),
                        status: "error".into(),
                        message: format!("Failed to query routers: {}", e),
                        details: None,
                        duration_ms: Instant::now().elapsed().as_millis() as u64,
                    });
                }
            }
        }
    }

    checks
}

// ============================================================================
// Category 7: external
// ============================================================================

async fn check_external(state: &ProxyState, include_device_tests: bool) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Device count
    {
        let start = Instant::now();
        let device_ids = state.external_manager.list_device_ids().await;
        let count = device_ids.len();
        checks.push(DiagnosticCheck {
            category: "external".into(),
            name: "external_devices_count".into(),
            status: if count > 0 { "ok" } else { "warning" }.into(),
            message: format!("{} external devices registered", count),
            details: Some(serde_json::json!({ "count": count, "ids": device_ids })),
            duration_ms: start.elapsed().as_millis() as u64,
        });

        // Optional: connectivity test per device
        if include_device_tests {
            match state.app_state.mongo.list_external_devices().await {
                Ok(devices) => {
                    for device in &devices {
                        let dev_start = Instant::now();
                        let (status, msg) = match device.status.as_str() {
                            "connected" => ("ok", format!("Device '{}' connected", device.display_name)),
                            "error" => ("error", format!("Device '{}' error: {}", device.display_name, device.last_error.as_deref().unwrap_or("unknown"))),
                            s => ("warning", format!("Device '{}' status: {}", device.display_name, s)),
                        };
                        checks.push(DiagnosticCheck {
                            category: "external".into(),
                            name: format!("external_connectivity_{}", device.device_id),
                            status: status.into(),
                            message: msg,
                            details: Some(serde_json::json!({
                                "device_id": device.device_id,
                                "ip": device.ip,
                            })),
                            duration_ms: dev_start.elapsed().as_millis() as u64,
                        });
                    }
                }
                Err(e) => {
                    checks.push(DiagnosticCheck {
                        category: "external".into(),
                        name: "external_connectivity".into(),
                        status: "error".into(),
                        message: format!("Failed to query devices: {}", e),
                        details: None,
                        duration_ms: Instant::now().elapsed().as_millis() as u64,
                    });
                }
            }
        }
    }

    checks
}

// ============================================================================
// Category 8: aranea
// ============================================================================

async fn check_aranea(state: &ProxyState) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    let start = Instant::now();
    let config = &state.aranea_client.config;
    let tid_ok = !config.tid.is_empty();
    let lacis_id_ok = !config.tenant_lacis_id.is_empty();

    let (status, msg) = if tid_ok && lacis_id_ok {
        ("ok", format!(
            "Aranea config valid (tid={}, tenant_lacis_id={})",
            config.tid, config.tenant_lacis_id
        ))
    } else {
        let mut missing = Vec::new();
        if !tid_ok { missing.push("tid"); }
        if !lacis_id_ok { missing.push("tenant_lacis_id"); }
        ("warning", format!("Aranea config incomplete, missing: {}", missing.join(", ")))
    };

    checks.push(DiagnosticCheck {
        category: "aranea".into(),
        name: "aranea_config_valid".into(),
        status: status.into(),
        message: msg,
        details: Some(serde_json::json!({
            "tid_present": tid_ok,
            "tenant_lacis_id_present": lacis_id_ok,
            "device_gate_url": config.device_gate_url,
        })),
        duration_ms: start.elapsed().as_millis() as u64,
    });

    checks
}

// ============================================================================
// Category 9: geoip
// ============================================================================

async fn check_geoip(state: &ProxyState) -> Vec<DiagnosticCheck> {
    let start = Instant::now();
    let loaded = state.geoip.is_some();

    vec![DiagnosticCheck {
        category: "geoip".into(),
        name: "geoip_db_loaded".into(),
        status: if loaded { "ok" } else { "warning" }.into(),
        message: if loaded {
            "GeoIP database loaded".into()
        } else {
            "GeoIP database not loaded (optional feature)".into()
        },
        details: None,
        duration_ms: start.elapsed().as_millis() as u64,
    }]
}

// ============================================================================
// Category 10: system
// ============================================================================

async fn check_system(state: &ProxyState) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Uptime (from AppState.start_time)
    {
        let start = Instant::now();
        let uptime_secs = state.app_state.start_time.elapsed().as_secs();
        let hours = uptime_secs / 3600;
        let mins = (uptime_secs % 3600) / 60;

        checks.push(DiagnosticCheck {
            category: "system".into(),
            name: "system_uptime".into(),
            status: "ok".into(),
            message: format!("LPG2 uptime: {}h {}m", hours, mins),
            details: Some(serde_json::json!({ "uptime_seconds": uptime_secs })),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Disk usage (df -h /)
    {
        let start = Instant::now();
        match tokio::process::Command::new("df")
            .args(["-h", "/"])
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                // Parse second line for usage info
                let lines: Vec<&str> = stdout.lines().collect();
                let (status, msg, details) = if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 5 {
                        let usage_pct = parts[4].trim_end_matches('%').parse::<u32>().unwrap_or(0);
                        let st = if usage_pct >= 90 {
                            "error"
                        } else if usage_pct >= 75 {
                            "warning"
                        } else {
                            "ok"
                        };
                        (
                            st,
                            format!("Disk usage: {} ({}% used)", parts[2], usage_pct),
                            Some(serde_json::json!({
                                "filesystem": parts[0],
                                "size": parts[1],
                                "used": parts[2],
                                "available": parts[3],
                                "usage_percent": usage_pct,
                            })),
                        )
                    } else {
                        ("warning", "Could not parse df output".into(), None)
                    }
                } else {
                    ("warning", "Unexpected df output".into(), None)
                };

                checks.push(DiagnosticCheck {
                    category: "system".into(),
                    name: "disk_usage".into(),
                    status: status.into(),
                    message: msg,
                    details,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                checks.push(DiagnosticCheck {
                    category: "system".into(),
                    name: "disk_usage".into(),
                    status: "error".into(),
                    message: format!("Failed to run df: {}", e),
                    details: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    checks
}

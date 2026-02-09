//! Agent context API handler
//!
//! GET /api/agent/context - Returns system state for AI agents in a single call.
//! Uses existing DB accessors (SSoT) to aggregate data from multiple sources.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::AuthUser;
use crate::proxy::ProxyState;

/// Query parameter to select which sections to include
#[derive(Debug, Deserialize)]
pub struct AgentContextQuery {
    /// Comma-separated section names: routes,ddns,omada,openwrt,external,security,settings,diagnostics
    pub sections: Option<String>,
}

/// Permission level info for the available_endpoints list
#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub method: String,
    pub path: String,
    pub required_permission: i32,
    pub description: String,
}

/// System-level context (always included)
#[derive(Debug, Serialize)]
pub struct SystemContext {
    pub version: String,
    pub uptime_seconds: u64,
    pub server_health: String,
    pub auth_user: AuthUser,
    pub available_endpoints: Vec<EndpointInfo>,
}

/// Full agent context response
#[derive(Debug, Serialize)]
pub struct AgentContext {
    pub system: SystemContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ddns: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omada: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openwrt: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
}

/// GET /api/agent/context - Aggregated system context for AI agents
pub async fn get_agent_context(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<AgentContextQuery>,
) -> Result<impl IntoResponse, AppError> {
    let sections: Vec<String> = query
        .sections
        .as_deref()
        .unwrap_or("routes,ddns,omada,openwrt,external,security,settings")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();

    // System context (always included)
    let uptime_seconds = state.app_state.start_time.elapsed().as_secs();

    let available_endpoints = build_endpoint_list(user.permission);

    let system = SystemContext {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        server_health: "ok".to_string(),
        auth_user: user,
        available_endpoints,
    };

    // Collect requested sections
    let routes = if sections.contains(&"routes".to_string()) {
        let r = state
            .app_state
            .mysql
            .list_routes()
            .await
            .unwrap_or_default();
        Some(serde_json::to_value(r).unwrap_or_default())
    } else {
        None
    };

    let ddns = if sections.contains(&"ddns".to_string()) {
        let d = state.app_state.mysql.list_ddns().await.unwrap_or_default();
        // Mask sensitive fields
        let masked: Vec<_> = d
            .into_iter()
            .map(|mut c| {
                c.password = c.password.as_ref().map(|_| "********".to_string());
                c.api_token = c.api_token.as_ref().map(|_| "********".to_string());
                c
            })
            .collect();
        Some(serde_json::to_value(masked).unwrap_or_default())
    } else {
        None
    };

    let omada = if sections.contains(&"omada".to_string()) {
        let controllers = state
            .app_state
            .mongo
            .list_omada_controllers()
            .await
            .unwrap_or_default();
        let devices = state
            .app_state
            .mongo
            .get_omada_devices(None, None)
            .await
            .unwrap_or_default();
        let clients = state
            .app_state
            .mongo
            .get_omada_clients(None, None, None)
            .await
            .unwrap_or_default();
        Some(serde_json::json!({
            "controllers": controllers.len(),
            "devices": devices.len(),
            "clients": clients.len(),
            "controller_list": controllers,
        }))
    } else {
        None
    };

    let openwrt = if sections.contains(&"openwrt".to_string()) {
        let routers = state
            .app_state
            .mongo
            .list_openwrt_routers()
            .await
            .unwrap_or_default();
        let clients = state
            .app_state
            .mongo
            .get_openwrt_clients(None)
            .await
            .unwrap_or_default();
        Some(serde_json::json!({
            "routers": routers.len(),
            "clients": clients.len(),
            "router_list": routers,
        }))
    } else {
        None
    };

    let external = if sections.contains(&"external".to_string()) {
        let devices = state
            .app_state
            .mongo
            .list_external_devices()
            .await
            .unwrap_or_default();
        let clients = state
            .app_state
            .mongo
            .get_external_clients(None)
            .await
            .unwrap_or_default();
        Some(serde_json::json!({
            "devices": devices.len(),
            "clients": clients.len(),
            "device_list": devices,
        }))
    } else {
        None
    };

    let security = if sections.contains(&"security".to_string()) {
        let blocked = state
            .app_state
            .mysql
            .list_blocked_ips()
            .await
            .unwrap_or_default();
        let recent_events = state
            .app_state
            .mongo
            .get_security_events(10, 0)
            .await
            .unwrap_or_default();
        Some(serde_json::json!({
            "blocked_ips": blocked,
            "recent_events": recent_events,
        }))
    } else {
        None
    };

    let settings = if sections.contains(&"settings".to_string()) {
        let s = state
            .app_state
            .mysql
            .list_settings()
            .await
            .unwrap_or_default();
        // Mask Discord webhook URL
        let masked: Vec<_> = s
            .into_iter()
            .map(|mut setting| {
                if setting.setting_key == "discord_webhook_url" && setting.setting_value.is_some() {
                    setting.setting_value = Some("********".to_string());
                }
                setting
            })
            .collect();
        Some(serde_json::to_value(masked).unwrap_or_default())
    } else {
        None
    };

    Ok(Json(AgentContext {
        system,
        routes,
        ddns,
        omada,
        openwrt,
        external,
        security,
        settings,
    }))
}

/// Helper macro: build EndpointInfo concisely
fn ep(method: &str, path: &str, perm: i32, desc: &str) -> EndpointInfo {
    EndpointInfo {
        method: method.into(),
        path: path.into(),
        required_permission: perm,
        description: desc.into(),
    }
}

/// Build list of available endpoints filtered by user permission.
/// This list MUST stay in sync with api/mod.rs route definitions (SSoT check).
fn build_endpoint_list(permission: i32) -> Vec<EndpointInfo> {
    let all_endpoints = vec![
        // ======== Read (>= 0) — GET endpoints, no mutation ========
        // Auth
        ep("GET", "/api/auth/me", 0, "Current user info"),
        // Routes
        ep("GET", "/api/routes", 0, "List proxy routes"),
        ep("GET", "/api/routes/:id", 0, "Get single route"),
        ep("GET", "/api/routes/status", 0, "All routes health status"),
        ep(
            "GET",
            "/api/routes/:id/status",
            0,
            "Single route health status",
        ),
        ep("GET", "/api/routes/:id/logs", 0, "Route access logs"),
        ep("GET", "/api/server-routes", 0, "Routes with subnet info"),
        // DDNS
        ep("GET", "/api/ddns", 0, "List DDNS configurations"),
        ep("GET", "/api/ddns/:id", 0, "Get single DDNS config"),
        ep(
            "GET",
            "/api/ddns/integrated",
            0,
            "DDNS with Omada WAN IP comparison",
        ),
        ep(
            "GET",
            "/api/ddns/:id/port-forwards",
            0,
            "Port forwarding rules for DDNS",
        ),
        // Security
        ep("GET", "/api/security/blocked-ips", 0, "List blocked IPs"),
        ep("GET", "/api/security/events", 0, "List security events"),
        ep(
            "GET",
            "/api/security/events/ip/:ip",
            0,
            "Security events by IP",
        ),
        ep(
            "GET",
            "/api/security/events/search",
            0,
            "Advanced security event search",
        ),
        // Settings
        ep("GET", "/api/settings", 0, "List all settings"),
        ep("GET", "/api/settings/restart", 0, "Get restart settings"),
        // Dashboard
        ep("GET", "/api/dashboard/stats", 0, "Dashboard statistics"),
        ep("GET", "/api/dashboard/access-log", 0, "Access log entries"),
        ep(
            "GET",
            "/api/dashboard/access-log/filter",
            0,
            "Filtered access log",
        ),
        ep(
            "GET",
            "/api/dashboard/access-log/search",
            0,
            "Advanced access log search",
        ),
        ep(
            "GET",
            "/api/dashboard/access-log/export",
            0,
            "Export access log CSV",
        ),
        ep("GET", "/api/dashboard/health", 0, "Health status"),
        ep(
            "GET",
            "/api/dashboard/status-distribution",
            0,
            "HTTP status distribution",
        ),
        ep(
            "GET",
            "/api/dashboard/hourly-stats",
            0,
            "Hourly request stats",
        ),
        ep("GET", "/api/dashboard/top-ips", 0, "Top IP addresses"),
        ep("GET", "/api/dashboard/top-paths", 0, "Top request paths"),
        ep("GET", "/api/dashboard/error-summary", 0, "Error summary"),
        ep(
            "GET",
            "/api/dashboard/ssl-status",
            0,
            "SSL certificate status",
        ),
        ep(
            "GET",
            "/api/dashboard/server-health",
            0,
            "Server health metrics",
        ),
        // Omada
        ep("GET", "/api/omada/controllers", 0, "List Omada controllers"),
        ep(
            "GET",
            "/api/omada/controllers/:id",
            0,
            "Get single controller",
        ),
        ep("GET", "/api/omada/devices", 0, "List Omada devices"),
        ep("GET", "/api/omada/clients", 0, "List Omada clients"),
        ep(
            "GET",
            "/api/omada/wireguard",
            0,
            "List Omada WireGuard peers",
        ),
        ep("GET", "/api/omada/summary", 0, "Omada network summary"),
        ep("GET", "/api/omada/status", 0, "Legacy network status"),
        // OpenWrt
        ep("GET", "/api/openwrt/routers", 0, "List OpenWrt routers"),
        ep("GET", "/api/openwrt/routers/:id", 0, "Get single router"),
        ep("GET", "/api/openwrt/clients", 0, "List OpenWrt clients"),
        ep("GET", "/api/openwrt/summary", 0, "OpenWrt summary"),
        // External
        ep("GET", "/api/external/devices", 0, "List external devices"),
        ep("GET", "/api/external/devices/:id", 0, "Get single device"),
        ep("GET", "/api/external/clients", 0, "List external clients"),
        ep(
            "GET",
            "/api/external/summary",
            0,
            "External devices summary",
        ),
        // WireGuard
        ep("GET", "/api/wireguard/peers", 0, "List WireGuard peers"),
        ep(
            "GET",
            "/api/wireguard/interfaces",
            0,
            "List WireGuard interfaces",
        ),
        // Aranea
        ep("GET", "/api/aranea/devices", 0, "List aranea devices"),
        ep(
            "GET",
            "/api/aranea/devices/:lacis_id/state",
            0,
            "Get device state",
        ),
        ep("GET", "/api/aranea/summary", 0, "Aranea config summary"),
        // LacisID
        ep("GET", "/api/lacis-id/candidates", 0, "LacisID candidates"),
        // Topology
        ep(
            "GET",
            "/api/topology",
            0,
            "Network topology (CelestialGlobe)",
        ),
        // Audit & logs
        ep("GET", "/api/audit", 0, "Audit logs"),
        ep("GET", "/api/logs/operations", 0, "Operation logs"),
        ep(
            "GET",
            "/api/logs/operations/summary",
            0,
            "Operation logs summary",
        ),
        ep("GET", "/api/my-ip", 0, "Detect client/server IP"),
        // Nginx (read)
        ep("GET", "/api/nginx/status", 0, "Nginx status"),
        ep("GET", "/api/nginx/config", 0, "Nginx config content"),
        ep(
            "GET",
            "/api/nginx/template-settings",
            0,
            "Nginx template settings",
        ),
        // Agent
        ep(
            "GET",
            "/api/agent/context",
            0,
            "Agent context (this endpoint)",
        ),
        // ======== Operate (>= 50) — sync triggers, diagnostics, network tools ========
        ep("POST", "/api/tools/sync/omada", 50, "Trigger Omada sync"),
        ep(
            "POST",
            "/api/tools/sync/openwrt",
            50,
            "Trigger OpenWrt sync",
        ),
        ep(
            "POST",
            "/api/tools/sync/external",
            50,
            "Trigger External sync",
        ),
        ep(
            "POST",
            "/api/tools/ddns/update-all",
            50,
            "Trigger all DDNS updates",
        ),
        ep(
            "POST",
            "/api/tools/network/ping",
            50,
            "Ping host from server",
        ),
        ep("POST", "/api/tools/network/dns", 50, "DNS lookup"),
        ep(
            "POST",
            "/api/tools/diagnostics",
            50,
            "Run system diagnostics",
        ),
        ep(
            "POST",
            "/api/ddns/:id/update",
            50,
            "Trigger single DDNS update",
        ),
        ep(
            "POST",
            "/api/omada/controllers/:id/sync",
            50,
            "Sync single Omada controller",
        ),
        ep(
            "POST",
            "/api/openwrt/routers/:id/poll",
            50,
            "Poll single OpenWrt router",
        ),
        ep(
            "POST",
            "/api/external/devices/:id/poll",
            50,
            "Poll single external device",
        ),
        // Connection tests (no permission, read-only side effect)
        ep(
            "POST",
            "/api/omada/controllers/test",
            0,
            "Test Omada connection (pre-registration)",
        ),
        ep("POST", "/api/omada/test", 0, "Legacy Omada connection test"),
        ep(
            "POST",
            "/api/openwrt/routers/test",
            0,
            "Test OpenWrt SSH connection",
        ),
        ep(
            "POST",
            "/api/external/devices/test",
            0,
            "Test external device connection",
        ),
        // Pure computation (no side effect)
        ep("POST", "/api/lacis-id/compute", 0, "Compute LacisID"),
        ep(
            "POST",
            "/api/wireguard/keypair",
            0,
            "Generate WireGuard key pair",
        ),
        ep(
            "POST",
            "/api/wireguard/config",
            0,
            "Generate WireGuard client config",
        ),
        // ======== Admin (>= 80) — CRUD create/update, config changes ========
        ep("POST", "/api/routes", 80, "Create proxy route"),
        ep("PUT", "/api/routes/:id", 80, "Update proxy route"),
        ep("POST", "/api/ddns", 80, "Create DDNS configuration"),
        ep("PUT", "/api/ddns/:id", 80, "Update DDNS configuration"),
        ep(
            "PUT",
            "/api/ddns/:id/link-omada",
            80,
            "Link DDNS to Omada controller",
        ),
        ep(
            "POST",
            "/api/security/blocked-ips",
            80,
            "Block an IP address",
        ),
        ep("PUT", "/api/settings/:key", 80, "Update setting"),
        ep(
            "PUT",
            "/api/settings/restart",
            80,
            "Update restart settings",
        ),
        ep(
            "POST",
            "/api/settings/test-discord",
            80,
            "Test Discord notification",
        ),
        ep(
            "POST",
            "/api/omada/controllers",
            80,
            "Register Omada controller",
        ),
        ep(
            "POST",
            "/api/openwrt/routers",
            80,
            "Register OpenWrt router",
        ),
        ep(
            "POST",
            "/api/external/devices",
            80,
            "Register external device",
        ),
        ep("POST", "/api/aranea/register", 80, "Register aranea device"),
        ep(
            "POST",
            "/api/lacis-id/assign/:device_id",
            80,
            "Assign LacisID to device",
        ),
        ep("POST", "/api/wireguard/peers", 80, "Create WireGuard peer"),
        ep(
            "PUT",
            "/api/wireguard/peers/:id",
            80,
            "Update WireGuard peer",
        ),
        ep(
            "POST",
            "/api/nginx/enable-full-proxy",
            80,
            "Enable full proxy mode",
        ),
        ep("POST", "/api/nginx/reload", 80, "Reload nginx"),
        ep("POST", "/api/nginx/test", 80, "Test nginx config"),
        ep("PUT", "/api/nginx/body-size", 80, "Update nginx body size"),
        ep(
            "PUT",
            "/api/nginx/template-settings",
            80,
            "Update nginx template settings",
        ),
        ep(
            "POST",
            "/api/nginx/regenerate",
            80,
            "Regenerate nginx config",
        ),
        // ======== Dangerous (== 100) — DELETE operations, confirm required ========
        ep(
            "DELETE",
            "/api/routes/:id",
            100,
            "Delete proxy route (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/ddns/:id",
            100,
            "Delete DDNS config (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/security/blocked-ips/:id",
            100,
            "Unblock IP (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/omada/controllers/:id",
            100,
            "Delete Omada controller (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/openwrt/routers/:id",
            100,
            "Delete OpenWrt router (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/external/devices/:id",
            100,
            "Delete external device (confirm required)",
        ),
        ep(
            "DELETE",
            "/api/wireguard/peers/:id",
            100,
            "Delete WireGuard peer (confirm required)",
        ),
        ep("POST", "/api/auth/api-key", 100, "Issue API key"),
        ep(
            "POST",
            "/api/settings/restart/trigger",
            100,
            "Trigger service restart",
        ),
    ];

    all_endpoints
        .into_iter()
        .filter(|e| permission >= e.required_permission)
        .collect()
}

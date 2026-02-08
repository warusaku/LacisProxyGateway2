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
        let r = state.app_state.mysql.list_routes().await.unwrap_or_default();
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
        let s = state.app_state.mysql.list_settings().await.unwrap_or_default();
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

/// Build list of available endpoints filtered by user permission
fn build_endpoint_list(permission: i32) -> Vec<EndpointInfo> {
    let all_endpoints = vec![
        // Read (>= 0)
        EndpointInfo { method: "GET".into(), path: "/api/routes".into(), required_permission: 0, description: "List proxy routes".into() },
        EndpointInfo { method: "GET".into(), path: "/api/ddns".into(), required_permission: 0, description: "List DDNS configurations".into() },
        EndpointInfo { method: "GET".into(), path: "/api/dashboard/stats".into(), required_permission: 0, description: "Dashboard statistics".into() },
        EndpointInfo { method: "GET".into(), path: "/api/security/blocked-ips".into(), required_permission: 0, description: "List blocked IPs".into() },
        EndpointInfo { method: "GET".into(), path: "/api/settings".into(), required_permission: 0, description: "List settings".into() },
        EndpointInfo { method: "GET".into(), path: "/api/omada/summary".into(), required_permission: 0, description: "Omada network summary".into() },
        EndpointInfo { method: "GET".into(), path: "/api/openwrt/summary".into(), required_permission: 0, description: "OpenWrt summary".into() },
        EndpointInfo { method: "GET".into(), path: "/api/external/summary".into(), required_permission: 0, description: "External devices summary".into() },
        EndpointInfo { method: "GET".into(), path: "/api/topology".into(), required_permission: 0, description: "Network topology".into() },
        EndpointInfo { method: "GET".into(), path: "/api/agent/context".into(), required_permission: 0, description: "Agent context (this endpoint)".into() },
        // Operate (>= 50)
        EndpointInfo { method: "POST".into(), path: "/api/tools/sync/omada".into(), required_permission: 50, description: "Trigger Omada sync".into() },
        EndpointInfo { method: "POST".into(), path: "/api/tools/sync/openwrt".into(), required_permission: 50, description: "Trigger OpenWrt sync".into() },
        EndpointInfo { method: "POST".into(), path: "/api/tools/diagnostics".into(), required_permission: 50, description: "Run diagnostics".into() },
        // Admin (>= 80)
        EndpointInfo { method: "POST".into(), path: "/api/routes".into(), required_permission: 80, description: "Create proxy route".into() },
        EndpointInfo { method: "PUT".into(), path: "/api/routes/:id".into(), required_permission: 80, description: "Update proxy route".into() },
        EndpointInfo { method: "POST".into(), path: "/api/ddns".into(), required_permission: 80, description: "Create DDNS configuration".into() },
        EndpointInfo { method: "PUT".into(), path: "/api/ddns/:id".into(), required_permission: 80, description: "Update DDNS configuration".into() },
        EndpointInfo { method: "PUT".into(), path: "/api/settings/:key".into(), required_permission: 80, description: "Update setting".into() },
        EndpointInfo { method: "POST".into(), path: "/api/nginx/reload".into(), required_permission: 80, description: "Reload nginx".into() },
        EndpointInfo { method: "POST".into(), path: "/api/nginx/regenerate".into(), required_permission: 80, description: "Regenerate nginx config".into() },
        // Dangerous (== 100)
        EndpointInfo { method: "DELETE".into(), path: "/api/routes/:id".into(), required_permission: 100, description: "Delete proxy route (confirm required)".into() },
        EndpointInfo { method: "DELETE".into(), path: "/api/ddns/:id".into(), required_permission: 100, description: "Delete DDNS configuration (confirm required)".into() },
        EndpointInfo { method: "DELETE".into(), path: "/api/security/blocked-ips/:id".into(), required_permission: 100, description: "Unblock IP (confirm required)".into() },
        EndpointInfo { method: "DELETE".into(), path: "/api/omada/controllers/:id".into(), required_permission: 100, description: "Delete Omada controller (confirm required)".into() },
        EndpointInfo { method: "DELETE".into(), path: "/api/openwrt/routers/:id".into(), required_permission: 100, description: "Delete OpenWrt router (confirm required)".into() },
        EndpointInfo { method: "DELETE".into(), path: "/api/external/devices/:id".into(), required_permission: 100, description: "Delete external device (confirm required)".into() },
        EndpointInfo { method: "POST".into(), path: "/api/auth/api-key".into(), required_permission: 100, description: "Issue API key".into() },
        EndpointInfo { method: "POST".into(), path: "/api/settings/restart/trigger".into(), required_permission: 100, description: "Trigger service restart".into() },
    ];

    all_endpoints
        .into_iter()
        .filter(|e| permission >= e.required_permission)
        .collect()
}

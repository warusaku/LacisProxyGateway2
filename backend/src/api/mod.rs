//! API module - HTTP handlers and routes

pub mod handlers;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::proxy::ProxyState;

pub fn routes(state: ProxyState) -> Router<ProxyState> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health_check))
        .route("/api/health", get(handlers::health_check))
        // Proxy routes management
        .route("/api/routes", get(handlers::list_routes))
        .route("/api/routes", post(handlers::create_route))
        .route("/api/routes/:id", get(handlers::get_route))
        .route("/api/routes/:id", put(handlers::update_route))
        .route("/api/routes/:id", delete(handlers::delete_route))
        // DDNS management
        .route("/api/ddns", get(handlers::list_ddns))
        .route("/api/ddns", post(handlers::create_ddns))
        .route("/api/ddns/:id", get(handlers::get_ddns))
        .route("/api/ddns/:id", put(handlers::update_ddns))
        .route("/api/ddns/:id", delete(handlers::delete_ddns))
        .route("/api/ddns/:id/update", post(handlers::trigger_ddns_update))
        // Security
        .route("/api/security/blocked-ips", get(handlers::list_blocked_ips))
        .route("/api/security/blocked-ips", post(handlers::block_ip))
        .route(
            "/api/security/blocked-ips/:id",
            delete(handlers::unblock_ip),
        )
        .route("/api/security/events", get(handlers::list_security_events))
        .route(
            "/api/security/events/ip/:ip",
            get(handlers::get_security_events_by_ip),
        )
        // Settings
        .route("/api/settings", get(handlers::list_settings))
        .route("/api/settings/:key", put(handlers::update_setting))
        .route(
            "/api/settings/test-discord",
            post(handlers::test_discord_notification),
        )
        // Restart settings
        .route("/api/settings/restart", get(handlers::get_restart_settings))
        .route("/api/settings/restart", put(handlers::update_restart_settings))
        .route(
            "/api/settings/restart/trigger",
            post(handlers::trigger_manual_restart),
        )
        // Audit
        .route("/api/audit", get(handlers::get_audit_logs))
        // Dashboard
        .route("/api/dashboard/stats", get(handlers::get_dashboard_stats))
        .route("/api/dashboard/access-log", get(handlers::get_access_log))
        .route(
            "/api/dashboard/access-log/filter",
            get(handlers::get_filtered_access_log),
        )
        .route("/api/dashboard/health", get(handlers::get_health_status))
        .route(
            "/api/dashboard/status-distribution",
            get(handlers::get_status_distribution),
        )
        .route("/api/dashboard/ssl-status", get(handlers::get_ssl_status))
        .route("/api/dashboard/server-health", get(handlers::get_server_health))
        // Omada
        .route("/api/omada/status", get(handlers::get_network_status))
        .route("/api/omada/test", post(handlers::test_connection))
        // Nginx management
        .route("/api/nginx/status", get(handlers::get_nginx_status))
        .route("/api/nginx/config", get(handlers::get_nginx_config))
        .route("/api/nginx/enable-full-proxy", post(handlers::enable_full_proxy))
        .route("/api/nginx/reload", post(handlers::reload_nginx_handler))
        .route("/api/nginx/test", post(handlers::test_nginx_config_handler))
        .route("/api/nginx/body-size", put(handlers::update_body_size))
}

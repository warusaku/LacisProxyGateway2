//! API module - HTTP handlers and routes

pub(crate) mod admin_guard;
pub(crate) mod auth_middleware;
pub mod handlers;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};

use crate::proxy::ProxyState;

pub fn routes(state: ProxyState) -> Router<ProxyState> {
    // ========================================================================
    // Group 1: Public routes - no auth, no network guard (health checks)
    // ========================================================================
    let public = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/health", get(handlers::health_check));

    // ========================================================================
    // Group 2: Auth endpoints - internet_access_guard only (no require_auth)
    // Login endpoints must be accessible without an existing session
    // ========================================================================
    let auth_open = Router::new()
        .route("/api/auth/login/local", post(handlers::auth::login_local))
        .route(
            "/api/auth/login/lacisoath",
            post(handlers::auth::login_lacisoath),
        )
        .route(
            "/api/auth/lacisoath-config",
            get(handlers::auth::lacisoath_config),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_guard::internet_access_guard,
        ));

    // ========================================================================
    // Group 3: Protected routes - internet_access_guard + require_auth
    // All admin/data endpoints require authentication
    // ========================================================================
    let protected = Router::new()
        // Auth session endpoints
        .route("/api/auth/me", get(handlers::auth::auth_me))
        .route("/api/auth/logout", post(handlers::auth::auth_logout))
        // Server routes (enhanced with subnet info)
        .route("/api/server-routes", get(handlers::list_server_routes))
        // Proxy routes management
        .route("/api/routes", get(handlers::list_routes))
        .route("/api/routes", post(handlers::create_route))
        .route("/api/routes/status", get(handlers::get_all_routes_status))
        .route("/api/routes/:id", get(handlers::get_route))
        .route("/api/routes/:id", put(handlers::update_route))
        .route("/api/routes/:id", delete(handlers::delete_route))
        .route("/api/routes/:id/status", get(handlers::get_route_status))
        .route("/api/routes/:id/logs", get(handlers::get_route_logs))
        // DDNS management
        .route("/api/ddns", get(handlers::list_ddns))
        .route("/api/ddns", post(handlers::create_ddns))
        .route("/api/ddns/:id", get(handlers::get_ddns))
        .route("/api/ddns/:id", put(handlers::update_ddns))
        .route("/api/ddns/:id", delete(handlers::delete_ddns))
        .route("/api/ddns/:id/update", post(handlers::trigger_ddns_update))
        .route("/api/ddns/integrated", get(handlers::list_ddns_integrated))
        .route(
            "/api/ddns/:id/link-omada",
            put(handlers::link_ddns_omada),
        )
        .route(
            "/api/ddns/:id/port-forwards",
            get(handlers::get_ddns_port_forwards),
        )
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
        .route(
            "/api/security/events/search",
            get(handlers::search_security_events),
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
        .route(
            "/api/settings/restart",
            put(handlers::update_restart_settings),
        )
        .route(
            "/api/settings/restart/trigger",
            post(handlers::trigger_manual_restart),
        )
        // Audit
        .route("/api/audit", get(handlers::get_audit_logs))
        // My IP (client IP detection)
        .route("/api/my-ip", get(handlers::get_my_ip))
        // Dashboard
        .route("/api/dashboard/stats", get(handlers::get_dashboard_stats))
        .route("/api/dashboard/access-log", get(handlers::get_access_log))
        .route(
            "/api/dashboard/access-log/filter",
            get(handlers::get_filtered_access_log),
        )
        .route(
            "/api/dashboard/access-log/search",
            get(handlers::search_access_log),
        )
        .route(
            "/api/dashboard/access-log/export",
            get(handlers::export_access_log),
        )
        .route("/api/dashboard/health", get(handlers::get_health_status))
        .route(
            "/api/dashboard/status-distribution",
            get(handlers::get_status_distribution),
        )
        .route(
            "/api/dashboard/hourly-stats",
            get(handlers::get_hourly_stats),
        )
        .route("/api/dashboard/top-ips", get(handlers::get_top_ips))
        .route("/api/dashboard/top-paths", get(handlers::get_top_paths))
        .route(
            "/api/dashboard/error-summary",
            get(handlers::get_error_summary),
        )
        .route(
            "/api/dashboard/ssl-status",
            get(handlers::get_ssl_status),
        )
        .route(
            "/api/dashboard/server-health",
            get(handlers::get_server_health),
        )
        // Omada: Controller management
        .route("/api/omada/controllers", post(handlers::register_controller))
        .route("/api/omada/controllers", get(handlers::list_controllers))
        .route(
            "/api/omada/controllers/test",
            post(handlers::test_controller_connection),
        )
        .route(
            "/api/omada/controllers/:id",
            get(handlers::get_controller),
        )
        .route(
            "/api/omada/controllers/:id",
            delete(handlers::delete_controller),
        )
        .route(
            "/api/omada/controllers/:id/sync",
            post(handlers::sync_controller),
        )
        // Omada: Data viewing
        .route("/api/omada/devices", get(handlers::get_omada_devices))
        .route("/api/omada/clients", get(handlers::get_omada_clients))
        .route("/api/omada/wireguard", get(handlers::get_omada_wireguard))
        .route("/api/omada/summary", get(handlers::get_omada_summary))
        // Omada: Legacy compatibility
        .route("/api/omada/status", get(handlers::get_network_status))
        .route("/api/omada/test", post(handlers::test_connection))
        // OpenWrt: Router management
        .route("/api/openwrt/routers", post(handlers::openwrt::register_router))
        .route("/api/openwrt/routers", get(handlers::openwrt::list_routers))
        .route(
            "/api/openwrt/routers/test",
            post(handlers::openwrt::test_router_connection),
        )
        .route(
            "/api/openwrt/routers/:id",
            get(handlers::openwrt::get_router),
        )
        .route(
            "/api/openwrt/routers/:id",
            delete(handlers::openwrt::delete_router),
        )
        .route(
            "/api/openwrt/routers/:id/poll",
            post(handlers::openwrt::poll_router),
        )
        .route("/api/openwrt/clients", get(handlers::openwrt::get_openwrt_clients))
        .route("/api/openwrt/summary", get(handlers::openwrt::get_openwrt_summary))
        // WireGuard: Key gen, peer CRUD, config
        .route("/api/wireguard/keypair", post(handlers::wireguard::generate_keypair))
        .route("/api/wireguard/peers", post(handlers::wireguard::create_peer))
        .route("/api/wireguard/peers", get(handlers::wireguard::get_peers))
        .route(
            "/api/wireguard/peers/:id",
            put(handlers::wireguard::update_peer),
        )
        .route(
            "/api/wireguard/peers/:id",
            delete(handlers::wireguard::delete_peer),
        )
        .route("/api/wireguard/config", post(handlers::wireguard::generate_config))
        .route("/api/wireguard/interfaces", get(handlers::wireguard::get_interfaces))
        // External: Device management
        .route("/api/external/devices", post(handlers::external::register_device))
        .route("/api/external/devices", get(handlers::external::list_devices))
        .route(
            "/api/external/devices/test",
            post(handlers::external::test_device_connection),
        )
        .route(
            "/api/external/devices/:id",
            get(handlers::external::get_device),
        )
        .route(
            "/api/external/devices/:id",
            delete(handlers::external::delete_device),
        )
        .route(
            "/api/external/devices/:id/poll",
            post(handlers::external::poll_device),
        )
        .route("/api/external/clients", get(handlers::external::get_external_clients))
        .route("/api/external/summary", get(handlers::external::get_external_summary))
        // Topology (CelestialGlobe)
        .route("/api/topology", get(handlers::get_topology))
        // araneaSDK
        .route("/api/aranea/register", post(handlers::aranea_register_device))
        .route("/api/aranea/devices", get(handlers::aranea_list_devices))
        .route(
            "/api/aranea/devices/:lacis_id/state",
            get(handlers::aranea_get_device_state),
        )
        .route("/api/aranea/summary", get(handlers::aranea_summary))
        // Tools: sync triggers + network diagnostics
        .route("/api/tools/sync/omada", post(handlers::tool_sync_omada))
        .route("/api/tools/sync/openwrt", post(handlers::tool_sync_openwrt))
        .route("/api/tools/sync/external", post(handlers::tool_sync_external))
        .route("/api/tools/ddns/update-all", post(handlers::tool_ddns_update_all))
        .route("/api/tools/network/ping", post(handlers::tool_network_ping))
        .route("/api/tools/network/dns", post(handlers::tool_network_dns))
        // Operation logs
        .route("/api/logs/operations", get(handlers::list_operation_logs))
        .route("/api/logs/operations/summary", get(handlers::get_operation_logs_summary))
        // LacisID
        .route("/api/lacis-id/candidates", get(handlers::lacis_id_candidates))
        .route("/api/lacis-id/compute", post(handlers::lacis_id_compute))
        .route("/api/lacis-id/assign/:device_id", post(handlers::lacis_id_assign))
        // Nginx management
        .route("/api/nginx/status", get(handlers::get_nginx_status))
        .route("/api/nginx/config", get(handlers::get_nginx_config))
        .route(
            "/api/nginx/enable-full-proxy",
            post(handlers::enable_full_proxy),
        )
        .route("/api/nginx/reload", post(handlers::reload_nginx_handler))
        .route("/api/nginx/test", post(handlers::test_nginx_config_handler))
        .route("/api/nginx/body-size", put(handlers::update_body_size))
        .route(
            "/api/nginx/template-settings",
            get(handlers::get_nginx_template_settings),
        )
        .route(
            "/api/nginx/template-settings",
            put(handlers::update_nginx_template_settings),
        )
        .route(
            "/api/nginx/regenerate",
            post(handlers::regenerate_nginx_config),
        )
        // Apply middleware layers (order: inner first, so require_auth runs before internet_access_guard)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware::require_auth,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_guard::internet_access_guard,
        ));

    public.merge(auth_open).merge(protected)
}

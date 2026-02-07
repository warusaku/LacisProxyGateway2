//! LacisProxyGateway2 - Reverse Proxy Gateway
//!
//! A reverse proxy gateway with DDNS integration, traffic routing,
//! security monitoring, and Discord notifications.

mod api;
mod config;
mod db;
mod ddns;
mod error;
mod geoip;
mod health;
mod models;
mod notify;
mod omada;
mod proxy;
mod restart;

use std::net::SocketAddr;
use std::sync::Arc;

use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::AppState;
use crate::ddns::DdnsUpdater;
use crate::health::HealthChecker;
use crate::notify::DiscordNotifier;
use crate::proxy::ProxyState;
use crate::restart::RestartScheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lacis_proxy_gateway=info,tower_http=debug".into()),
        )
        .init();

    tracing::info!("Starting LacisProxyGateway2...");

    // Load configuration
    let config = config::Config::load()?;
    tracing::info!("Configuration loaded");

    // Initialize database connections
    let app_state = AppState::new(&config).await?;
    tracing::info!("Database connections established");

    // Initialize notifier
    let notifier = Arc::new(DiscordNotifier::new(app_state.clone()));

    // Initialize proxy state (includes DdnsUpdater and optional GeoIP)
    let proxy_state = ProxyState::new(
        app_state.clone(),
        notifier.clone(),
        config.server.geoip_db_path.as_deref(),
    )
    .await?;
    let route_count = proxy_state.router.read().await.len();
    tracing::info!(
        "Proxy router initialized with {} active routes",
        route_count
    );

    // Start background tasks (use the same DdnsUpdater from proxy_state)
    start_background_tasks(app_state.clone(), notifier.clone(), proxy_state.ddns_updater.clone());

    // Build application router
    let cors = CorsLayer::permissive();

    let api_routes = api::routes(proxy_state.clone());

    // Build router with proxy fallback
    let app = api_routes
        .fallback(proxy::proxy_handler)
        .with_state(proxy_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Start background tasks (DDNS updater, health checker, restart scheduler)
fn start_background_tasks(
    app_state: AppState,
    notifier: Arc<DiscordNotifier>,
    ddns_updater: Arc<DdnsUpdater>,
) {
    // DDNS updater (use shared instance)
    tokio::spawn(async move {
        ddns_updater.start().await;
    });

    // Health checker
    let health_checker = Arc::new(HealthChecker::new(app_state.clone(), notifier));
    tokio::spawn(async move {
        health_checker.start().await;
    });

    // Restart scheduler
    let restart_scheduler = Arc::new(RestartScheduler::new(app_state.mysql.clone()));
    tokio::spawn(async move {
        restart_scheduler.start_monitoring().await;
    });

    tracing::info!("Background tasks started");
}

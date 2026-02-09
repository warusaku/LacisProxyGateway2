//! LacisProxyGateway2 - Reverse Proxy Gateway
//!
//! A reverse proxy gateway with DDNS integration, traffic routing,
//! security monitoring, and Discord notifications.

mod api;
mod aranea;
mod config;
mod db;
mod ddns;
mod error;
mod external;
mod geoip;
mod health;
mod lacis_id;
mod models;
mod node_order;
mod notify;
mod user_object_ingester;
mod omada;
mod openwrt;
mod proxy;
mod restart;
mod wireguard;

use std::net::SocketAddr;
use std::sync::Arc;

use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::AppState;
use crate::ddns::DdnsUpdater;
use crate::external::{ExternalDeviceManager, ExternalSyncer};
use crate::health::HealthChecker;
use crate::notify::DiscordNotifier;
use crate::omada::{OmadaManager, OmadaSyncer};
use crate::openwrt::{OpenWrtManager, OpenWrtSyncer};
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

    // Initialize OmadaManager (multi-controller management)
    let omada_manager = Arc::new(OmadaManager::new(app_state.mongo.clone()));

    // MySQL → MongoDB migration (one-time, startup)
    match omada_manager.migrate_from_mysql(&app_state.mysql).await {
        Ok(true) => tracing::info!("Migrated omada_config from MySQL to MongoDB"),
        Ok(false) => tracing::debug!("Omada MySQL migration skipped (already migrated or empty)"),
        Err(e) => tracing::warn!("Omada MySQL migration failed (non-fatal): {}", e),
    }

    // Load existing controllers from MongoDB
    match omada_manager.load_all().await {
        Ok(count) => tracing::info!("OmadaManager loaded {} controllers", count),
        Err(e) => tracing::warn!("OmadaManager load failed (non-fatal): {}", e),
    }

    // Initialize OpenWrtManager (multi-router SSH management)
    let openwrt_manager = Arc::new(OpenWrtManager::new(app_state.mongo.clone()));
    match openwrt_manager.load_all().await {
        Ok(count) => tracing::info!("OpenWrtManager loaded {} routers", count),
        Err(e) => tracing::warn!("OpenWrtManager load failed (non-fatal): {}", e),
    }

    // Initialize ExternalDeviceManager (Mercury AC, Generic)
    let external_manager = Arc::new(ExternalDeviceManager::new(app_state.mongo.clone()));
    match external_manager.load_all().await {
        Ok(count) => tracing::info!("ExternalManager loaded {} devices", count),
        Err(e) => tracing::warn!("ExternalManager load failed (non-fatal): {}", e),
    }

    // Initialize AraneaClient for mobes2.0 Cloud Functions proxy
    let aranea_client = Arc::new(aranea::AraneaClient::new(config.aranea));
    if aranea_client.is_configured() {
        tracing::info!(
            "AraneaClient configured (tid: {})",
            aranea_client.config.tid
        );
    } else {
        tracing::info!("AraneaClient not configured (no aranea section in config)");
    }

    // Initialize proxy state (includes DdnsUpdater, optional GeoIP, auth config, managers)
    let proxy_state = ProxyState::new(
        app_state.clone(),
        notifier.clone(),
        config.server.geoip_db_path.as_deref(),
        config.auth,
        omada_manager.clone(),
        openwrt_manager.clone(),
        external_manager.clone(),
        aranea_client,
    )
    .await?;
    let route_count = proxy_state.router.read().await.len();
    tracing::info!(
        "Proxy router initialized with {} active routes",
        route_count
    );

    // Migrate to nodeOrder SSoT (one-time, if cg_node_order is empty)
    match node_order::migrate_to_node_order(&app_state.mongo).await {
        Ok(()) => tracing::debug!("NodeOrder migration check complete"),
        Err(e) => tracing::warn!("NodeOrder migration failed (non-fatal): {}", e),
    }

    // Repair Omada device parent relationships (AP→Switch heuristic)
    {
        let ingester = node_order::NodeOrderIngester::new(app_state.mongo.clone());
        match ingester.repair_omada_device_parents().await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("NodeOrder: repaired {} AP device parents", count);
                }
            }
            Err(e) => tracing::warn!("NodeOrder repair failed (non-fatal): {}", e),
        }
    }

    // Migrate cg_node_order → user_object_detail (one-time, if user_object_detail is empty)
    match user_object_ingester::migrate_to_user_object_detail(&app_state.mongo).await {
        Ok(()) => tracing::debug!("UserObjectDetail migration check complete"),
        Err(e) => tracing::warn!("UserObjectDetail migration failed (non-fatal): {}", e),
    }

    // Ensure device_state_history table exists
    match app_state.mysql.ensure_device_state_history_table().await {
        Ok(()) => tracing::debug!("device_state_history table ready"),
        Err(e) => tracing::warn!("device_state_history table creation failed (non-fatal): {}", e),
    }

    // Refresh araneaDevice cache (non-blocking, non-fatal)
    if proxy_state.aranea_client.is_configured() {
        match proxy_state.aranea_client.refresh_device_cache().await {
            Ok(count) => tracing::info!("AraneaDevice cache loaded: {} devices", count),
            Err(e) => tracing::warn!("AraneaDevice cache refresh failed (non-fatal): {}", e),
        }
    }

    // Start background tasks (use the same DdnsUpdater from proxy_state)
    start_background_tasks(
        app_state.clone(),
        notifier.clone(),
        proxy_state.ddns_updater.clone(),
        omada_manager,
        openwrt_manager,
        external_manager,
    );

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

/// Start background tasks (DDNS updater, health checker, restart scheduler, syncers)
fn start_background_tasks(
    app_state: AppState,
    notifier: Arc<DiscordNotifier>,
    ddns_updater: Arc<DdnsUpdater>,
    omada_manager: Arc<OmadaManager>,
    openwrt_manager: Arc<OpenWrtManager>,
    external_manager: Arc<ExternalDeviceManager>,
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

    // Omada syncer (60s interval, all controllers)
    let omada_syncer = Arc::new(OmadaSyncer::new(omada_manager, app_state.mongo.clone(), app_state.mysql.clone()));
    tokio::spawn(async move {
        omada_syncer.start().await;
    });

    // OpenWrt syncer (30s interval, all routers)
    let openwrt_syncer = Arc::new(OpenWrtSyncer::new(openwrt_manager, app_state.mongo.clone(), app_state.mysql.clone()));
    tokio::spawn(async move {
        openwrt_syncer.start().await;
    });

    // External device syncer (60s interval, Mercury AC etc.)
    let external_syncer = Arc::new(ExternalSyncer::new(
        external_manager,
        app_state.mongo.clone(),
        app_state.mysql.clone(),
    ));
    tokio::spawn(async move {
        external_syncer.start().await;
    });

    tracing::info!("Background tasks started");
}

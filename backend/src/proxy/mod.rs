//! Proxy module - Reverse proxy functionality

mod handler;
mod router;
pub(crate) mod ws_handler;

pub use self::handler::proxy_handler;
pub use self::router::ProxyRouter;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::AuthConfig;
use crate::db::AppState;
use crate::ddns::DdnsUpdater;
use crate::geoip::GeoIpReader;
use crate::notify::DiscordNotifier;

/// Shared proxy router state
#[derive(Clone)]
pub struct ProxyState {
    pub router: Arc<RwLock<ProxyRouter>>,
    pub app_state: AppState,
    pub http_client: reqwest::Client,
    pub ddns_updater: Arc<DdnsUpdater>,
    pub notifier: Arc<DiscordNotifier>,
    pub geoip: Option<Arc<GeoIpReader>>,
    pub auth_config: AuthConfig,
}

impl ProxyState {
    pub async fn new(
        app_state: AppState,
        notifier: Arc<DiscordNotifier>,
        geoip_db_path: Option<&str>,
        auth_config: AuthConfig,
    ) -> anyhow::Result<Self> {
        // Load initial routes from database (with DDNS hostname info)
        let routes = app_state.mysql.list_active_routes_with_ddns().await?;
        let router = ProxyRouter::new(routes);

        // Create HTTP client with sensible defaults
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .build()?;

        // Create DDNS updater
        let ddns_updater = Arc::new(DdnsUpdater::new(app_state.clone(), notifier.clone()));

        // Initialize GeoIP reader (optional, non-fatal on failure)
        let geoip = geoip_db_path.and_then(|path| {
            match GeoIpReader::open(path) {
                Ok(reader) => Some(Arc::new(reader)),
                Err(e) => {
                    tracing::warn!("GeoIP database not available: {} (path: {})", e, path);
                    None
                }
            }
        });

        Ok(Self {
            router: Arc::new(RwLock::new(router)),
            app_state,
            http_client,
            ddns_updater,
            notifier,
            geoip,
            auth_config,
        })
    }

    /// Reload routes from database
    pub async fn reload_routes(&self) -> anyhow::Result<()> {
        let routes = self.app_state.mysql.list_active_routes_with_ddns().await?;
        let count = routes.len();
        let mut router = self.router.write().await;
        *router = ProxyRouter::new(routes);
        tracing::info!("Proxy routes reloaded: {} active routes", count);
        Ok(())
    }
}

//! OpenWrtSyncer: Periodic SSH polling for all registered routers
//!
//! Runs in a background tokio task. Every 30 seconds, polls all routers
//! via SSH and upserts status/clients to MongoDB.

use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::db::mongo::MongoDb;
use crate::openwrt::manager::OpenWrtManager;

/// Background synchronization service for OpenWrt/AsusWrt routers
pub struct OpenWrtSyncer {
    manager: Arc<OpenWrtManager>,
    mongo: Arc<MongoDb>,
}

impl OpenWrtSyncer {
    pub fn new(manager: Arc<OpenWrtManager>, mongo: Arc<MongoDb>) -> Self {
        Self { manager, mongo }
    }

    /// Start the background sync loop (runs forever)
    pub async fn start(self: Arc<Self>) {
        tracing::info!("[OpenWrtSync] Starting background sync (interval: 30s)");

        // Initial sync after 10 seconds
        time::sleep(Duration::from_secs(10)).await;

        loop {
            self.sync_all_routers().await;
            time::sleep(Duration::from_secs(30)).await;
        }
    }

    /// Sync all registered routers
    async fn sync_all_routers(&self) {
        let router_ids = self.manager.list_router_ids().await;

        if router_ids.is_empty() {
            return;
        }

        tracing::debug!("[OpenWrtSync] Syncing {} routers", router_ids.len());

        for id in router_ids {
            if let Err(e) = self.poll_router(&id).await {
                tracing::warn!("[OpenWrtSync] Router {} sync failed: {}", id, e);
                let _ = self
                    .mongo
                    .update_openwrt_router_status(
                        &id, "error", None, None, None, None, None, 0, None, Some(&e),
                    )
                    .await;
            }
        }
    }

    /// Poll a single router: fetch status + clients via SSH
    async fn poll_router(&self, router_id: &str) -> Result<(), String> {
        let client = self
            .manager
            .get_client(router_id)
            .await
            .ok_or_else(|| format!("Router {} not found in manager", router_id))?;

        // 1. Get status
        let status = client.get_status().await?;

        // 2. Get clients
        let clients = client.get_clients().await?;

        // 3. Update router status in MongoDB
        self.mongo
            .update_openwrt_router_status(
                router_id,
                "online",
                status.wan_ip.as_deref(),
                status.lan_ip.as_deref(),
                status.ssid_24g.as_deref(),
                status.ssid_5g.as_deref(),
                status.uptime_seconds,
                clients.len() as u32,
                status.firmware_version.as_deref(),
                None,
            )
            .await?;

        // 4. Upsert clients (marks old ones inactive)
        self.mongo
            .upsert_openwrt_clients(router_id, &clients)
            .await?;

        tracing::debug!(
            "[OpenWrtSync] Router {} synced: {} clients",
            router_id,
            clients.len()
        );

        Ok(())
    }

    /// Manual poll trigger for a specific router
    pub async fn poll_one(&self, router_id: &str) -> Result<(), String> {
        self.poll_router(router_id).await
    }
}

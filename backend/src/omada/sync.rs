//! OmadaSyncer: Periodic data synchronization for all controllers
//!
//! Runs in a background tokio task. Every 60 seconds, iterates all registered
//! controllers, fetches sites/devices/clients/wireguard data, and upserts to MongoDB.

use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::db::mongo::MongoDb;
use crate::omada::manager::OmadaManager;

/// Background synchronization service
pub struct OmadaSyncer {
    manager: Arc<OmadaManager>,
    mongo: Arc<MongoDb>,
}

impl OmadaSyncer {
    pub fn new(manager: Arc<OmadaManager>, mongo: Arc<MongoDb>) -> Self {
        Self { manager, mongo }
    }

    /// Start the background sync loop (runs forever)
    pub async fn start(self: Arc<Self>) {
        tracing::info!("[OmadaSync] Starting background sync (interval: 60s)");

        // Initial sync after 5 seconds (give controllers time to initialize)
        time::sleep(Duration::from_secs(5)).await;

        loop {
            self.sync_all_controllers().await;
            time::sleep(Duration::from_secs(60)).await;
        }
    }

    /// Sync all registered controllers
    async fn sync_all_controllers(&self) {
        let controller_ids = self.manager.list_controller_ids().await;

        if controller_ids.is_empty() {
            return;
        }

        tracing::debug!(
            "[OmadaSync] Syncing {} controllers",
            controller_ids.len()
        );

        for id in controller_ids {
            if let Err(e) = self.sync_controller(&id).await {
                tracing::warn!("[OmadaSync] Controller {} sync failed: {}", id, e);
                let _ = self
                    .mongo
                    .update_omada_controller_status(&id, "error", Some(&e))
                    .await;
            }
        }
    }

    /// Sync a single controller: fetch all data and upsert to MongoDB
    async fn sync_controller(&self, controller_id: &str) -> Result<(), String> {
        let client = self
            .manager
            .get_client(controller_id)
            .await
            .ok_or_else(|| format!("Controller {} not found in manager", controller_id))?;

        // 1. Controller info → update version in omada_controllers
        let info = client.get_controller_info().await?;

        // 2. Get sites
        let sites = client.get_sites().await?;

        // Update controller doc: version info + site mappings (preserve fid/tid)
        if let Ok(Some(mut ctrl_doc)) = self.mongo.get_omada_controller(controller_id).await {
            let mut updated_sites = Vec::new();
            for site in &sites {
                let existing = ctrl_doc
                    .sites
                    .iter()
                    .find(|s| s.site_id == site.site_id);

                updated_sites.push(crate::db::mongo::omada::OmadaSiteMapping {
                    site_id: site.site_id.clone(),
                    name: site.name.clone(),
                    region: site.region.clone(),
                    fid: existing.and_then(|e| e.fid.clone()),
                    tid: existing.and_then(|e| e.tid.clone()),
                    fid_display_name: existing.and_then(|e| e.fid_display_name.clone()),
                });
            }
            ctrl_doc.sites = updated_sites;
            ctrl_doc.controller_ver = info.controller_ver;
            ctrl_doc.api_ver = info.api_ver;
            ctrl_doc.updated_at = chrono::Utc::now().to_rfc3339();
            let _ = self.mongo.upsert_omada_controller(&ctrl_doc).await;
        }

        // 3. For each site, fetch devices, clients, WG peers
        let mut total_devices = 0usize;
        let mut total_clients = 0usize;
        let mut total_wg_peers = 0usize;

        for site in &sites {
            // Devices
            match client.get_devices_for_site(&site.site_id).await {
                Ok(devices) => {
                    total_devices += devices.len();
                    self.mongo
                        .upsert_omada_devices(controller_id, &site.site_id, &devices)
                        .await?;
                }
                Err(e) => {
                    tracing::warn!(
                        "[OmadaSync] Devices fetch failed for site {}: {}",
                        site.site_id,
                        e
                    );
                }
            }

            // Clients
            match client.get_clients_for_site(&site.site_id).await {
                Ok(clients) => {
                    total_clients += clients.len();
                    self.mongo
                        .upsert_omada_clients(controller_id, &site.site_id, &clients)
                        .await?;
                }
                Err(e) => {
                    tracing::warn!(
                        "[OmadaSync] Clients fetch failed for site {}: {}",
                        site.site_id,
                        e
                    );
                }
            }

            // WireGuard peers
            match client.get_wireguard_peers(&site.site_id).await {
                Ok(peers) => {
                    total_wg_peers += peers.len();
                    self.mongo
                        .upsert_omada_wg_peers(controller_id, &site.site_id, &peers)
                        .await?;
                }
                Err(e) => {
                    tracing::warn!(
                        "[OmadaSync] WG peers fetch failed for site {}: {}",
                        site.site_id,
                        e
                    );
                }
            }
        }

        // 4. Update status to connected
        self.mongo
            .update_omada_controller_status(controller_id, "connected", None)
            .await?;

        tracing::debug!(
            "[OmadaSync] Controller {} synced: {} devices, {} clients, {} wg_peers",
            controller_id,
            total_devices,
            total_clients,
            total_wg_peers
        );

        Ok(())
    }

    /// Manual sync trigger for a specific controller
    pub async fn sync_one(&self, controller_id: &str) -> Result<(), String> {
        self.sync_controller(controller_id).await
    }
}

//! ExternalSyncer: Periodic polling for all registered external devices
//!
//! Runs in a background tokio task. Every 60 seconds, polls devices
//! that support auto-polling (Mercury AC) and upserts to MongoDB.

use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::db::mongo::MongoDb;
use crate::external::manager::{DeviceProtocol, ExternalDeviceManager};
use crate::external::mercury::MercuryClient;
use crate::node_order::NodeOrderIngester;

/// Background synchronization service for external devices
pub struct ExternalSyncer {
    manager: Arc<ExternalDeviceManager>,
    mongo: Arc<MongoDb>,
    ingester: NodeOrderIngester,
}

impl ExternalSyncer {
    pub fn new(manager: Arc<ExternalDeviceManager>, mongo: Arc<MongoDb>) -> Self {
        let ingester = NodeOrderIngester::new(mongo.clone());
        Self {
            manager,
            mongo,
            ingester,
        }
    }

    /// Start the background sync loop (runs forever)
    pub async fn start(self: Arc<Self>) {
        tracing::info!("[ExternalSync] Starting background sync (interval: 60s)");

        // Initial sync after 15 seconds
        time::sleep(Duration::from_secs(15)).await;

        loop {
            self.sync_all_devices().await;
            time::sleep(Duration::from_secs(60)).await;
        }
    }

    /// Sync all registered devices
    async fn sync_all_devices(&self) {
        let device_ids = self.manager.list_device_ids().await;

        if device_ids.is_empty() {
            return;
        }

        tracing::debug!("[ExternalSync] Syncing {} devices", device_ids.len());

        for id in device_ids {
            if let Err(e) = self.poll_device(&id).await {
                tracing::warn!("[ExternalSync] Device {} sync failed: {}", id, e);
                let _ = self
                    .mongo
                    .update_external_device_status(&id, "error", None, 0, Some(&e))
                    .await;
            }
        }
    }

    /// Poll a single device based on its protocol
    async fn poll_device(&self, device_id: &str) -> Result<(), String> {
        let protocol = self
            .manager
            .get_protocol(device_id)
            .await
            .ok_or_else(|| format!("Device {} not found in manager", device_id))?;

        match protocol {
            DeviceProtocol::MercuryAC => self.poll_mercury(device_id).await,
            DeviceProtocol::Generic | DeviceProtocol::Deco => {
                // Generic and Deco devices don't support auto-polling
                Ok(())
            }
        }
    }

    /// Poll a Mercury AC device
    async fn poll_mercury(&self, device_id: &str) -> Result<(), String> {
        let device = self
            .mongo
            .get_external_device(device_id)
            .await?
            .ok_or_else(|| format!("Device {} not found in MongoDB", device_id))?;

        let mut client = MercuryClient::new(
            device.ip.clone(),
            device.username.unwrap_or_else(|| "admin".to_string()),
            device.password.unwrap_or_default(),
        );

        // Login
        client.login().await?;

        // Get status
        let status = client.get_status().await?;

        // Get clients
        let clients = client.get_clients().await?;

        // Update device status
        self.mongo
            .update_external_device_status(
                device_id,
                "online",
                status.model.as_deref(),
                clients.len() as u32,
                None,
            )
            .await?;

        // Upsert clients
        self.mongo
            .upsert_external_clients(device_id, &clients)
            .await?;

        // Ingest into nodeOrder SSoT
        if let Err(e) = self.ingester.ingest_external(device_id).await {
            tracing::warn!(
                "[ExternalSync] NodeOrder ingestion failed for device {}: {}",
                device_id,
                e
            );
        }

        tracing::debug!(
            "[ExternalSync] Device {} synced: {} clients",
            device_id,
            clients.len()
        );

        Ok(())
    }

    /// Manual poll trigger for a specific device
    pub async fn poll_one(&self, device_id: &str) -> Result<(), String> {
        self.poll_device(device_id).await
    }
}

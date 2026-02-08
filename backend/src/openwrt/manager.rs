//! OpenWrtManager: Multi-router lifecycle management
//!
//! Manages multiple SshRouterClient instances (one per router).
//! Handles registration, connection testing, and loading from MongoDB.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::db::mongo::openwrt::OpenWrtRouterDoc;
use crate::db::mongo::MongoDb;
use crate::omada::client::normalize_mac;
use crate::openwrt::client::{RouterFirmware, SshRouterClient};

/// Manages multiple SSH router client instances
pub struct OpenWrtManager {
    routers: RwLock<HashMap<String, Arc<SshRouterClient>>>,
    mongo: Arc<MongoDb>,
}

impl OpenWrtManager {
    pub fn new(mongo: Arc<MongoDb>) -> Self {
        Self {
            routers: RwLock::new(HashMap::new()),
            mongo,
        }
    }

    /// Load all routers from MongoDB and create SshRouterClient instances
    pub async fn load_all(&self) -> Result<usize, String> {
        let routers = self.mongo.list_openwrt_routers().await?;
        let mut map = self.routers.write().await;

        for router in &routers {
            let client = Arc::new(SshRouterClient::new(
                router.ip.clone(),
                router.port,
                router.username.clone(),
                router.password.clone(),
                RouterFirmware::from_str(&router.firmware),
            ));
            map.insert(router.router_id.clone(), client);
        }

        let count = map.len();
        tracing::info!("[OpenWrtManager] Loaded {} routers", count);
        Ok(count)
    }

    /// Register a new router
    pub async fn register_router(
        &self,
        display_name: &str,
        mac: &str,
        ip: &str,
        port: u16,
        username: &str,
        password: &str,
        firmware: &str,
    ) -> Result<OpenWrtRouterDoc, String> {
        let router_id = normalize_mac(mac);
        let now = Utc::now().to_rfc3339();

        let doc = OpenWrtRouterDoc {
            router_id: router_id.clone(),
            display_name: display_name.to_string(),
            mac: router_id.clone(),
            ip: ip.to_string(),
            port,
            username: username.to_string(),
            password: password.to_string(),
            firmware: firmware.to_string(),
            status: "offline".to_string(),
            wan_ip: None,
            lan_ip: None,
            ssid_24g: None,
            ssid_5g: None,
            uptime_seconds: None,
            client_count: 0,
            firmware_version: None,
            last_error: None,
            omada_controller_id: None,
            omada_site_id: None,
            lacis_id: None,
            product_type: "101".to_string(), // Router
            network_device_type: "Router".to_string(),
            last_polled_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.mongo.upsert_openwrt_router(&doc).await?;

        let client = Arc::new(SshRouterClient::new(
            ip.to_string(),
            port,
            username.to_string(),
            password.to_string(),
            RouterFirmware::from_str(firmware),
        ));

        {
            let mut map = self.routers.write().await;
            map.insert(router_id.clone(), client);
        }

        tracing::info!(
            "[OpenWrtManager] Registered router: {} ({})",
            display_name,
            router_id
        );

        Ok(doc)
    }

    /// Test SSH connection without registering
    pub async fn test_connection(
        ip: &str,
        port: u16,
        username: &str,
        password: &str,
        firmware: &str,
    ) -> Result<serde_json::Value, String> {
        let client = SshRouterClient::new(
            ip.to_string(),
            port,
            username.to_string(),
            password.to_string(),
            RouterFirmware::from_str(firmware),
        );

        client.test_connection().await?;
        let status = client.get_status().await?;

        Ok(serde_json::json!({
            "success": true,
            "status": status,
        }))
    }

    /// Remove a router (from map and MongoDB)
    pub async fn remove_router(&self, router_id: &str) -> Result<(), String> {
        {
            let mut map = self.routers.write().await;
            map.remove(router_id);
        }

        self.mongo.delete_openwrt_router(router_id).await?;

        tracing::info!("[OpenWrtManager] Removed router: {}", router_id);
        Ok(())
    }

    /// Get a client instance for a specific router
    pub fn get_client_blocking(&self, _router_id: &str) -> Option<Arc<SshRouterClient>> {
        // Use try_read for non-async access
        None
    }

    /// Get a client instance (async)
    pub async fn get_client(&self, router_id: &str) -> Option<Arc<SshRouterClient>> {
        let map = self.routers.read().await;
        map.get(router_id).cloned()
    }

    /// Get all router IDs
    pub async fn list_router_ids(&self) -> Vec<String> {
        let map = self.routers.read().await;
        map.keys().cloned().collect()
    }

    /// Get MongoDB reference
    pub fn mongo(&self) -> &Arc<MongoDb> {
        &self.mongo
    }
}

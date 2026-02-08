//! ExternalDeviceManager: Multi-device lifecycle management
//!
//! Manages external network devices (Mercury AC, Generic, future DECO).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::db::mongo::external::ExternalDeviceDoc;
use crate::db::mongo::MongoDb;
use crate::external::mercury::MercuryClient;
use crate::omada::client::normalize_mac;

/// Device protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceProtocol {
    MercuryAC,
    Deco,    // Future: RSA+AES
    Generic, // Manual only, no polling
}

impl DeviceProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceProtocol::MercuryAC => "mercury_ac",
            DeviceProtocol::Deco => "deco",
            DeviceProtocol::Generic => "generic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mercury_ac" => DeviceProtocol::MercuryAC,
            "deco" => DeviceProtocol::Deco,
            _ => DeviceProtocol::Generic,
        }
    }

    /// Map to mobes2.0 product_type code
    pub fn product_type(&self) -> &'static str {
        match self {
            DeviceProtocol::MercuryAC => "103", // AccessPoint
            DeviceProtocol::Deco => "103",       // AccessPoint
            DeviceProtocol::Generic => "191",    // Unknown
        }
    }

    /// Map to mobes2.0 network device type
    pub fn network_device_type(&self) -> &'static str {
        match self {
            DeviceProtocol::MercuryAC => "AccessPoint",
            DeviceProtocol::Deco => "AccessPoint",
            DeviceProtocol::Generic => "Unknown",
        }
    }
}

/// Manages external device instances
pub struct ExternalDeviceManager {
    devices: RwLock<HashMap<String, DeviceProtocol>>,
    mongo: Arc<MongoDb>,
}

impl ExternalDeviceManager {
    pub fn new(mongo: Arc<MongoDb>) -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
            mongo,
        }
    }

    /// Load all devices from MongoDB
    pub async fn load_all(&self) -> Result<usize, String> {
        let devices = self.mongo.list_external_devices().await?;
        let mut map = self.devices.write().await;

        for device in &devices {
            let protocol = DeviceProtocol::from_str(&device.protocol);
            map.insert(device.device_id.clone(), protocol);
        }

        let count = map.len();
        tracing::info!("[ExternalManager] Loaded {} devices", count);
        Ok(count)
    }

    /// Register a new device
    pub async fn register_device(
        &self,
        display_name: &str,
        mac: &str,
        ip: &str,
        protocol: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<ExternalDeviceDoc, String> {
        let device_id = normalize_mac(mac);
        let proto = DeviceProtocol::from_str(protocol);
        let now = Utc::now().to_rfc3339();

        let doc = ExternalDeviceDoc {
            device_id: device_id.clone(),
            display_name: display_name.to_string(),
            mac: device_id.clone(),
            ip: ip.to_string(),
            protocol: proto.as_str().to_string(),
            username: username.map(String::from),
            password: password.map(String::from),
            status: "offline".to_string(),
            device_model: None,
            client_count: 0,
            last_error: None,
            omada_controller_id: None,
            omada_site_id: None,
            lacis_id: None,
            product_type: proto.product_type().to_string(),
            network_device_type: proto.network_device_type().to_string(),
            last_polled_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.mongo.upsert_external_device(&doc).await?;

        {
            let mut map = self.devices.write().await;
            map.insert(device_id.clone(), proto);
        }

        tracing::info!(
            "[ExternalManager] Registered device: {} ({}, {})",
            display_name,
            device_id,
            protocol
        );

        Ok(doc)
    }

    /// Test connection without registering
    pub async fn test_connection(
        ip: &str,
        protocol: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let proto = DeviceProtocol::from_str(protocol);

        match proto {
            DeviceProtocol::MercuryAC => {
                let mut client = MercuryClient::new(
                    ip.to_string(),
                    username.unwrap_or("admin").to_string(),
                    password.unwrap_or("").to_string(),
                );
                client.login().await?;
                let status = client.get_status().await?;

                Ok(serde_json::json!({
                    "success": true,
                    "protocol": "mercury_ac",
                    "model": status.model,
                    "firmware": status.firmware,
                }))
            }
            DeviceProtocol::Deco => Err("DECO protocol not yet implemented".to_string()),
            DeviceProtocol::Generic => Ok(serde_json::json!({
                "success": true,
                "protocol": "generic",
                "message": "Generic device registered (no polling)",
            })),
        }
    }

    /// Remove a device (from map and MongoDB)
    pub async fn remove_device(&self, device_id: &str) -> Result<(), String> {
        {
            let mut map = self.devices.write().await;
            map.remove(device_id);
        }

        self.mongo.delete_external_device(device_id).await?;

        tracing::info!("[ExternalManager] Removed device: {}", device_id);
        Ok(())
    }

    /// Get device protocol
    pub async fn get_protocol(&self, device_id: &str) -> Option<DeviceProtocol> {
        let map = self.devices.read().await;
        map.get(device_id).cloned()
    }

    /// Get all device IDs
    pub async fn list_device_ids(&self) -> Vec<String> {
        let map = self.devices.read().await;
        map.keys().cloned().collect()
    }

    /// Get MongoDB reference
    pub fn mongo(&self) -> &Arc<MongoDb> {
        &self.mongo
    }
}

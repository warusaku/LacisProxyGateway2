//! AraneaClient - proxy to mobes2.0 Cloud Functions
//!
//! Proxies requests to:
//! - araneaDeviceGate: device registration
//! - deviceStateReport: device state querying

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::AraneaConfig;

/// Cached araneaDevice entry: MAC → prefix-3 LacisID
#[derive(Debug, Clone)]
pub struct AraneaDeviceCacheEntry {
    pub lacis_id: String, // prefix-3 LacisID (20 digits)
    pub mac: String,      // normalized 12-digit uppercase HEX
}

/// AraneaClient holds tenant config and proxies requests to Cloud Functions
#[derive(Clone)]
pub struct AraneaClient {
    http_client: reqwest::Client,
    pub config: AraneaConfig,
    /// MAC → araneaDevice LacisID cache (prefix-3). Updated on startup + every 60 min.
    device_cache: Arc<RwLock<HashMap<String, AraneaDeviceCacheEntry>>>,
}

#[derive(Debug, Serialize)]
struct DeviceGateRequest {
    tid: String,
    #[serde(rename = "lacisId")]
    lacis_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    cic: String,
    mac: String,
    #[serde(rename = "productType")]
    product_type: String,
    #[serde(rename = "productCode")]
    product_code: String,
    #[serde(rename = "deviceType")]
    device_type: String,
}

#[derive(Debug, Serialize)]
struct DeviceStateRequest {
    tid: String,
    #[serde(rename = "lacisId")]
    lacis_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    cic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "targetLacisId")]
    target_lacis_id: Option<String>,
    mode: String, // "query" or "list"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AraneaDeviceRegistration {
    pub mac: String,
    pub product_type: String,
    pub product_code: String,
    pub device_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AraneaDeviceState {
    #[serde(rename = "lacisId")]
    pub lacis_id: Option<String>,
    pub state: Option<serde_json::Value>,
    pub mqtt_connected: Option<bool>,
    pub last_seen: Option<String>,
}

impl AraneaClient {
    pub fn new(config: AraneaConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            config,
            device_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if aranea is configured (has required tenant info)
    pub fn is_configured(&self) -> bool {
        !self.config.tid.is_empty()
            && !self.config.tenant_lacis_id.is_empty()
            && !self.config.tenant_cic.is_empty()
    }

    /// Register a device via araneaDeviceGate Cloud Function
    pub async fn register_device(
        &self,
        reg: &AraneaDeviceRegistration,
    ) -> Result<serde_json::Value, String> {
        if !self.is_configured() {
            return Err(
                "Aranea not configured: missing tid/tenant_lacis_id/tenant_cic".to_string(),
            );
        }

        let payload = DeviceGateRequest {
            tid: self.config.tid.clone(),
            lacis_id: self.config.tenant_lacis_id.clone(),
            user_id: self.config.tenant_user_id.clone(),
            cic: self.config.tenant_cic.clone(),
            mac: reg.mac.clone(),
            product_type: reg.product_type.clone(),
            product_code: reg.product_code.clone(),
            device_type: reg.device_type.clone(),
        };

        let resp = self
            .http_client
            .post(&self.config.device_gate_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("araneaDeviceGate request failed: {}", e))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("araneaDeviceGate response parse failed: {}", e))?;

        if status.is_success() {
            Ok(body)
        } else {
            Err(format!(
                "araneaDeviceGate returned {}: {}",
                status,
                serde_json::to_string(&body).unwrap_or_default()
            ))
        }
    }

    /// Query device states via deviceStateReport Cloud Function
    pub async fn get_device_states(
        &self,
        target_lacis_id: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if !self.is_configured() {
            return Err("Aranea not configured".to_string());
        }

        let mode = if target_lacis_id.is_some() {
            "query"
        } else {
            "list"
        };

        let payload = DeviceStateRequest {
            tid: self.config.tid.clone(),
            lacis_id: self.config.tenant_lacis_id.clone(),
            user_id: self.config.tenant_user_id.clone(),
            cic: self.config.tenant_cic.clone(),
            target_lacis_id: target_lacis_id.map(|s| s.to_string()),
            mode: mode.to_string(),
        };

        let resp = self
            .http_client
            .post(&self.config.device_state_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("deviceStateReport request failed: {}", e))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("deviceStateReport response parse failed: {}", e))?;

        if status.is_success() {
            Ok(body)
        } else {
            Err(format!(
                "deviceStateReport returned {}: {}",
                status,
                serde_json::to_string(&body).unwrap_or_default()
            ))
        }
    }

    /// Refresh the MAC → araneaDevice cache by fetching all device states.
    /// Called on startup and every 60 minutes.
    pub async fn refresh_device_cache(&self) -> Result<usize, String> {
        if !self.is_configured() {
            return Ok(0);
        }

        let response = self.get_device_states(None).await?;

        let mut new_cache = HashMap::new();

        // Parse response: expected format is { "devices": [ { "lacisId": "3...", "mac": "..." }, ... ] }
        // or similar array structure
        if let Some(devices) = response.get("devices").and_then(|v| v.as_array()) {
            for dev in devices {
                let lacis_id = dev
                    .get("lacisId")
                    .or_else(|| dev.get("lacis_id"))
                    .and_then(|v| v.as_str());
                let mac = dev.get("mac").and_then(|v| v.as_str());

                if let (Some(lid), Some(m)) = (lacis_id, mac) {
                    // Only cache prefix-3 devices (araneaDevice)
                    if lid.starts_with('3') && lid.len() == 20 {
                        let normalized_mac = m
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric())
                            .collect::<String>()
                            .to_uppercase();
                        if normalized_mac.len() == 12 {
                            new_cache.insert(
                                normalized_mac.clone(),
                                AraneaDeviceCacheEntry {
                                    lacis_id: lid.to_string(),
                                    mac: normalized_mac,
                                },
                            );
                        }
                    }
                }
            }
        }

        let count = new_cache.len();
        let mut cache = self.device_cache.write().await;
        *cache = new_cache;

        tracing::info!(
            "[AraneaClient] Device cache refreshed: {} araneaDevices",
            count
        );
        Ok(count)
    }

    /// Look up whether a MAC address corresponds to a registered araneaDevice.
    /// Returns the prefix-3 LacisID if found.
    pub async fn lookup_aranea_device(&self, mac: &str) -> Option<String> {
        let normalized = mac
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        let cache = self.device_cache.read().await;
        cache.get(&normalized).map(|entry| entry.lacis_id.clone())
    }

    /// Get aranea config summary (for frontend display)
    pub fn get_config_summary(&self) -> serde_json::Value {
        serde_json::json!({
            "configured": self.is_configured(),
            "tid": if self.config.tid.is_empty() { None } else { Some(&self.config.tid) },
            "tenant_user_id": if self.config.tenant_user_id.is_empty() { None } else { Some(&self.config.tenant_user_id) },
            "device_gate_url": &self.config.device_gate_url,
            "device_state_url": &self.config.device_state_url,
        })
    }
}

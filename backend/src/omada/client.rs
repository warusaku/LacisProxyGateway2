//! Omada OpenAPI client
//!
//! Supports two construction modes:
//! - `with_config()`: External config injection (used by OmadaManager)
//! - `new()` + `load_config()`: MySQL-based config (legacy, for migration)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc as StdArc;
use tokio::sync::RwLock;

use crate::db::MySqlDb;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmadaConfig {
    pub client_id: String,
    pub client_secret: String,
    pub omadac_id: String,
    pub site_id: String,
    pub base_url: String,
}

#[derive(Debug, Clone)]
struct TokenInfo {
    access_token: String,
    expires_at: std::time::Instant,
}

// ============================================================================
// MAC address normalization (mobes2.0 compatible)
// ============================================================================

/// Normalize MAC address to uppercase 12-char hex without separators.
/// Compatible with mobes2.0 normalizeMac() function.
pub fn normalize_mac(mac: &str) -> String {
    mac.replace([':', '-', '.'], "").to_uppercase()
}

// ============================================================================
// API response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub(crate) struct OmadaResponse<T> {
    #[serde(rename = "errorCode")]
    pub error_code: i32,
    pub msg: Option<String>,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct TokenResult {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListResult<T> {
    pub data: Option<Vec<T>>,
}

// ============================================================================
// Controller Info (GET /api/info - no auth required)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ControllerInfo {
    #[serde(rename = "controllerVer")]
    pub controller_ver: String,
    #[serde(rename = "apiVer")]
    pub api_ver: String,
    pub configured: bool,
    #[serde(rename = "omadacId")]
    pub omadac_id: String,
}

// ============================================================================
// Site
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaSite {
    #[serde(rename = "siteId")]
    pub site_id: String,
    pub name: String,
    pub region: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
    pub scenario: Option<String>,
}

// ============================================================================
// Device (infrastructure: gateway, switch, AP)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaDevice {
    pub mac: String,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub model: Option<String>,
    pub ip: Option<String>,
    pub status: i32, // 0=offline, 1=online
    #[serde(rename = "firmwareVersion")]
    pub firmware_version: Option<String>,
}

// ============================================================================
// Client device (connected endpoint)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OmadaClientDevice {
    pub mac: String,
    pub name: Option<String>,
    #[serde(rename = "hostName")]
    pub host_name: Option<String>,
    pub ip: Option<String>,
    #[serde(rename = "ipv6List")]
    pub ipv6_list: Option<Vec<String>>,
    pub vendor: Option<String>,
    #[serde(rename = "deviceType")]
    pub device_type: Option<String>,
    #[serde(rename = "deviceCategory")]
    pub device_category: Option<String>,
    #[serde(rename = "osName")]
    pub os_name: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "connectType")]
    pub connect_type: Option<i32>,
    pub wireless: Option<bool>,
    pub ssid: Option<String>,
    #[serde(rename = "signalLevel")]
    pub signal_level: Option<i32>,
    pub rssi: Option<i32>,
    #[serde(rename = "apMac")]
    pub ap_mac: Option<String>,
    #[serde(rename = "apName")]
    pub ap_name: Option<String>,
    #[serde(rename = "wifiMode")]
    pub wifi_mode: Option<i32>,
    pub channel: Option<i32>,
    #[serde(rename = "switchMac")]
    pub switch_mac: Option<String>,
    #[serde(rename = "switchName")]
    pub switch_name: Option<String>,
    pub port: Option<i32>,
    pub vid: Option<i32>,
    #[serde(rename = "trafficDown")]
    pub traffic_down: Option<i64>,
    #[serde(rename = "trafficUp")]
    pub traffic_up: Option<i64>,
    pub uptime: Option<i64>,
    #[serde(rename = "lastSeen")]
    pub last_seen: Option<i64>,
    pub active: Option<bool>,
    pub blocked: Option<bool>,
    pub guest: Option<bool>,
}

// ============================================================================
// WireGuard Peer
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WireGuardPeer {
    pub id: String,
    pub name: String,
    pub status: bool,
    #[serde(rename = "interfaceId")]
    pub interface_id: String,
    #[serde(rename = "interfaceName")]
    pub interface_name: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "allowAddress")]
    pub allow_address: Vec<String>,
    #[serde(rename = "keepAlive")]
    pub keep_alive: Option<i32>,
    pub comment: Option<String>,
}

// ============================================================================
// Legacy types (backward compatible)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct PortForwardingRule {
    pub name: Option<String>,
    #[serde(rename = "wanPortId")]
    pub wan_port_id: Option<String>,
    #[serde(rename = "externalPort")]
    pub external_port: Option<String>,
    #[serde(rename = "internalPort")]
    pub internal_port: Option<String>,
    #[serde(rename = "internalIp")]
    pub internal_ip: Option<String>,
    pub protocol: Option<String>,
    pub enable: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub gateway_online: bool,
    pub gateway_ip: Option<String>,
    pub wan_ip: Option<String>,
    pub devices: Vec<OmadaDevice>,
    pub port_forwarding: Vec<PortForwardingRule>,
    pub configured: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GatewayInfo {
    #[serde(rename = "publicIp")]
    public_ip: Option<String>,
}

// ============================================================================
// mobes2.0 compatible type mapping
// ============================================================================

/// Map Omada device_type to mobes2.0 product_type code
pub fn device_type_to_product_type(device_type: &str) -> &'static str {
    match device_type.to_lowercase().as_str() {
        "gateway" => "101", // Router
        "switch" => "102",  // Switch
        "ap" => "103",      // AccessPoint
        _ => "191",         // Unknown
    }
}

/// Map Omada device_type to mobes2.0 NetworkDeviceType string
pub fn device_type_to_network_device_type(device_type: &str) -> &'static str {
    match device_type.to_lowercase().as_str() {
        "gateway" => "Router",
        "switch" => "Switch",
        "ap" => "AccessPoint",
        _ => "Unknown",
    }
}

// ============================================================================
// OmadaClient
// ============================================================================

pub struct OmadaClient {
    config: RwLock<Option<OmadaConfig>>,
    token: RwLock<Option<TokenInfo>>,
    http_client: Client,
    /// MySQL DB reference (only for legacy load_config, None for with_config clients)
    db: Option<StdArc<MySqlDb>>,
}

impl OmadaClient {
    /// Create with external config injection (OmadaManager uses this)
    pub fn with_config(config: OmadaConfig) -> Self {
        let http_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: RwLock::new(Some(config)),
            token: RwLock::new(None),
            http_client,
            db: None,
        }
    }

    /// Create with MySQL DB reference (legacy constructor for migration)
    pub fn new(db: StdArc<MySqlDb>) -> Self {
        let http_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: RwLock::new(None),
            token: RwLock::new(None),
            http_client,
            db: Some(db),
        }
    }

    /// Get the current config (read-only snapshot)
    pub async fn get_config(&self) -> Option<OmadaConfig> {
        self.config.read().await.clone()
    }

    // ========================================================================
    // Token management
    // ========================================================================

    pub async fn ensure_token(&self) -> Result<String, String> {
        // Check if we have a valid token
        {
            let token = self.token.read().await;
            if let Some(ref t) = *token {
                if t.expires_at > std::time::Instant::now() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/authorize/token?grant_type=client_credentials",
            cfg.base_url
        );

        let body = serde_json::json!({
            "omadacId": cfg.omadac_id,
            "client_id": cfg.client_id,
            "client_secret": cfg.client_secret
        });

        let resp = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Token request failed: {}", e))?;

        let result: OmadaResponse<TokenResult> = resp
            .json()
            .await
            .map_err(|e| format!("Token parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Token error: {:?}", result.msg));
        }

        let token_result = result.result.ok_or("No token in response")?;
        let expires_in = token_result.expires_in.unwrap_or(7200);

        let token_info = TokenInfo {
            access_token: token_result.access_token.clone(),
            expires_at: std::time::Instant::now()
                + std::time::Duration::from_secs((expires_in - 60) as u64),
        };

        {
            let mut token = self.token.write().await;
            *token = Some(token_info);
        }

        tracing::info!("[Omada] Token acquired, expires in {} sec", expires_in);
        Ok(token_result.access_token)
    }

    // ========================================================================
    // Controller Info (GET /api/info - no auth required)
    // ========================================================================

    /// Get controller info (no auth required).
    /// Used to auto-discover omadac_id during registration.
    pub async fn get_controller_info_from_url(
        http_client: &Client,
        base_url: &str,
    ) -> Result<ControllerInfo, String> {
        let url = format!("{}/api/info", base_url);
        let resp = http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Controller info request failed: {}", e))?;

        let result: OmadaResponse<ControllerInfo> = resp
            .json()
            .await
            .map_err(|e| format!("Controller info parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Controller info error: {:?}", result.msg));
        }

        result
            .result
            .ok_or("No controller info in response".to_string())
    }

    /// Get controller info for this client's configured controller
    pub async fn get_controller_info(&self) -> Result<ControllerInfo, String> {
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;
        Self::get_controller_info_from_url(&self.http_client, &cfg.base_url).await
    }

    // ========================================================================
    // Sites
    // ========================================================================

    /// Get all sites for this controller (paginated)
    pub async fn get_sites(&self) -> Result<Vec<OmadaSite>, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let mut all_sites = Vec::new();
        let mut page = 1;
        let page_size = 100;

        loop {
            let url = format!(
                "{}/openapi/v1/{}/sites?page={}&pageSize={}",
                cfg.base_url, cfg.omadac_id, page, page_size
            );

            let resp = self
                .http_client
                .get(&url)
                .header("Authorization", format!("AccessToken={}", token))
                .send()
                .await
                .map_err(|e| format!("Sites request failed: {}", e))?;

            let result: OmadaResponse<ListResult<OmadaSite>> = resp
                .json()
                .await
                .map_err(|e| format!("Sites parse failed: {}", e))?;

            if result.error_code != 0 {
                return Err(format!("Sites error: {:?}", result.msg));
            }

            let sites = result.result.and_then(|r| r.data).unwrap_or_default();
            let count = sites.len();
            all_sites.extend(sites);

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_sites)
    }

    // ========================================================================
    // Devices (per site)
    // ========================================================================

    /// Get devices for a specific site
    pub async fn get_devices_for_site(&self, site_id: &str) -> Result<Vec<OmadaDevice>, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/devices?page=1&pageSize=100",
            cfg.base_url, cfg.omadac_id, site_id
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await
            .map_err(|e| format!("Devices request failed: {}", e))?;

        let result: OmadaResponse<ListResult<OmadaDevice>> = resp
            .json()
            .await
            .map_err(|e| format!("Devices parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Devices error: {:?}", result.msg));
        }

        Ok(result.result.and_then(|r| r.data).unwrap_or_default())
    }

    // ========================================================================
    // Clients (per site, paginated)
    // ========================================================================

    /// Get connected clients for a specific site (paginated)
    pub async fn get_clients_for_site(
        &self,
        site_id: &str,
    ) -> Result<Vec<OmadaClientDevice>, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let mut all_clients = Vec::new();
        let mut page = 1;
        let page_size = 100;

        loop {
            let url = format!(
                "{}/openapi/v1/{}/sites/{}/clients?page={}&pageSize={}",
                cfg.base_url, cfg.omadac_id, site_id, page, page_size
            );

            let resp = self
                .http_client
                .get(&url)
                .header("Authorization", format!("AccessToken={}", token))
                .send()
                .await
                .map_err(|e| format!("Clients request failed: {}", e))?;

            let result: OmadaResponse<ListResult<OmadaClientDevice>> = resp
                .json()
                .await
                .map_err(|e| format!("Clients parse failed: {}", e))?;

            if result.error_code != 0 {
                return Err(format!("Clients error: {:?}", result.msg));
            }

            let clients = result.result.and_then(|r| r.data).unwrap_or_default();
            let count = clients.len();
            all_clients.extend(clients);

            if count < page_size {
                break;
            }
            page += 1;
        }

        Ok(all_clients)
    }

    // ========================================================================
    // WireGuard Peers (per site)
    // ========================================================================

    /// Get WireGuard peers for a specific site
    pub async fn get_wireguard_peers(&self, site_id: &str) -> Result<Vec<WireGuardPeer>, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/vpn/wireguard-peers?page=1&pageSize=100",
            cfg.base_url, cfg.omadac_id, site_id
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await
            .map_err(|e| format!("WireGuard peers request failed: {}", e))?;

        let result: OmadaResponse<ListResult<WireGuardPeer>> = resp
            .json()
            .await
            .map_err(|e| format!("WireGuard peers parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("WireGuard peers error: {:?}", result.msg));
        }

        Ok(result.result.and_then(|r| r.data).unwrap_or_default())
    }

    // ========================================================================
    // Legacy methods (backward compatible)
    // ========================================================================

    /// Load config from MySQL (legacy, for migration support)
    pub async fn load_config(&self) -> Result<bool, String> {
        let db = self.db.as_ref().ok_or("No MySQL DB reference")?;

        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT config_key, config_value FROM omada_config")
                .fetch_all(db.pool())
                .await
                .map_err(|e| e.to_string())?;

        let mut client_id = String::new();
        let mut client_secret = String::new();
        let mut omadac_id = String::new();
        let mut site_id = String::new();
        let mut base_url = "https://192.168.3.50".to_string();

        for (key, value) in rows {
            let val = value.unwrap_or_default();
            match key.as_str() {
                "client_id" => client_id = val,
                "client_secret" => client_secret = val,
                "omadac_id" => omadac_id = val,
                "site_id" => site_id = val,
                "base_url" => {
                    if !val.is_empty() {
                        base_url = val
                    }
                }
                _ => {}
            }
        }

        let configured =
            !client_id.is_empty() && !client_secret.is_empty() && !omadac_id.is_empty();

        if configured {
            let mut config = self.config.write().await;
            *config = Some(OmadaConfig {
                client_id,
                client_secret,
                omadac_id,
                site_id,
                base_url,
            });
        }

        Ok(configured)
    }

    /// Get the site_id (auto-discover if empty) - legacy helper
    async fn get_site_id(&self) -> Result<String, String> {
        {
            let config = self.config.read().await;
            if let Some(ref cfg) = *config {
                if !cfg.site_id.is_empty() {
                    return Ok(cfg.site_id.clone());
                }
            }
        }

        // Auto-discover: get first site
        let sites = self.get_sites().await?;
        let site = sites.first().ok_or("No sites found")?;
        let site_id = site.site_id.clone();

        // Update config in memory
        {
            let mut config = self.config.write().await;
            if let Some(ref mut cfg) = *config {
                cfg.site_id = site_id.clone();
            }
        }

        tracing::info!("[Omada] Auto-discovered site: {} ({})", site.name, site_id);
        Ok(site_id)
    }

    /// Get devices (legacy: uses auto-discovered site_id)
    pub async fn get_devices(&self) -> Result<Vec<OmadaDevice>, String> {
        let site_id = self.get_site_id().await?;
        self.get_devices_for_site(&site_id).await
    }

    /// Get gateway WAN status (legacy)
    pub async fn get_gateway_wan_status(&self) -> Result<Option<String>, String> {
        let token = self.ensure_token().await?;
        let site_id = self.get_site_id().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let devices = self.get_devices().await?;
        let gateway = devices.iter().find(|d| d.device_type == "gateway");

        let gateway_mac = match gateway {
            Some(g) => &g.mac,
            None => return Ok(None),
        };

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/gateways/{}/wan",
            cfg.base_url, cfg.omadac_id, site_id, gateway_mac
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let result: Result<OmadaResponse<Vec<GatewayInfo>>, _> = r.json().await;
                if let Ok(data) = result {
                    if data.error_code == 0 {
                        if let Some(wans) = data.result {
                            if let Some(wan) = wans.first() {
                                return Ok(wan.public_ip.clone());
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    /// Get port forwarding rules (legacy)
    pub async fn get_port_forwarding(&self) -> Result<Vec<PortForwardingRule>, String> {
        let token = self.ensure_token().await?;
        let site_id = self.get_site_id().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let devices = self.get_devices().await?;
        let gateway = devices.iter().find(|d| d.device_type == "gateway");

        let gateway_mac = match gateway {
            Some(g) => &g.mac,
            None => return Ok(vec![]),
        };

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/gateways/{}/port-forwards",
            cfg.base_url, cfg.omadac_id, site_id, gateway_mac
        );

        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let result: Result<OmadaResponse<ListResult<PortForwardingRule>>, _> =
                    r.json().await;
                if let Ok(data) = result {
                    if data.error_code == 0 {
                        return Ok(data.result.and_then(|r| r.data).unwrap_or_default());
                    }
                }
                Ok(vec![])
            }
            Err(e) => {
                tracing::warn!("[Omada] Port forwarding fetch failed: {}", e);
                Ok(vec![])
            }
        }
    }

    /// Get full network status (legacy, for backward compatible API)
    pub async fn get_network_status(&self) -> NetworkStatus {
        // Load config if not loaded (MySQL path)
        if self.config.read().await.is_none() {
            if self.db.is_some() {
                let _ = self.load_config().await;
            }
        }

        let configured = self.config.read().await.is_some();

        if !configured {
            return NetworkStatus {
                gateway_online: false,
                gateway_ip: None,
                wan_ip: None,
                devices: vec![],
                port_forwarding: vec![],
                configured: false,
                error: Some("Omada not configured".to_string()),
            };
        }

        let devices = match self.get_devices().await {
            Ok(d) => d,
            Err(e) => {
                return NetworkStatus {
                    gateway_online: false,
                    gateway_ip: None,
                    wan_ip: None,
                    devices: vec![],
                    port_forwarding: vec![],
                    configured: true,
                    error: Some(e),
                };
            }
        };

        let gateway = devices.iter().find(|d| d.device_type == "gateway");
        let gateway_online = gateway.map(|g| g.status == 1).unwrap_or(false);
        let gateway_ip = gateway.and_then(|g| g.ip.clone());

        let wan_ip = self.get_gateway_wan_status().await.ok().flatten();
        let port_forwarding = self.get_port_forwarding().await.unwrap_or_default();

        NetworkStatus {
            gateway_online,
            gateway_ip,
            wan_ip,
            devices,
            port_forwarding,
            configured: true,
            error: None,
        }
    }

    /// Create a temporary client for connection testing (no DB, no state)
    pub fn create_test_client(base_url: &str, client_id: &str, client_secret: &str) -> Self {
        let http_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        // We create with a minimal config; omadac_id will be filled after get_controller_info
        Self {
            config: RwLock::new(Some(OmadaConfig {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
                omadac_id: String::new(),
                site_id: String::new(),
                base_url: base_url.to_string(),
            })),
            token: RwLock::new(None),
            http_client,
            db: None,
        }
    }

    /// Update the omadac_id in config (used after get_controller_info)
    pub async fn set_omadac_id(&self, omadac_id: &str) {
        let mut config = self.config.write().await;
        if let Some(ref mut cfg) = *config {
            cfg.omadac_id = omadac_id.to_string();
        }
    }

    /// Get the internal HTTP client (for static methods)
    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    // ========================================================================
    // WireGuard Peer CRUD (Omada OpenAPI)
    // ========================================================================

    /// Create a WireGuard peer via Omada OpenAPI
    pub async fn create_wireguard_peer(
        &self,
        site_id: &str,
        req: &CreateWgPeerRequest,
    ) -> Result<serde_json::Value, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/vpn/wireguard-peers",
            cfg.base_url, cfg.omadac_id, site_id
        );

        let body = serde_json::to_value(req).map_err(|e| format!("Serialize: {}", e))?;

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Create WG peer request failed: {}", e))?;

        let result: OmadaResponse<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Create WG peer parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Create WG peer error: {:?}", result.msg));
        }

        Ok(result.result.unwrap_or(serde_json::json!({})))
    }

    /// Update a WireGuard peer via Omada OpenAPI
    pub async fn update_wireguard_peer(
        &self,
        site_id: &str,
        peer_id: &str,
        req: &UpdateWgPeerRequest,
    ) -> Result<(), String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/vpn/wireguard-peers/{}",
            cfg.base_url, cfg.omadac_id, site_id, peer_id
        );

        let body = serde_json::to_value(req).map_err(|e| format!("Serialize: {}", e))?;

        let resp = self
            .http_client
            .put(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Update WG peer request failed: {}", e))?;

        let result: OmadaResponse<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Update WG peer parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Update WG peer error: {:?}", result.msg));
        }

        Ok(())
    }

    /// Delete a WireGuard peer via Omada OpenAPI
    pub async fn delete_wireguard_peer(&self, site_id: &str, peer_id: &str) -> Result<(), String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/vpn/wireguard-peers/{}",
            cfg.base_url, cfg.omadac_id, site_id, peer_id
        );

        let resp = self
            .http_client
            .delete(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await
            .map_err(|e| format!("Delete WG peer request failed: {}", e))?;

        let result: OmadaResponse<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| format!("Delete WG peer parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Delete WG peer error: {:?}", result.msg));
        }

        Ok(())
    }
}

// ============================================================================
// WireGuard Peer CRUD request types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWgPeerRequest {
    pub name: String,
    #[serde(rename = "interfaceId")]
    pub interface_id: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "allowAddress")]
    pub allow_address: Vec<String>,
    #[serde(rename = "keepAlive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWgPeerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "allowAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_address: Option<Vec<String>>,
    #[serde(rename = "keepAlive")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

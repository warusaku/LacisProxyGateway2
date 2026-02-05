//! Omada OpenAPI client

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use std::sync::Arc as StdArc;
use crate::db::MySqlDb;

#[derive(Debug, Clone)]
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

pub struct OmadaClient {
    config: RwLock<Option<OmadaConfig>>,
    token: RwLock<Option<TokenInfo>>,
    http_client: Client,
    db: StdArc<MySqlDb>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct OmadaClient_ {
    pub mac: String,
    pub name: Option<String>,
    #[serde(rename = "hostName")]
    pub host_name: Option<String>,
    pub ip: String,
    pub vendor: Option<String>,
    #[serde(rename = "connectType")]
    pub connect_type: Option<i32>, // 1=wireless, 2=wired
    pub active: Option<bool>,
}

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
struct OmadaResponse<T> {
    #[serde(rename = "errorCode")]
    error_code: i32,
    msg: Option<String>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct TokenResult {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ListResult<T> {
    data: Option<Vec<T>>,
}

#[derive(Debug, Deserialize)]
struct SiteInfo {
    #[serde(rename = "siteId")]
    site_id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct GatewayInfo {
    #[serde(rename = "publicIp")]
    public_ip: Option<String>,
}

impl OmadaClient {
    pub fn new(db: StdArc<MySqlDb>) -> Self {
        let http_client = Client::builder()
            .danger_accept_invalid_certs(true) // Local certs
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: RwLock::new(None),
            token: RwLock::new(None),
            http_client,
            db,
        }
    }

    pub async fn load_config(&self) -> Result<bool, String> {
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT config_key, config_value FROM omada_config"
        )
        .fetch_all(self.db.pool())
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
                "base_url" => if !val.is_empty() { base_url = val },
                _ => {}
            }
        }

        let configured = !client_id.is_empty() && !client_secret.is_empty() && !omadac_id.is_empty();

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

    async fn ensure_token(&self) -> Result<String, String> {
        // Check if we have a valid token
        {
            let token = self.token.read().await;
            if let Some(ref t) = *token {
                if t.expires_at > std::time::Instant::now() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Need to get a new token
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

        let resp = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Token request failed: {}", e))?;

        let result: OmadaResponse<TokenResult> = resp.json().await
            .map_err(|e| format!("Token parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Token error: {:?}", result.msg));
        }

        let token_result = result.result.ok_or("No token in response")?;
        let expires_in = token_result.expires_in.unwrap_or(7200);

        let token_info = TokenInfo {
            access_token: token_result.access_token.clone(),
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs((expires_in - 60) as u64),
        };

        {
            let mut token = self.token.write().await;
            *token = Some(token_info);
        }

        tracing::info!("[Omada] Token acquired, expires in {} sec", expires_in);
        Ok(token_result.access_token)
    }

    async fn auto_discover_site(&self) -> Result<String, String> {
        let token = self.ensure_token().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites?page=1&pageSize=100",
            cfg.base_url, cfg.omadac_id
        );

        let resp = self.http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await
            .map_err(|e| format!("Sites request failed: {}", e))?;

        let result: OmadaResponse<ListResult<SiteInfo>> = resp.json().await
            .map_err(|e| format!("Sites parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Sites error: {:?}", result.msg));
        }

        let sites = result.result
            .and_then(|r| r.data)
            .ok_or("No sites found")?;

        if let Some(site) = sites.first() {
            // Save site_id to database
            let _ = sqlx::query(
                "INSERT INTO omada_config (config_key, config_value) VALUES ('site_id', ?)
                 ON DUPLICATE KEY UPDATE config_value = ?"
            )
            .bind(&site.site_id)
            .bind(&site.site_id)
            .execute(self.db.pool())
            .await;

            tracing::info!("[Omada] Auto-discovered site: {} ({})", site.name, site.site_id);
            return Ok(site.site_id.clone());
        }

        Err("No sites found".to_string())
    }

    async fn get_site_id(&self) -> Result<String, String> {
        {
            let config = self.config.read().await;
            if let Some(ref cfg) = *config {
                if !cfg.site_id.is_empty() {
                    return Ok(cfg.site_id.clone());
                }
            }
        }

        // Auto-discover site
        let site_id = self.auto_discover_site().await?;

        // Update config in memory
        {
            let mut config = self.config.write().await;
            if let Some(ref mut cfg) = *config {
                cfg.site_id = site_id.clone();
            }
        }

        Ok(site_id)
    }

    pub async fn get_devices(&self) -> Result<Vec<OmadaDevice>, String> {
        let token = self.ensure_token().await?;
        let site_id = self.get_site_id().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/devices?page=1&pageSize=100",
            cfg.base_url, cfg.omadac_id, site_id
        );

        let resp = self.http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await
            .map_err(|e| format!("Devices request failed: {}", e))?;

        let result: OmadaResponse<ListResult<OmadaDevice>> = resp.json().await
            .map_err(|e| format!("Devices parse failed: {}", e))?;

        if result.error_code != 0 {
            return Err(format!("Devices error: {:?}", result.msg));
        }

        Ok(result.result.and_then(|r| r.data).unwrap_or_default())
    }

    pub async fn get_gateway_wan_status(&self) -> Result<Option<String>, String> {
        let token = self.ensure_token().await?;
        let site_id = self.get_site_id().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        // Get gateway device first
        let devices = self.get_devices().await?;
        let gateway = devices.iter().find(|d| d.device_type == "gateway");

        if gateway.is_none() {
            return Ok(None);
        }

        let gateway_mac = &gateway.unwrap().mac;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/gateways/{}/wan",
            cfg.base_url, cfg.omadac_id, site_id, gateway_mac
        );

        let resp = self.http_client
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
            Err(_) => Ok(None)
        }
    }

    pub async fn get_port_forwarding(&self) -> Result<Vec<PortForwardingRule>, String> {
        let token = self.ensure_token().await?;
        let site_id = self.get_site_id().await?;
        let config = self.config.read().await;
        let cfg = config.as_ref().ok_or("Omada not configured")?;

        // Get gateway MAC
        let devices = self.get_devices().await?;
        let gateway = devices.iter().find(|d| d.device_type == "gateway");

        if gateway.is_none() {
            return Ok(vec![]);
        }

        let gateway_mac = &gateway.unwrap().mac;

        let url = format!(
            "{}/openapi/v1/{}/sites/{}/gateways/{}/port-forwards",
            cfg.base_url, cfg.omadac_id, site_id, gateway_mac
        );

        let resp = self.http_client
            .get(&url)
            .header("Authorization", format!("AccessToken={}", token))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let result: Result<OmadaResponse<ListResult<PortForwardingRule>>, _> = r.json().await;
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

    pub async fn get_network_status(&self) -> NetworkStatus {
        // Load config if not loaded
        if self.config.read().await.is_none() {
            let _ = self.load_config().await;
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

        // Get devices
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

        // Get WAN IP
        let wan_ip = self.get_gateway_wan_status().await.ok().flatten();

        // Get port forwarding
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
}

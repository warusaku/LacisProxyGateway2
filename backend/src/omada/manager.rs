//! OmadaManager: Multi-controller lifecycle management
//!
//! Manages multiple OmadaClient instances (one per controller),
//! handles registration, connection testing, and MySQL→MongoDB migration.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use reqwest::Client;
use tokio::sync::RwLock;

use crate::db::mongo::omada::{OmadaControllerDoc, OmadaSiteMapping};
use crate::db::mongo::MongoDb;
use crate::db::MySqlDb;
use crate::omada::client::{OmadaClient, OmadaConfig};

/// Result of a connection test
#[derive(Debug, serde::Serialize)]
pub struct TestResult {
    pub success: bool,
    pub controller_ver: Option<String>,
    pub api_ver: Option<String>,
    pub omadac_id: Option<String>,
    pub sites: Vec<TestSiteInfo>,
    pub device_count: u64,
    pub error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct TestSiteInfo {
    pub site_id: String,
    pub name: String,
}

/// Manages multiple OmadaClient instances
pub struct OmadaManager {
    /// controller_id → OmadaClient instance
    clients: RwLock<HashMap<String, Arc<OmadaClient>>>,
    mongo: Arc<MongoDb>,
}

impl OmadaManager {
    pub fn new(mongo: Arc<MongoDb>) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            mongo,
        }
    }

    /// Load all controllers from MongoDB and create OmadaClient instances
    pub async fn load_all(&self) -> Result<usize, String> {
        let controllers = self.mongo.list_omada_controllers().await?;
        let mut map = self.clients.write().await;

        for ctrl in &controllers {
            let config = OmadaConfig {
                client_id: ctrl.client_id.clone(),
                client_secret: ctrl.client_secret.clone(),
                omadac_id: ctrl.omadac_id.clone(),
                // Use first site_id if available
                site_id: ctrl.sites.first().map(|s| s.site_id.clone()).unwrap_or_default(),
                base_url: ctrl.base_url.clone(),
            };

            let client = Arc::new(OmadaClient::with_config(config));
            map.insert(ctrl.controller_id.clone(), client);
        }

        let count = map.len();
        tracing::info!("[OmadaManager] Loaded {} controllers", count);
        Ok(count)
    }

    /// Register a new controller
    /// 1. GET /api/info → omadac_id
    /// 2. Token test
    /// 3. Sites discovery
    /// 4. MongoDB upsert
    /// 5. Add OmadaClient to map
    pub async fn register_controller(
        &self,
        display_name: &str,
        base_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<OmadaControllerDoc, String> {
        // 1. Get controller info (no auth required)
        let http_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client: {}", e))?;

        let info =
            OmadaClient::get_controller_info_from_url(&http_client, base_url).await?;

        // 2. Create test client and verify token
        let test_client = OmadaClient::create_test_client(base_url, client_id, client_secret);
        test_client.set_omadac_id(&info.omadac_id).await;
        test_client.ensure_token().await?;

        // 3. Discover sites
        let sites = test_client.get_sites().await?;
        let site_mappings: Vec<OmadaSiteMapping> = sites
            .iter()
            .map(|s| OmadaSiteMapping {
                site_id: s.site_id.clone(),
                name: s.name.clone(),
                region: s.region.clone(),
                fid: None,
                tid: None,
                fid_display_name: None,
            })
            .collect();

        // 4. Build and save controller document
        let now = Utc::now().to_rfc3339();
        let controller_doc = OmadaControllerDoc {
            controller_id: info.omadac_id.clone(),
            display_name: display_name.to_string(),
            base_url: base_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            omadac_id: info.omadac_id.clone(),
            controller_ver: info.controller_ver,
            api_ver: info.api_ver,
            status: "connected".to_string(),
            last_error: None,
            sites: site_mappings,
            last_synced_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.mongo
            .upsert_omada_controller(&controller_doc)
            .await?;

        // 5. Add client to active map
        let config = OmadaConfig {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            omadac_id: info.omadac_id.clone(),
            site_id: sites.first().map(|s| s.site_id.clone()).unwrap_or_default(),
            base_url: base_url.to_string(),
        };
        let client = Arc::new(OmadaClient::with_config(config));
        {
            let mut map = self.clients.write().await;
            map.insert(info.omadac_id.clone(), client);
        }

        tracing::info!(
            "[OmadaManager] Registered controller: {} ({})",
            display_name,
            info.omadac_id
        );

        Ok(controller_doc)
    }

    /// Test connection without registering
    pub async fn test_connection(
        base_url: &str,
        client_id: &str,
        client_secret: &str,
    ) -> TestResult {
        // 1. Get controller info
        let http_client = match Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return TestResult {
                    success: false,
                    controller_ver: None,
                    api_ver: None,
                    omadac_id: None,
                    sites: vec![],
                    device_count: 0,
                    error: Some(format!("HTTP client error: {}", e)),
                };
            }
        };

        let info = match OmadaClient::get_controller_info_from_url(&http_client, base_url).await {
            Ok(i) => i,
            Err(e) => {
                return TestResult {
                    success: false,
                    controller_ver: None,
                    api_ver: None,
                    omadac_id: None,
                    sites: vec![],
                    device_count: 0,
                    error: Some(format!("Controller info: {}", e)),
                };
            }
        };

        // 2. Token test
        let test_client = OmadaClient::create_test_client(base_url, client_id, client_secret);
        test_client.set_omadac_id(&info.omadac_id).await;

        if let Err(e) = test_client.ensure_token().await {
            return TestResult {
                success: false,
                controller_ver: Some(info.controller_ver),
                api_ver: Some(info.api_ver),
                omadac_id: Some(info.omadac_id),
                sites: vec![],
                device_count: 0,
                error: Some(format!("Token: {}", e)),
            };
        }

        // 3. Sites
        let sites = match test_client.get_sites().await {
            Ok(s) => s,
            Err(e) => {
                return TestResult {
                    success: false,
                    controller_ver: Some(info.controller_ver),
                    api_ver: Some(info.api_ver),
                    omadac_id: Some(info.omadac_id),
                    sites: vec![],
                    device_count: 0,
                    error: Some(format!("Sites: {}", e)),
                };
            }
        };

        // 4. Device count (from first site)
        let mut device_count = 0u64;
        for site in &sites {
            if let Ok(devices) = test_client.get_devices_for_site(&site.site_id).await {
                device_count += devices.len() as u64;
            }
        }

        let site_infos: Vec<TestSiteInfo> = sites
            .iter()
            .map(|s| TestSiteInfo {
                site_id: s.site_id.clone(),
                name: s.name.clone(),
            })
            .collect();

        TestResult {
            success: true,
            controller_ver: Some(info.controller_ver),
            api_ver: Some(info.api_ver),
            omadac_id: Some(info.omadac_id),
            sites: site_infos,
            device_count,
            error: None,
        }
    }

    /// Remove a controller (from map and MongoDB)
    pub async fn remove_controller(&self, controller_id: &str) -> Result<(), String> {
        {
            let mut map = self.clients.write().await;
            map.remove(controller_id);
        }

        self.mongo
            .delete_omada_controller(controller_id)
            .await?;

        tracing::info!("[OmadaManager] Removed controller: {}", controller_id);
        Ok(())
    }

    /// Get a client instance for a specific controller
    pub async fn get_client(&self, controller_id: &str) -> Option<Arc<OmadaClient>> {
        let map = self.clients.read().await;
        map.get(controller_id).cloned()
    }

    /// Get all controller IDs
    pub async fn list_controller_ids(&self) -> Vec<String> {
        let map = self.clients.read().await;
        map.keys().cloned().collect()
    }

    /// Get MongoDB reference (for syncer)
    pub fn mongo(&self) -> &Arc<MongoDb> {
        &self.mongo
    }

    /// Migrate from MySQL omada_config to MongoDB omada_controllers (one-time, startup)
    pub async fn migrate_from_mysql(&self, mysql: &MySqlDb) -> Result<bool, String> {
        // Check if MongoDB already has controllers
        let existing = self.mongo.list_omada_controllers().await?;
        if !existing.is_empty() {
            tracing::info!("[OmadaManager] MongoDB already has {} controllers, skipping MySQL migration", existing.len());
            return Ok(false);
        }

        // Read MySQL omada_config
        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT config_key, config_value FROM omada_config")
                .fetch_all(mysql.pool())
                .await
                .map_err(|e| format!("MySQL read omada_config: {}", e))?;

        if rows.is_empty() {
            tracing::info!("[OmadaManager] MySQL omada_config is empty, nothing to migrate");
            return Ok(false);
        }

        let mut client_id = String::new();
        let mut client_secret = String::new();
        let mut omadac_id = String::new();
        let mut site_id = String::new();
        let mut base_url = "https://192.168.3.50".to_string();

        for (key, value) in &rows {
            let val = value.as_deref().unwrap_or("");
            match key.as_str() {
                "client_id" => client_id = val.to_string(),
                "client_secret" => client_secret = val.to_string(),
                "omadac_id" => omadac_id = val.to_string(),
                "site_id" => site_id = val.to_string(),
                "base_url" => {
                    if !val.is_empty() {
                        base_url = val.to_string()
                    }
                }
                _ => {}
            }
        }

        if client_id.is_empty() || client_secret.is_empty() {
            tracing::info!("[OmadaManager] MySQL omada_config incomplete, skipping migration");
            return Ok(false);
        }

        // Try to get controller info and sites
        let display_name = "Migrated Controller".to_string();
        let now = Utc::now().to_rfc3339();

        // If omadac_id is known, build a minimal doc; otherwise try to discover
        if omadac_id.is_empty() {
            let http_client = Client::builder()
                .danger_accept_invalid_certs(true)
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| format!("HTTP client: {}", e))?;

            match OmadaClient::get_controller_info_from_url(&http_client, &base_url).await {
                Ok(info) => omadac_id = info.omadac_id,
                Err(e) => {
                    tracing::warn!("[OmadaManager] Migration: cannot reach controller at {}: {}", base_url, e);
                    return Ok(false);
                }
            }
        }

        let mut sites = Vec::new();
        if !site_id.is_empty() {
            sites.push(OmadaSiteMapping {
                site_id: site_id.clone(),
                name: "Migrated Site".to_string(),
                region: None,
                fid: None,
                tid: None,
                fid_display_name: None,
            });
        }

        let controller_doc = OmadaControllerDoc {
            controller_id: omadac_id.clone(),
            display_name,
            base_url: base_url.clone(),
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            omadac_id: omadac_id.clone(),
            controller_ver: String::new(),
            api_ver: String::new(),
            status: "disconnected".to_string(),
            last_error: None,
            sites,
            last_synced_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.mongo
            .upsert_omada_controller(&controller_doc)
            .await?;

        // Add to active clients
        let config = OmadaConfig {
            client_id,
            client_secret,
            omadac_id: omadac_id.clone(),
            site_id,
            base_url,
        };
        let client = Arc::new(OmadaClient::with_config(config));
        {
            let mut map = self.clients.write().await;
            map.insert(omadac_id.clone(), client);
        }

        tracing::info!("[OmadaManager] Migrated MySQL omada_config to MongoDB (controller_id: {})", omadac_id);
        Ok(true)
    }
}

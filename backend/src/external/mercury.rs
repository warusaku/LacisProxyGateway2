//! Mercury AC HTTP JSON API Client
//!
//! Based on is10m (aranea_ISMS) design documents.
//! Uses XOR password encoding for authentication.

use reqwest::Client;
use serde_json::Value;

// ============================================================================
// Types
// ============================================================================

/// Mercury AP status
#[derive(Debug, Clone, serde::Serialize)]
pub struct MercuryStatus {
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub client_count: u32,
    pub online: bool,
}

/// Mercury client info
#[derive(Debug, Clone, serde::Serialize)]
pub struct MercuryClientInfo {
    pub mac: String,
    pub ip: Option<String>,
    pub hostname: Option<String>,
}

// ============================================================================
// XOR password encoding (is10m orgAuthPwd() compatible)
// ============================================================================

/// XOR encode password for Mercury AC authentication
/// Based on is10m JavaScript implementation: org_enc_pwd()
fn xor_encode_password(password: &str) -> String {
    let key = b"RDpbLfCPsJZ7fiv";
    let mut encoded = String::new();
    for (i, byte) in password.bytes().enumerate() {
        let xored = byte ^ key[i % key.len()];
        encoded.push_str(&format!("{:02x}", xored));
    }
    encoded
}

// ============================================================================
// Mercury Client
// ============================================================================

/// HTTP client for Mercury AC access points
pub struct MercuryClient {
    pub ip: String,
    pub username: String,
    pub password: String,
    http_client: Client,
    stok: Option<String>,
}

impl MercuryClient {
    pub fn new(ip: String, username: String, password: String) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            ip,
            username,
            password,
            http_client,
            stok: None,
        }
    }

    /// Login to Mercury AC and get stok token
    pub async fn login(&mut self) -> Result<(), String> {
        let encoded_pwd = xor_encode_password(&self.password);

        let body = serde_json::json!({
            "method": "do",
            "login": {
                "password": encoded_pwd
            }
        });

        let url = format!("http://{}/", self.ip);
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Mercury login request failed: {}", e))?;

        let result: Value = resp
            .json()
            .await
            .map_err(|e| format!("Mercury login parse failed: {}", e))?;

        // Extract stok from response
        if let Some(stok) = result.get("stok").and_then(|v| v.as_str()) {
            self.stok = Some(stok.to_string());
            Ok(())
        } else if let Some(error_code) = result.get("error_code").and_then(|v| v.as_i64()) {
            Err(format!("Mercury login error code: {}", error_code))
        } else {
            Err("Mercury login: no stok in response".to_string())
        }
    }

    /// Get Mercury device status
    pub async fn get_status(&self) -> Result<MercuryStatus, String> {
        let stok = self.stok.as_ref().ok_or("Not logged in")?;

        let body = serde_json::json!({
            "method": "get",
            "system": { "sysinfo": {} }
        });

        let url = format!("http://{}/stok={}/ds", self.ip, stok);
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Mercury status request failed: {}", e))?;

        let result: Value = resp
            .json()
            .await
            .map_err(|e| format!("Mercury status parse failed: {}", e))?;

        let sysinfo = result.get("system").and_then(|s| s.get("sysinfo"));

        Ok(MercuryStatus {
            model: sysinfo
                .and_then(|s| s.get("model"))
                .and_then(|v| v.as_str())
                .map(String::from),
            firmware: sysinfo
                .and_then(|s| s.get("sw_version"))
                .and_then(|v| v.as_str())
                .map(String::from),
            client_count: 0, // Will be populated from get_clients
            online: true,
        })
    }

    /// Get connected clients
    pub async fn get_clients(&self) -> Result<Vec<MercuryClientInfo>, String> {
        let stok = self.stok.as_ref().ok_or("Not logged in")?;

        let body = serde_json::json!({
            "method": "get",
            "hosts_info": { "table": "host_info" }
        });

        let url = format!("http://{}/stok={}/ds", self.ip, stok);
        let resp = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Mercury clients request failed: {}", e))?;

        let result: Value = resp
            .json()
            .await
            .map_err(|e| format!("Mercury clients parse failed: {}", e))?;

        let mut clients = Vec::new();

        if let Some(hosts) = result.get("hosts_info").and_then(|h| h.get("host_info")) {
            if let Some(arr) = hosts.as_array() {
                for host in arr {
                    let mac = host
                        .get("mac")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if mac.is_empty() {
                        continue;
                    }

                    clients.push(MercuryClientInfo {
                        mac: crate::omada::client::normalize_mac(&mac),
                        ip: host.get("ip").and_then(|v| v.as_str()).map(String::from),
                        hostname: host
                            .get("hostname")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    });
                }
            } else if let Some(obj) = hosts.as_object() {
                // Some Mercury models return an object with index keys
                for (_key, host) in obj {
                    let mac = host
                        .get("mac")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if mac.is_empty() {
                        continue;
                    }

                    clients.push(MercuryClientInfo {
                        mac: crate::omada::client::normalize_mac(&mac),
                        ip: host.get("ip").and_then(|v| v.as_str()).map(String::from),
                        hostname: host
                            .get("hostname")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    });
                }
            }
        }

        Ok(clients)
    }
}

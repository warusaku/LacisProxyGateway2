//! No-IP provider implementation

use async_trait::async_trait;

use super::DdnsProviderTrait;
use crate::models::DdnsConfig;

pub struct NoIpProvider {
    client: reqwest::Client,
}

impl NoIpProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for NoIpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DdnsProviderTrait for NoIpProvider {
    async fn update(&self, config: &DdnsConfig, ip: &str) -> Result<(), String> {
        let username = config
            .username
            .as_ref()
            .ok_or("Username required for No-IP")?;
        let password = config
            .password
            .as_ref()
            .ok_or("Password required for No-IP")?;

        // No-IP update URL format
        // https://dynupdate.no-ip.com/nic/update?hostname=<hostname>&myip=<ip>
        let url = format!(
            "https://dynupdate.no-ip.com/nic/update?hostname={}&myip={}",
            config.hostname, ip
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(username, Some(password))
            .header("User-Agent", "LacisProxyGateway2/1.0 hideaki@example.com")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Parse No-IP response (same format as DynDNS)
        let response_code = body.split_whitespace().next().unwrap_or("");

        match response_code {
            "good" | "nochg" => {
                tracing::info!("No-IP update successful for {}: {}", config.hostname, body);
                Ok(())
            }
            "badauth" => Err("Bad authentication credentials".to_string()),
            "nohost" => Err("Hostname does not exist".to_string()),
            "badagent" => Err("Bad user agent - update client".to_string()),
            "abuse" => Err("Hostname has been blocked due to abuse".to_string()),
            "!donator" => Err("Feature not available for free accounts".to_string()),
            "911" => Err("No-IP server error".to_string()),
            _ => Err(format!("Unknown response: {}", body)),
        }
    }

    fn name(&self) -> &'static str {
        "No-IP"
    }
}

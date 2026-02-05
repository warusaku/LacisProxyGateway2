//! DynDNS provider implementation

use async_trait::async_trait;

use super::DdnsProviderTrait;
use crate::models::DdnsConfig;

pub struct DynDnsProvider {
    client: reqwest::Client,
}

impl DynDnsProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DynDnsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DdnsProviderTrait for DynDnsProvider {
    async fn update(&self, config: &DdnsConfig, ip: &str) -> Result<(), String> {
        let username = config
            .username
            .as_ref()
            .ok_or("Username required for DynDNS")?;
        let password = config
            .password
            .as_ref()
            .ok_or("Password required for DynDNS")?;

        // DynDNS update URL format
        // https://members.dyndns.org/nic/update?hostname=<hostname>&myip=<ip>
        let url = format!(
            "https://members.dyndns.org/nic/update?hostname={}&myip={}",
            config.hostname, ip
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(username, Some(password))
            .header("User-Agent", "LacisProxyGateway2/1.0")
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

        // Parse DynDNS response
        // good <ip> - Success
        // nochg <ip> - No change needed
        // badauth - Bad credentials
        // notfqdn - Hostname not a FQDN
        // nohost - Hostname doesn't exist
        // numhost - Too many hosts
        // abuse - Hostname blocked
        // dnserr - DNS error
        // 911 - Server error

        let response_code = body.split_whitespace().next().unwrap_or("");

        match response_code {
            "good" | "nochg" => {
                tracing::info!("DynDNS update successful for {}: {}", config.hostname, body);
                Ok(())
            }
            "badauth" => Err("Bad authentication credentials".to_string()),
            "notfqdn" => Err("Hostname is not a fully qualified domain name".to_string()),
            "nohost" => Err("Hostname does not exist in your account".to_string()),
            "numhost" => Err("Too many hosts or aliases".to_string()),
            "abuse" => Err("Hostname has been blocked due to abuse".to_string()),
            "dnserr" => Err("DNS error on server side".to_string()),
            "911" => Err("DynDNS server error".to_string()),
            _ => Err(format!("Unknown response: {}", body)),
        }
    }

    fn name(&self) -> &'static str {
        "DynDNS"
    }
}

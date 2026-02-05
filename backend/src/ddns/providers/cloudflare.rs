//! Cloudflare provider implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::DdnsProviderTrait;
use crate::models::DdnsConfig;

pub struct CloudflareProvider {
    client: reqwest::Client,
}

impl CloudflareProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for CloudflareProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct CloudflareDnsRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    errors: Vec<CloudflareError>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareDnsResult {
    id: String,
}

#[async_trait]
impl DdnsProviderTrait for CloudflareProvider {
    async fn update(&self, config: &DdnsConfig, ip: &str) -> Result<(), String> {
        let api_token = config
            .api_token
            .as_ref()
            .ok_or("API token required for Cloudflare")?;
        let zone_id = config
            .zone_id
            .as_ref()
            .ok_or("Zone ID required for Cloudflare")?;

        // First, get the existing DNS record ID
        let record_id = self
            .get_record_id(api_token, zone_id, &config.hostname)
            .await?;

        // Determine record type based on IP format
        let record_type = if ip.contains(':') { "AAAA" } else { "A" };

        let record = CloudflareDnsRecord {
            record_type: record_type.to_string(),
            name: config.hostname.clone(),
            content: ip.to_string(),
            ttl: 1, // Auto TTL
            proxied: false,
        };

        // Update the DNS record
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            zone_id, record_id
        );

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", api_token))
            .header("Content-Type", "application/json")
            .json(&record)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let cf_response: CloudflareResponse<CloudflareDnsResult> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if cf_response.success {
            tracing::info!(
                "Cloudflare update successful for {}: {}",
                config.hostname,
                ip
            );
            Ok(())
        } else {
            let errors: Vec<String> = cf_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            Err(format!("Cloudflare error: {}", errors.join(", ")))
        }
    }

    fn name(&self) -> &'static str {
        "Cloudflare"
    }
}

impl CloudflareProvider {
    async fn get_record_id(
        &self,
        api_token: &str,
        zone_id: &str,
        hostname: &str,
    ) -> Result<String, String> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}",
            zone_id, hostname
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_token))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Failed to get DNS records: {}", e))?;

        #[derive(Debug, Deserialize)]
        struct ListResult {
            id: String,
        }

        let cf_response: CloudflareResponse<Vec<ListResult>> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse DNS list response: {}", e))?;

        if !cf_response.success {
            let errors: Vec<String> = cf_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(format!("Cloudflare error: {}", errors.join(", ")));
        }

        cf_response
            .result
            .and_then(|records| records.into_iter().next())
            .map(|r| r.id)
            .ok_or_else(|| format!("DNS record not found for {}", hostname))
    }
}

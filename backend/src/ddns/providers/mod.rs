//! DDNS providers

mod cloudflare;
mod dyndns;
mod noip;

pub use self::cloudflare::CloudflareProvider;
pub use self::dyndns::DynDnsProvider;
pub use self::noip::NoIpProvider;

use async_trait::async_trait;

use crate::models::DdnsConfig;

/// DDNS provider trait
#[async_trait]
pub trait DdnsProviderTrait: Send + Sync {
    /// Update the DNS record with the current IP
    async fn update(&self, config: &DdnsConfig, ip: &str) -> Result<(), String>;

    /// Get the provider name
    fn name(&self) -> &'static str;
}

/// Get the current public IP address
pub async fn get_public_ip() -> Result<String, String> {
    let client = reqwest::Client::new();

    // Try multiple IP services
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
    ];

    for service in &services {
        match client
            .get(*service)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(ip) = response.text().await {
                        let ip = ip.trim().to_string();
                        // Basic IP validation
                        if is_valid_ip(&ip) {
                            return Ok(ip);
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err("Failed to get public IP from all services".to_string())
}

fn is_valid_ip(ip: &str) -> bool {
    // Basic IPv4 validation
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        return parts.iter().all(|p| p.parse::<u8>().is_ok());
    }

    // Basic IPv6 validation (contains colons)
    ip.contains(':')
}

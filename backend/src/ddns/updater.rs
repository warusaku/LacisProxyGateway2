//! DDNS update scheduler

use std::sync::Arc;
use std::time::Duration;

use tokio::time::interval;

use super::providers::{
    get_public_ip, CloudflareProvider, DdnsProviderTrait, DynDnsProvider, NoIpProvider,
};
use crate::db::AppState;
use crate::models::{DdnsProvider, DdnsStatus};
use crate::notify::DiscordNotifier;

/// DDNS updater that runs in the background
pub struct DdnsUpdater {
    app_state: AppState,
    dyndns: DynDnsProvider,
    noip: NoIpProvider,
    cloudflare: CloudflareProvider,
    notifier: Arc<DiscordNotifier>,
}

impl DdnsUpdater {
    pub fn new(app_state: AppState, notifier: Arc<DiscordNotifier>) -> Self {
        Self {
            app_state,
            dyndns: DynDnsProvider::new(),
            noip: NoIpProvider::new(),
            cloudflare: CloudflareProvider::new(),
            notifier,
        }
    }

    /// Start the DDNS update loop
    pub async fn start(self: Arc<Self>) {
        tracing::info!("Starting DDNS updater...");

        // Run every 60 seconds
        let mut interval_timer = interval(Duration::from_secs(60));

        loop {
            interval_timer.tick().await;

            if let Err(e) = self.update_all().await {
                tracing::error!("DDNS update cycle failed: {}", e);
            }
        }
    }

    /// Update all active DDNS configurations
    async fn update_all(&self) -> anyhow::Result<()> {
        let configs = self.app_state.mysql.list_active_ddns().await?;

        if configs.is_empty() {
            return Ok(());
        }

        // Get current public IP
        let current_ip = match get_public_ip().await {
            Ok(ip) => ip,
            Err(e) => {
                tracing::error!("Failed to get public IP: {}", e);
                return Ok(());
            }
        };

        for config in configs {
            // Check if IP has changed
            if config.last_ip.as_ref() == Some(&current_ip) {
                tracing::debug!("IP unchanged for {}, skipping", config.hostname);
                continue;
            }

            // Get appropriate provider
            let provider: &dyn DdnsProviderTrait = match config.provider {
                DdnsProvider::DynDns => &self.dyndns,
                DdnsProvider::NoIp => &self.noip,
                DdnsProvider::Cloudflare => &self.cloudflare,
            };

            tracing::info!(
                "Updating DDNS for {} via {}: {} -> {}",
                config.hostname,
                provider.name(),
                config.last_ip.as_deref().unwrap_or("unknown"),
                current_ip
            );

            match provider.update(&config, &current_ip).await {
                Ok(()) => {
                    // Update database with new IP
                    if let Err(e) = self
                        .app_state
                        .mysql
                        .update_ddns_ip(config.id, &current_ip, DdnsStatus::Active, None)
                        .await
                    {
                        tracing::error!("Failed to update DDNS status in DB: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("DDNS update failed for {}: {}", config.hostname, e);

                    // Update database with error
                    if let Err(db_err) = self.app_state.mysql.set_ddns_error(config.id, &e).await {
                        tracing::error!("Failed to update DDNS error in DB: {}", db_err);
                    }

                    // Log security event
                    if let Err(log_err) = self
                        .app_state
                        .mongo
                        .log_ddns_failure(&config.hostname, &config.provider.to_string(), &e)
                        .await
                    {
                        tracing::error!("Failed to log DDNS failure: {}", log_err);
                    }

                    // Send Discord notification
                    self.notifier
                        .notify_ddns_failure(&config.hostname, &config.provider.to_string(), &e)
                        .await;
                }
            }
        }

        Ok(())
    }

    /// Manually trigger update for a specific DDNS config
    pub async fn update_single(&self, config_id: i32) -> Result<(), String> {
        let config = self
            .app_state
            .mysql
            .get_ddns(config_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("DDNS config {} not found", config_id))?;

        let current_ip = get_public_ip().await?;

        let provider: &dyn DdnsProviderTrait = match config.provider {
            DdnsProvider::DynDns => &self.dyndns,
            DdnsProvider::NoIp => &self.noip,
            DdnsProvider::Cloudflare => &self.cloudflare,
        };

        provider.update(&config, &current_ip).await?;

        self.app_state
            .mysql
            .update_ddns_ip(config.id, &current_ip, DdnsStatus::Active, None)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

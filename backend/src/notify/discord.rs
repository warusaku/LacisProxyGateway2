//! Discord webhook notifications

use chrono::Utc;
use serde::Serialize;

use crate::db::AppState;
use crate::models::Severity;

/// Discord notifier
pub struct DiscordNotifier {
    client: reqwest::Client,
    app_state: AppState,
}

#[derive(Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    timestamp: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<DiscordField>,
}

#[derive(Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Serialize)]
struct DiscordWebhookPayload {
    embeds: Vec<DiscordEmbed>,
}

impl DiscordNotifier {
    pub fn new(app_state: AppState) -> Self {
        Self {
            client: reqwest::Client::new(),
            app_state,
        }
    }

    /// Get webhook URL from settings
    async fn get_webhook_url(&self) -> Option<String> {
        self.app_state
            .mysql
            .get_discord_webhook_url()
            .await
            .ok()
            .flatten()
    }

    /// Check if a notification type is enabled
    async fn is_notify_enabled(&self, notify_type: &str) -> bool {
        self.app_state
            .mysql
            .is_discord_notify_enabled(notify_type)
            .await
            .unwrap_or(false)
    }

    /// Send a Discord notification
    async fn send(&self, embed: DiscordEmbed) {
        let webhook_url = match self.get_webhook_url().await {
            Some(url) => url,
            None => {
                tracing::debug!("Discord webhook URL not configured");
                return;
            }
        };

        let payload = DiscordWebhookPayload {
            embeds: vec![embed],
        };

        match self
            .client
            .post(&webhook_url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    tracing::warn!("Discord webhook returned status: {}", response.status());
                }
            }
            Err(e) => {
                tracing::error!("Failed to send Discord notification: {}", e);
            }
        }
    }

    /// Convert severity to Discord embed color
    fn severity_to_color(severity: Severity) -> u32 {
        match severity {
            Severity::Low => 0x3498db,      // Blue
            Severity::Medium => 0xf39c12,   // Orange
            Severity::High => 0xe74c3c,     // Red
            Severity::Critical => 0x9b59b6, // Purple
        }
    }

    /// Notify IP blocked
    pub async fn notify_ip_blocked(&self, ip: &str, reason: &str, severity: Severity) {
        if !self.is_notify_enabled("security").await {
            return;
        }

        let embed = DiscordEmbed {
            title: "IP Blocked".to_string(),
            description: format!("An IP address has been blocked."),
            color: Self::severity_to_color(severity),
            timestamp: Utc::now().to_rfc3339(),
            fields: vec![
                DiscordField {
                    name: "IP Address".to_string(),
                    value: ip.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Reason".to_string(),
                    value: reason.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Severity".to_string(),
                    value: format!("{:?}", severity),
                    inline: true,
                },
            ],
        };

        self.send(embed).await;
    }

    /// Notify DDNS failure
    pub async fn notify_ddns_failure(&self, hostname: &str, provider: &str, error: &str) {
        if !self.is_notify_enabled("ddns").await {
            return;
        }

        let embed = DiscordEmbed {
            title: "DDNS Update Failed".to_string(),
            description: format!("Failed to update DDNS record for {}", hostname),
            color: Self::severity_to_color(Severity::High),
            timestamp: Utc::now().to_rfc3339(),
            fields: vec![
                DiscordField {
                    name: "Hostname".to_string(),
                    value: hostname.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Provider".to_string(),
                    value: provider.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Error".to_string(),
                    value: error.to_string(),
                    inline: false,
                },
            ],
        };

        self.send(embed).await;
    }

    /// Notify health check failure
    pub async fn notify_health_failure(&self, path: &str, target: &str, consecutive_failures: u32) {
        if !self.is_notify_enabled("health").await {
            return;
        }

        let severity = if consecutive_failures >= 5 {
            Severity::Critical
        } else if consecutive_failures >= 3 {
            Severity::High
        } else {
            Severity::Medium
        };

        let embed = DiscordEmbed {
            title: "Health Check Failed".to_string(),
            description: format!("Route {} is experiencing issues", path),
            color: Self::severity_to_color(severity),
            timestamp: Utc::now().to_rfc3339(),
            fields: vec![
                DiscordField {
                    name: "Path".to_string(),
                    value: path.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Target".to_string(),
                    value: target.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Consecutive Failures".to_string(),
                    value: consecutive_failures.to_string(),
                    inline: true,
                },
            ],
        };

        self.send(embed).await;
    }

    /// Notify health recovery
    pub async fn notify_health_recovery(&self, path: &str, target: &str) {
        if !self.is_notify_enabled("health").await {
            return;
        }

        let embed = DiscordEmbed {
            title: "Health Check Recovered".to_string(),
            description: format!("Route {} is now healthy", path),
            color: 0x2ecc71, // Green
            timestamp: Utc::now().to_rfc3339(),
            fields: vec![
                DiscordField {
                    name: "Path".to_string(),
                    value: path.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Target".to_string(),
                    value: target.to_string(),
                    inline: true,
                },
            ],
        };

        self.send(embed).await;
    }

    /// Notify rate limit exceeded
    pub async fn notify_rate_limit(&self, ip: &str, requests: i32) {
        if !self.is_notify_enabled("security").await {
            return;
        }

        let embed = DiscordEmbed {
            title: "Rate Limit Exceeded".to_string(),
            description: format!("IP {} exceeded rate limit", ip),
            color: Self::severity_to_color(Severity::Medium),
            timestamp: Utc::now().to_rfc3339(),
            fields: vec![
                DiscordField {
                    name: "IP Address".to_string(),
                    value: ip.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Requests".to_string(),
                    value: requests.to_string(),
                    inline: true,
                },
            ],
        };

        self.send(embed).await;
    }
}

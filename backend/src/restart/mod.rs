//! Restart scheduler module
//! Handles scheduled restarts and resource-based auto-restart

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::MySqlDb;

#[derive(Debug, Clone)]
pub struct RestartConfig {
    /// Scheduled restart enabled
    pub scheduled_enabled: bool,
    /// Scheduled restart time (HH:MM format, 24h)
    pub scheduled_time: String,
    /// Auto-restart on high resource enabled
    pub auto_restart_enabled: bool,
    /// CPU threshold percentage (default 90)
    pub cpu_threshold: u32,
    /// RAM threshold percentage (default 90)
    pub ram_threshold: u32,
    /// Last restart timestamp
    pub last_restart: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self {
            scheduled_enabled: false,
            scheduled_time: "04:00".to_string(),
            auto_restart_enabled: false,
            cpu_threshold: 90,
            ram_threshold: 90,
            last_restart: None,
        }
    }
}

pub struct RestartScheduler {
    config: RwLock<RestartConfig>,
    db: Arc<MySqlDb>,
    last_scheduled_check: RwLock<Option<chrono::NaiveDate>>,
}

impl RestartScheduler {
    pub fn new(db: Arc<MySqlDb>) -> Self {
        Self {
            config: RwLock::new(RestartConfig::default()),
            db,
            last_scheduled_check: RwLock::new(None),
        }
    }

    /// Load configuration from database
    pub async fn load_config(&self) -> Result<(), String> {
        let settings = self.db.list_settings().await.map_err(|e| e.to_string())?;

        let mut config = self.config.write().await;

        for setting in settings {
            match setting.setting_key.as_str() {
                "restart_scheduled_enabled" => {
                    config.scheduled_enabled = setting
                        .setting_value
                        .as_deref()
                        .map(|v| v == "true" || v == "1")
                        .unwrap_or(false);
                }
                "restart_scheduled_time" => {
                    if let Some(v) = setting.setting_value {
                        config.scheduled_time = v;
                    }
                }
                "restart_auto_enabled" => {
                    config.auto_restart_enabled = setting
                        .setting_value
                        .as_deref()
                        .map(|v| v == "true" || v == "1")
                        .unwrap_or(false);
                }
                "restart_cpu_threshold" => {
                    if let Some(v) = setting.setting_value {
                        config.cpu_threshold = v.parse().unwrap_or(90);
                    }
                }
                "restart_ram_threshold" => {
                    if let Some(v) = setting.setting_value {
                        config.ram_threshold = v.parse().unwrap_or(90);
                    }
                }
                _ => {}
            }
        }

        tracing::info!(
            "[RestartScheduler] Config loaded: scheduled={}, time={}, auto={}, cpu_thresh={}%, ram_thresh={}%",
            config.scheduled_enabled,
            config.scheduled_time,
            config.auto_restart_enabled,
            config.cpu_threshold,
            config.ram_threshold
        );

        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> RestartConfig {
        self.config.read().await.clone()
    }

    /// Get current CPU usage percentage
    fn get_cpu_usage() -> f64 {
        let read_stat = || -> (u64, u64) {
            let content = std::fs::read_to_string("/proc/stat").unwrap_or_default();
            if let Some(line) = content.lines().next() {
                let parts: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() >= 4 {
                    let idle = parts[3];
                    let total: u64 = parts.iter().sum();
                    return (idle, total);
                }
            }
            (0, 0)
        };

        let (idle1, total1) = read_stat();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let (idle2, total2) = read_stat();

        let idle_delta = idle2.saturating_sub(idle1);
        let total_delta = total2.saturating_sub(total1);

        if total_delta > 0 {
            ((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Get current RAM usage percentage
    fn get_ram_usage() -> f64 {
        let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mut mem_total = 0u64;
        let mut mem_available = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value: u64 = parts[1].parse().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => mem_total = value,
                    "MemAvailable:" => mem_available = value,
                    _ => {}
                }
            }
        }

        if mem_total > 0 {
            ((mem_total - mem_available) as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if scheduled restart time has been reached
    fn should_scheduled_restart(&self, config: &RestartConfig) -> bool {
        if !config.scheduled_enabled {
            return false;
        }

        let now = chrono::Local::now();
        let current_time = now.format("%H:%M").to_string();

        // Check if we're within the restart window (exact minute match)
        current_time == config.scheduled_time
    }

    /// Trigger system restart
    async fn trigger_restart(&self, reason: &str) {
        tracing::warn!("[RestartScheduler] Triggering system restart: {}", reason);

        // Send Discord notification if configured
        if let Ok(Some(webhook_url)) = self.db.get_discord_webhook_url().await {
            let client = reqwest::Client::new();
            let _ = client
                .post(&webhook_url)
                .json(&serde_json::json!({
                    "embeds": [{
                        "title": "System Restart Triggered",
                        "description": reason,
                        "color": 15158332,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "footer": {"text": "LacisProxyGateway2"}
                    }]
                }))
                .send()
                .await;
        }

        // Wait a moment for notification to send
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Execute restart command
        let output = std::process::Command::new("sudo")
            .args(["systemctl", "reboot"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                tracing::info!("[RestartScheduler] Restart command executed successfully");
            }
            Ok(o) => {
                tracing::error!(
                    "[RestartScheduler] Restart command failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                );
            }
            Err(e) => {
                tracing::error!("[RestartScheduler] Failed to execute restart: {}", e);
            }
        }
    }

    /// Main monitoring loop
    pub async fn start_monitoring(self: Arc<Self>) {
        tracing::info!("[RestartScheduler] Starting monitoring...");

        // Load initial config
        if let Err(e) = self.load_config().await {
            tracing::error!("[RestartScheduler] Failed to load config: {}", e);
        }

        let check_interval = tokio::time::Duration::from_secs(30);

        loop {
            tokio::time::sleep(check_interval).await;

            // Reload config periodically
            if let Err(e) = self.load_config().await {
                tracing::warn!("[RestartScheduler] Failed to reload config: {}", e);
            }

            let config = self.get_config().await;

            // Check scheduled restart
            if self.should_scheduled_restart(&config) {
                let today = chrono::Local::now().date_naive();
                let mut last_check = self.last_scheduled_check.write().await;

                // Only trigger once per day
                if last_check.map(|d| d != today).unwrap_or(true) {
                    *last_check = Some(today);
                    drop(last_check);
                    self.trigger_restart(&format!(
                        "Scheduled restart at {}",
                        config.scheduled_time
                    ))
                    .await;
                    continue;
                }
            }

            // Check resource-based restart
            if config.auto_restart_enabled {
                let cpu_usage = Self::get_cpu_usage();
                let ram_usage = Self::get_ram_usage();

                tracing::debug!(
                    "[RestartScheduler] CPU: {:.1}%, RAM: {:.1}%",
                    cpu_usage,
                    ram_usage
                );

                if cpu_usage >= config.cpu_threshold as f64 {
                    self.trigger_restart(&format!(
                        "CPU usage critical: {:.1}% (threshold: {}%)",
                        cpu_usage, config.cpu_threshold
                    ))
                    .await;
                    continue;
                }

                if ram_usage >= config.ram_threshold as f64 {
                    self.trigger_restart(&format!(
                        "RAM usage critical: {:.1}% (threshold: {}%)",
                        ram_usage, config.ram_threshold
                    ))
                    .await;
                    continue;
                }
            }
        }
    }
}

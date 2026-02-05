//! Health check scheduler

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::db::AppState;
use crate::models::HealthCheck;
use crate::notify::DiscordNotifier;

/// Track consecutive failures per route
type FailureTracker = HashMap<i32, u32>;

/// Health checker that runs in the background
pub struct HealthChecker {
    app_state: AppState,
    client: reqwest::Client,
    notifier: Arc<DiscordNotifier>,
    failures: Arc<RwLock<FailureTracker>>,
}

impl HealthChecker {
    pub fn new(app_state: AppState, notifier: Arc<DiscordNotifier>) -> Self {
        Self {
            app_state,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .connect_timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
            notifier,
            failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the health check loop
    pub async fn start(self: Arc<Self>) {
        tracing::info!("Starting health checker...");

        // Get check interval from settings (default 60 seconds)
        let check_interval = self
            .app_state
            .mysql
            .get_health_check_settings()
            .await
            .map(|(interval, _, _)| interval as u64)
            .unwrap_or(60);

        let mut interval_timer = interval(Duration::from_secs(check_interval));

        loop {
            interval_timer.tick().await;

            if let Err(e) = self.check_all().await {
                tracing::error!("Health check cycle failed: {}", e);
            }
        }
    }

    /// Check all active routes
    async fn check_all(&self) -> anyhow::Result<()> {
        let routes = self.app_state.mysql.list_active_routes().await?;

        if routes.is_empty() {
            return Ok(());
        }

        let (_, timeout_ms, failure_threshold) = self
            .app_state
            .mysql
            .get_health_check_settings()
            .await
            .unwrap_or((60, 5000, 3));

        for route in routes {
            let healthy = self.check_route(&route.target, timeout_ms as u64).await;

            // Record health check
            let check = HealthCheck {
                timestamp: Utc::now(),
                route_id: route.id,
                target: route.target.clone(),
                healthy: healthy.is_ok(),
                response_time_ms: healthy.as_ref().ok().copied(),
                status_code: healthy.as_ref().err().and_then(|e| e.parse::<i32>().ok()),
                error: healthy.as_ref().err().map(|s| s.to_string()),
            };

            if let Err(e) = self.app_state.mongo.save_health_check(&check).await {
                tracing::warn!("Failed to save health check: {}", e);
            }

            // Track failures
            let mut failures = self.failures.write().await;
            if healthy.is_err() {
                let count = failures.entry(route.id).or_insert(0);
                *count += 1;

                tracing::warn!(
                    "Health check failed for {} ({}): consecutive failures = {}",
                    route.path,
                    route.target,
                    *count
                );

                // Notify if threshold reached
                if *count == failure_threshold as u32 {
                    // Log security event
                    if let Err(e) = self
                        .app_state
                        .mongo
                        .log_health_check_failure(route.id, &route.target, *count)
                        .await
                    {
                        tracing::warn!("Failed to log health check failure: {}", e);
                    }

                    // Send Discord notification
                    self.notifier
                        .notify_health_failure(&route.path, &route.target, *count)
                        .await;
                }
            } else {
                // Check if we're recovering from failure
                if let Some(prev_count) = failures.get(&route.id) {
                    if *prev_count >= failure_threshold as u32 {
                        // Send recovery notification
                        self.notifier
                            .notify_health_recovery(&route.path, &route.target)
                            .await;

                        tracing::info!(
                            "Health check recovered for {} ({})",
                            route.path,
                            route.target
                        );
                    }
                }

                // Reset failure count
                failures.remove(&route.id);
            }
        }

        Ok(())
    }

    /// Check a single route's health
    async fn check_route(&self, target: &str, timeout_ms: u64) -> Result<i32, String> {
        let start = Instant::now();

        // Use HEAD request for efficiency
        let response = self
            .client
            .head(target)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "timeout".to_string()
                } else if e.is_connect() {
                    "connection_failed".to_string()
                } else {
                    e.to_string()
                }
            })?;

        let elapsed_ms = start.elapsed().as_millis() as i32;

        // Consider 2xx and 3xx as healthy
        if response.status().is_success() || response.status().is_redirection() {
            Ok(elapsed_ms)
        } else {
            Err(response.status().as_u16().to_string())
        }
    }

    /// Get current failure counts
    pub async fn get_failures(&self) -> HashMap<i32, u32> {
        self.failures.read().await.clone()
    }
}

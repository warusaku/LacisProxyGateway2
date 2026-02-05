//! Settings CRUD operations

use crate::error::AppError;
use crate::models::Setting;

use super::MySqlDb;

impl MySqlDb {
    /// Get all settings
    pub async fn list_settings(&self) -> Result<Vec<Setting>, AppError> {
        let settings = sqlx::query_as::<_, Setting>(
            r#"
            SELECT id, setting_key, setting_value, description, updated_at
            FROM settings
            ORDER BY setting_key ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(settings)
    }

    /// Get a single setting by key
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let setting = sqlx::query_as::<_, Setting>(
            r#"
            SELECT id, setting_key, setting_value, description, updated_at
            FROM settings
            WHERE setting_key = ?
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(setting.and_then(|s| s.setting_value))
    }

    /// Get a setting as bool
    pub async fn get_setting_bool(&self, key: &str) -> Result<bool, AppError> {
        let value = self.get_setting(key).await?;
        Ok(value.map(|v| v == "true" || v == "1").unwrap_or(false))
    }

    /// Get a setting as i32
    pub async fn get_setting_i32(&self, key: &str, default: i32) -> Result<i32, AppError> {
        let value = self.get_setting(key).await?;
        Ok(value.and_then(|v| v.parse().ok()).unwrap_or(default))
    }

    /// Update a setting
    pub async fn set_setting(&self, key: &str, value: Option<&str>) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE settings
            SET setting_value = ?
            WHERE setting_key = ?
            "#,
        )
        .bind(value)
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Create or update a setting
    pub async fn upsert_setting(
        &self,
        key: &str,
        value: Option<&str>,
        description: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO settings (setting_key, setting_value, description)
            VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE setting_value = VALUES(setting_value)
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(description)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get Discord webhook URL
    pub async fn get_discord_webhook_url(&self) -> Result<Option<String>, AppError> {
        self.get_setting("discord_webhook_url").await
    }

    /// Check if Discord notifications are enabled for a type
    pub async fn is_discord_notify_enabled(&self, notify_type: &str) -> Result<bool, AppError> {
        let key = format!("discord_notify_{}", notify_type);
        self.get_setting_bool(&key).await
    }

    /// Get rate limit settings
    pub async fn get_rate_limit_settings(&self) -> Result<(bool, i32), AppError> {
        let enabled = self.get_setting_bool("rate_limit_enabled").await?;
        let requests_per_minute = self
            .get_setting_i32("rate_limit_requests_per_minute", 60)
            .await?;
        Ok((enabled, requests_per_minute))
    }

    /// Get health check settings
    pub async fn get_health_check_settings(&self) -> Result<(i32, i32, i32), AppError> {
        let interval = self
            .get_setting_i32("health_check_interval_sec", 60)
            .await?;
        let timeout = self
            .get_setting_i32("health_check_timeout_ms", 5000)
            .await?;
        let threshold = self
            .get_setting_i32("health_check_failure_threshold", 3)
            .await?;
        Ok((interval, timeout, threshold))
    }
}

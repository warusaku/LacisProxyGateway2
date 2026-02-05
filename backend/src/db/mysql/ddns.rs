//! DDNS configuration CRUD operations

use chrono::Utc;
use sqlx::Row;

use crate::error::AppError;
use crate::models::{CreateDdnsRequest, DdnsConfig, DdnsConfigRow, DdnsStatus, UpdateDdnsRequest};

use super::MySqlDb;

impl MySqlDb {
    /// Get all DDNS configurations
    pub async fn list_ddns(&self) -> Result<Vec<DdnsConfig>, AppError> {
        let rows = sqlx::query_as::<_, DdnsConfigRow>(
            r#"
            SELECT id, provider, hostname, username, password, api_token, zone_id,
                   update_interval_sec, last_ip, last_update, last_error, status,
                   created_at, updated_at
            FROM ddns_configs
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let configs: Result<Vec<DdnsConfig>, _> = rows.into_iter().map(DdnsConfig::try_from).collect();
        configs.map_err(|e| AppError::InternalError(e))
    }

    /// Get active DDNS configurations
    pub async fn list_active_ddns(&self) -> Result<Vec<DdnsConfig>, AppError> {
        let rows = sqlx::query_as::<_, DdnsConfigRow>(
            r#"
            SELECT id, provider, hostname, username, password, api_token, zone_id,
                   update_interval_sec, last_ip, last_update, last_error, status,
                   created_at, updated_at
            FROM ddns_configs
            WHERE status = 'active'
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let configs: Result<Vec<DdnsConfig>, _> = rows.into_iter().map(DdnsConfig::try_from).collect();
        configs.map_err(|e| AppError::InternalError(e))
    }

    /// Get a single DDNS configuration by ID
    pub async fn get_ddns(&self, id: i32) -> Result<Option<DdnsConfig>, AppError> {
        let row = sqlx::query_as::<_, DdnsConfigRow>(
            r#"
            SELECT id, provider, hostname, username, password, api_token, zone_id,
                   update_interval_sec, last_ip, last_update, last_error, status,
                   created_at, updated_at
            FROM ddns_configs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(DdnsConfig::try_from(r).map_err(|e| AppError::InternalError(e))?)),
            None => Ok(None),
        }
    }

    /// Create a new DDNS configuration
    pub async fn create_ddns(&self, req: &CreateDdnsRequest) -> Result<i32, AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO ddns_configs (provider, hostname, username, password, api_token, zone_id, update_interval_sec)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(req.provider.to_string())
        .bind(&req.hostname)
        .bind(&req.username)
        .bind(&req.password)
        .bind(&req.api_token)
        .bind(&req.zone_id)
        .bind(req.update_interval_sec)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    /// Update an existing DDNS configuration
    pub async fn update_ddns(&self, id: i32, req: &UpdateDdnsRequest) -> Result<bool, AppError> {
        let existing = self
            .get_ddns(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("DDNS config {} not found", id)))?;

        let hostname = req.hostname.as_ref().unwrap_or(&existing.hostname);
        let username = req.username.as_ref().or(existing.username.as_ref());
        let password = req.password.as_ref().or(existing.password.as_ref());
        let api_token = req.api_token.as_ref().or(existing.api_token.as_ref());
        let zone_id = req.zone_id.as_ref().or(existing.zone_id.as_ref());
        let update_interval_sec = req
            .update_interval_sec
            .unwrap_or(existing.update_interval_sec);
        let status = req.status.unwrap_or(existing.status);

        let result = sqlx::query(
            r#"
            UPDATE ddns_configs
            SET hostname = ?, username = ?, password = ?, api_token = ?,
                zone_id = ?, update_interval_sec = ?, status = ?
            WHERE id = ?
            "#,
        )
        .bind(hostname)
        .bind(username)
        .bind(password)
        .bind(api_token)
        .bind(zone_id)
        .bind(update_interval_sec)
        .bind(status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a DDNS configuration
    pub async fn delete_ddns(&self, id: i32) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM ddns_configs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update DDNS last IP and status
    pub async fn update_ddns_ip(
        &self,
        id: i32,
        ip: &str,
        status: DdnsStatus,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE ddns_configs
            SET last_ip = ?, last_update = ?, status = ?, last_error = ?
            WHERE id = ?
            "#,
        )
        .bind(ip)
        .bind(Utc::now())
        .bind(status.to_string())
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set DDNS error status
    pub async fn set_ddns_error(&self, id: i32, error: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE ddns_configs
            SET status = 'error', last_error = ?
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get count of active DDNS configs
    pub async fn count_active_ddns(&self) -> Result<u32, AppError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM ddns_configs WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<i64, _>("count") as u32)
    }
}

impl std::fmt::Display for DdnsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DdnsStatus::Active => write!(f, "active"),
            DdnsStatus::Error => write!(f, "error"),
            DdnsStatus::Disabled => write!(f, "disabled"),
        }
    }
}

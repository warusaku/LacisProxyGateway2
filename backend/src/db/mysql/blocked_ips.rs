//! Blocked IPs CRUD operations

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::error::AppError;
use crate::models::{BlockIpRequest, BlockedIp};

use super::MySqlDb;

impl MySqlDb {
    /// Get all blocked IPs (including expired for history)
    pub async fn list_blocked_ips(&self) -> Result<Vec<BlockedIp>, AppError> {
        let ips = sqlx::query_as::<_, BlockedIp>(
            r#"
            SELECT id, ip, reason, blocked_by, expires_at, created_at
            FROM blocked_ips
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(ips)
    }

    /// Get currently active blocked IPs (not expired)
    pub async fn list_active_blocked_ips(&self) -> Result<Vec<BlockedIp>, AppError> {
        let ips = sqlx::query_as::<_, BlockedIp>(
            r#"
            SELECT id, ip, reason, blocked_by, expires_at, created_at
            FROM blocked_ips
            WHERE expires_at IS NULL OR expires_at > NOW()
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(ips)
    }

    /// Check if an IP is blocked
    pub async fn is_ip_blocked(&self, ip: &str) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM blocked_ips
            WHERE ip = ? AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(ip)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<i64, _>("count") > 0)
    }

    /// Get a single blocked IP by ID
    pub async fn get_blocked_ip(&self, id: i32) -> Result<Option<BlockedIp>, AppError> {
        let ip = sqlx::query_as::<_, BlockedIp>(
            r#"
            SELECT id, ip, reason, blocked_by, expires_at, created_at
            FROM blocked_ips
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(ip)
    }

    /// Block an IP address
    pub async fn block_ip(&self, req: &BlockIpRequest, blocked_by: &str) -> Result<i32, AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO blocked_ips (ip, reason, blocked_by, expires_at)
            VALUES (?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE reason = VALUES(reason), blocked_by = VALUES(blocked_by), expires_at = VALUES(expires_at)
            "#,
        )
        .bind(&req.ip)
        .bind(&req.reason)
        .bind(blocked_by)
        .bind(&req.expires_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    /// Block an IP with auto-detection source
    pub async fn auto_block_ip(
        &self,
        ip: &str,
        reason: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<i32, AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO blocked_ips (ip, reason, blocked_by, expires_at)
            VALUES (?, ?, 'auto', ?)
            ON DUPLICATE KEY UPDATE reason = VALUES(reason), blocked_by = VALUES(blocked_by), expires_at = VALUES(expires_at)
            "#,
        )
        .bind(ip)
        .bind(reason)
        .bind(&expires_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    /// Unblock an IP (delete from blocked list)
    pub async fn unblock_ip(&self, id: i32) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM blocked_ips WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Unblock an IP by address
    pub async fn unblock_ip_by_address(&self, ip: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM blocked_ips WHERE ip = ?")
            .bind(ip)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get count of blocked IPs (active only)
    pub async fn count_blocked_ips(&self) -> Result<u32, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM blocked_ips WHERE expires_at IS NULL OR expires_at > NOW()",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get::<i64, _>("count") as u32)
    }

    /// Clean up expired blocks
    pub async fn cleanup_expired_blocks(&self) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM blocked_ips WHERE expires_at IS NOT NULL AND expires_at <= NOW()",
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

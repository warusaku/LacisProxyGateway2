//! Audit log database operations

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::error::AppError;
use crate::models::AuditLog;

use super::MySqlDb;

impl MySqlDb {
    /// Log a configuration change
    pub async fn log_audit(
        &self,
        entity_type: &str,
        entity_id: Option<i32>,
        action: &str,
        field_name: Option<&str>,
        old_value: Option<&str>,
        new_value: Option<&str>,
        changed_by: &str,
        ip_address: Option<&str>,
    ) -> Result<i32, AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO config_audit_log
            (entity_type, entity_id, action, field_name, old_value, new_value, changed_by, ip_address)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(action)
        .bind(field_name)
        .bind(old_value)
        .bind(new_value)
        .bind(changed_by)
        .bind(ip_address)
        .execute(self.pool())
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(result.last_insert_id() as i32)
    }

    /// Get recent audit logs
    pub async fn get_audit_logs(&self, limit: i64, offset: i64) -> Result<Vec<AuditLog>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                entity_type,
                entity_id,
                action,
                field_name,
                old_value,
                new_value,
                changed_by,
                ip_address,
                created_at
            FROM config_audit_log
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await
        .map_err(AppError::DatabaseError)?;

        let logs = rows
            .iter()
            .map(|row| AuditLog {
                id: row.get("id"),
                entity_type: row.get("entity_type"),
                entity_id: row.get("entity_id"),
                action: row.get("action"),
                field_name: row.get("field_name"),
                old_value: row.get("old_value"),
                new_value: row.get("new_value"),
                changed_by: row.get("changed_by"),
                ip_address: row.get("ip_address"),
                created_at: row.get::<Option<DateTime<Utc>>, _>("created_at"),
            })
            .collect();

        Ok(logs)
    }

    /// Get audit logs for a specific entity
    pub async fn get_audit_logs_by_entity(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<AuditLog>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                entity_type,
                entity_id,
                action,
                field_name,
                old_value,
                new_value,
                changed_by,
                ip_address,
                created_at
            FROM config_audit_log
            WHERE entity_type = ? AND entity_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(self.pool())
        .await
        .map_err(AppError::DatabaseError)?;

        let logs = rows
            .iter()
            .map(|row| AuditLog {
                id: row.get("id"),
                entity_type: row.get("entity_type"),
                entity_id: row.get("entity_id"),
                action: row.get("action"),
                field_name: row.get("field_name"),
                old_value: row.get("old_value"),
                new_value: row.get("new_value"),
                changed_by: row.get("changed_by"),
                ip_address: row.get("ip_address"),
                created_at: row.get::<Option<DateTime<Utc>>, _>("created_at"),
            })
            .collect();

        Ok(logs)
    }
}

//! MySQL device_state_history table CRUD
//!
//! Records state transitions (online/offline/StaticOnline/StaticOffline)
//! for devices in user_object_detail. Writes happen only when the ingester
//! detects a state_type change.

use super::MySqlDb;

impl MySqlDb {
    /// Ensure device_state_history table exists (auto-migration on startup)
    pub async fn ensure_device_state_history_table(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS device_state_history (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                device_id VARCHAR(20) NOT NULL,
                state_type VARCHAR(20) NOT NULL,
                previous_state VARCHAR(20),
                changed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                source VARCHAR(20) NOT NULL DEFAULT 'syncer',
                INDEX idx_device_time (device_id, changed_at)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
            "#,
        )
        .execute(self.pool())
        .await
        .map_err(|e| format!("Failed to create device_state_history table: {}", e))?;

        Ok(())
    }

    /// Insert a state change record
    pub async fn insert_device_state_change(
        &self,
        device_id: &str,
        state_type: &str,
        previous_state: Option<&str>,
        source: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            INSERT INTO device_state_history (device_id, state_type, previous_state, source)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(device_id)
        .bind(state_type)
        .bind(previous_state)
        .bind(source)
        .execute(self.pool())
        .await
        .map_err(|e| format!("Failed to insert device state change: {}", e))?;

        Ok(())
    }
}

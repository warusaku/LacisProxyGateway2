//! Proxy routes CRUD operations

use sqlx::Row;

use crate::error::AppError;
use crate::models::{CreateRouteRequest, ProxyRoute, ProxyRouteWithDdns, UpdateRouteRequest};

use super::MySqlDb;

impl MySqlDb {
    /// Get all proxy routes ordered by priority
    pub async fn list_routes(&self) -> Result<Vec<ProxyRoute>, AppError> {
        let routes = sqlx::query_as::<_, ProxyRoute>(
            r#"
            SELECT id, path, target, ddns_config_id, priority, active, strip_prefix, preserve_host,
                   timeout_ms, created_at, updated_at
            FROM proxy_routes
            ORDER BY priority ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(routes)
    }

    /// Get active proxy routes ordered by priority
    pub async fn list_active_routes(&self) -> Result<Vec<ProxyRoute>, AppError> {
        let routes = sqlx::query_as::<_, ProxyRoute>(
            r#"
            SELECT id, path, target, ddns_config_id, priority, active, strip_prefix, preserve_host,
                   timeout_ms, created_at, updated_at
            FROM proxy_routes
            WHERE active = TRUE
            ORDER BY priority ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(routes)
    }

    /// Get active proxy routes with DDNS hostname for routing decisions
    pub async fn list_active_routes_with_ddns(&self) -> Result<Vec<ProxyRouteWithDdns>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT r.id, r.path, r.target, r.ddns_config_id, r.priority, r.active,
                   r.strip_prefix, r.preserve_host, r.timeout_ms, r.created_at, r.updated_at,
                   d.hostname as ddns_hostname
            FROM proxy_routes r
            LEFT JOIN ddns_configs d ON r.ddns_config_id = d.id
            WHERE r.active = TRUE
            ORDER BY r.priority ASC, r.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let routes = rows
            .into_iter()
            .map(|row| {
                let route = ProxyRoute {
                    id: row.get("id"),
                    path: row.get("path"),
                    target: row.get("target"),
                    ddns_config_id: row.get("ddns_config_id"),
                    priority: row.get("priority"),
                    active: row.get("active"),
                    strip_prefix: row.get("strip_prefix"),
                    preserve_host: row.get("preserve_host"),
                    timeout_ms: row.get("timeout_ms"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                ProxyRouteWithDdns {
                    route,
                    ddns_hostname: row.get("ddns_hostname"),
                }
            })
            .collect();

        Ok(routes)
    }

    /// Get a single route by ID
    pub async fn get_route(&self, id: i32) -> Result<Option<ProxyRoute>, AppError> {
        let route = sqlx::query_as::<_, ProxyRoute>(
            r#"
            SELECT id, path, target, ddns_config_id, priority, active, strip_prefix, preserve_host,
                   timeout_ms, created_at, updated_at
            FROM proxy_routes
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(route)
    }

    /// Create a new proxy route
    pub async fn create_route(&self, req: &CreateRouteRequest) -> Result<i32, AppError> {
        let result = sqlx::query(
            r#"
            INSERT INTO proxy_routes (path, target, ddns_config_id, priority, active, strip_prefix, preserve_host, timeout_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&req.path)
        .bind(&req.target)
        .bind(req.ddns_config_id)
        .bind(req.priority)
        .bind(req.active)
        .bind(req.strip_prefix)
        .bind(req.preserve_host)
        .bind(req.timeout_ms)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    /// Update an existing route
    pub async fn update_route(&self, id: i32, req: &UpdateRouteRequest) -> Result<bool, AppError> {
        let existing = self
            .get_route(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Route {} not found", id)))?;

        let path = req.path.as_ref().unwrap_or(&existing.path);
        let target = req.target.as_ref().unwrap_or(&existing.target);
        let ddns_config_id = match &req.ddns_config_id {
            Some(v) => *v,
            None => existing.ddns_config_id,
        };
        let priority = req.priority.unwrap_or(existing.priority);
        let active = req.active.unwrap_or(existing.active);
        let strip_prefix = req.strip_prefix.unwrap_or(existing.strip_prefix);
        let preserve_host = req.preserve_host.unwrap_or(existing.preserve_host);
        let timeout_ms = req.timeout_ms.unwrap_or(existing.timeout_ms);

        let result = sqlx::query(
            r#"
            UPDATE proxy_routes
            SET path = ?, target = ?, ddns_config_id = ?, priority = ?, active = ?,
                strip_prefix = ?, preserve_host = ?, timeout_ms = ?
            WHERE id = ?
            "#,
        )
        .bind(path)
        .bind(target)
        .bind(ddns_config_id)
        .bind(priority)
        .bind(active)
        .bind(strip_prefix)
        .bind(preserve_host)
        .bind(timeout_ms)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a route
    pub async fn delete_route(&self, id: i32) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM proxy_routes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get count of active routes
    pub async fn count_active_routes(&self) -> Result<u32, AppError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM proxy_routes WHERE active = TRUE")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<i64, _>("count") as u32)
    }
}

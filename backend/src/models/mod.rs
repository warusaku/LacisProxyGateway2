//! Data models for LacisProxyGateway2

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Proxy Route Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProxyRoute {
    pub id: i32,
    pub path: String,
    pub target: String,
    pub ddns_config_id: Option<i32>,
    pub priority: i32,
    pub active: bool,
    pub strip_prefix: bool,
    pub preserve_host: bool,
    pub timeout_ms: i32,
    pub websocket_support: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Extended route with DDNS hostname for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRouteWithDdns {
    #[serde(flatten)]
    pub route: ProxyRoute,
    pub ddns_hostname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRouteRequest {
    pub path: String,
    pub target: String,
    pub ddns_config_id: Option<i32>,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default = "default_true")]
    pub strip_prefix: bool,
    #[serde(default)]
    pub preserve_host: bool,
    #[serde(default = "default_timeout")]
    pub timeout_ms: i32,
    #[serde(default)]
    pub websocket_support: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRouteRequest {
    pub path: Option<String>,
    pub target: Option<String>,
    pub ddns_config_id: Option<Option<i32>>,
    pub priority: Option<i32>,
    pub active: Option<bool>,
    pub strip_prefix: Option<bool>,
    pub preserve_host: Option<bool>,
    pub timeout_ms: Option<i32>,
    pub websocket_support: Option<bool>,
}

fn default_priority() -> i32 {
    100
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> i32 {
    30000
}

// ============================================================================
// DDNS Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DdnsProvider {
    #[serde(rename = "dyndns")]
    DynDns,
    #[serde(rename = "noip")]
    NoIp,
    #[serde(rename = "cloudflare")]
    Cloudflare,
}

impl std::fmt::Display for DdnsProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DdnsProvider::DynDns => write!(f, "dyndns"),
            DdnsProvider::NoIp => write!(f, "noip"),
            DdnsProvider::Cloudflare => write!(f, "cloudflare"),
        }
    }
}

impl std::str::FromStr for DdnsProvider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dyndns" => Ok(DdnsProvider::DynDns),
            "noip" => Ok(DdnsProvider::NoIp),
            "cloudflare" => Ok(DdnsProvider::Cloudflare),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DdnsStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "disabled")]
    Disabled,
}

impl std::str::FromStr for DdnsStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(DdnsStatus::Active),
            "error" => Ok(DdnsStatus::Error),
            "disabled" => Ok(DdnsStatus::Disabled),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

/// Raw DDNS config from database (uses String for enum fields)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DdnsConfigRow {
    pub id: i32,
    pub provider: String,
    pub hostname: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_token: Option<String>,
    pub zone_id: Option<String>,
    pub update_interval_sec: i32,
    pub last_ip: Option<String>,
    pub last_update: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdnsConfig {
    pub id: i32,
    pub provider: DdnsProvider,
    pub hostname: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_token: Option<String>,
    pub zone_id: Option<String>,
    pub update_interval_sec: i32,
    pub last_ip: Option<String>,
    pub last_update: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub status: DdnsStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<DdnsConfigRow> for DdnsConfig {
    type Error = String;

    fn try_from(row: DdnsConfigRow) -> Result<Self, Self::Error> {
        Ok(DdnsConfig {
            id: row.id,
            provider: row.provider.parse()?,
            hostname: row.hostname,
            username: row.username,
            password: row.password,
            api_token: row.api_token,
            zone_id: row.zone_id,
            update_interval_sec: row.update_interval_sec,
            last_ip: row.last_ip,
            last_update: row.last_update,
            last_error: row.last_error,
            status: row.status.parse()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDdnsRequest {
    pub provider: DdnsProvider,
    pub hostname: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_token: Option<String>,
    pub zone_id: Option<String>,
    #[serde(default = "default_update_interval")]
    pub update_interval_sec: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDdnsRequest {
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_token: Option<String>,
    pub zone_id: Option<String>,
    pub update_interval_sec: Option<i32>,
    pub status: Option<DdnsStatus>,
}

fn default_update_interval() -> i32 {
    300
}

// ============================================================================
// Blocked IP Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BlockedIp {
    pub id: i32,
    pub ip: String,
    pub reason: Option<String>,
    pub blocked_by: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BlockIpRequest {
    pub ip: String,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Settings Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub id: i32,
    pub setting_key: String,
    pub setting_value: Option<String>,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: Option<String>,
}

// ============================================================================
// Access Log Models (MongoDB)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLog {
    pub timestamp: DateTime<Utc>,
    pub ip: String,
    pub method: String,
    pub path: String,
    pub route_id: Option<i32>,
    pub target: Option<String>,
    pub status: i32,
    pub response_time_ms: i32,
    pub request_size: Option<i32>,
    pub response_size: Option<i32>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
}

// ============================================================================
// Security Event Models (MongoDB)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    IpBlocked,
    RateLimitExceeded,
    SuspiciousActivity,
    DdnsFailure,
    HealthCheckFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub ip: Option<String>,
    pub details: serde_json::Value,
    pub severity: Severity,
    pub notified: bool,
}

// ============================================================================
// Health Check Models (MongoDB)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub timestamp: DateTime<Utc>,
    pub route_id: i32,
    pub target: String,
    pub healthy: bool,
    pub response_time_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error: Option<String>,
}

// ============================================================================
// Dashboard Models
// ============================================================================

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_requests_today: u64,
    pub active_routes: u32,
    pub active_ddns: u32,
    pub blocked_ips: u32,
    pub server_health: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct RouteHealth {
    pub route_id: i32,
    pub path: String,
    pub target: String,
    pub healthy: bool,
    pub last_check: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
}

/// Route statistics for detailed status
#[derive(Debug, Default, Serialize)]
pub struct RouteStats {
    pub requests_today: u64,
    pub requests_last_hour: u64,
    pub error_rate_percent: f64,
    pub avg_response_time_ms: f64,
}

// ============================================================================
// Audit Log Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: i32,
    pub entity_type: String,
    pub entity_id: Option<i32>,
    pub action: String,
    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub ip_address: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

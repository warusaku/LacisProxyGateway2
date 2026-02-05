//! Nginx configuration management handlers

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::fs;

use crate::error::AppError;
use crate::proxy::ProxyState;

use super::SuccessResponse;

/// Default nginx config path
const NGINX_SITES_AVAILABLE: &str = "/etc/nginx/sites-available";
const NGINX_SITES_ENABLED: &str = "/etc/nginx/sites-enabled";

/// Nginx status response
#[derive(Serialize)]
pub struct NginxStatus {
    pub running: bool,
    pub config_valid: bool,
    pub proxy_mode: String, // "selective" or "full_proxy"
    pub config_path: Option<String>,
    pub last_reload: Option<String>,
    pub error: Option<String>,
}

/// Nginx config update request
#[derive(Deserialize)]
pub struct UpdateNginxConfigRequest {
    pub enable_full_proxy: bool,
    pub backend_port: Option<u16>,
    pub server_name: Option<String>,
}

/// GET /api/nginx/status - Get nginx status
pub async fn get_nginx_status(
    State(_state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let running = check_nginx_running().await;
    let (config_valid, error) = test_nginx_config().await;
    let proxy_mode = detect_proxy_mode().await;
    let config_path = find_config_path().await;

    Ok(Json(NginxStatus {
        running,
        config_valid,
        proxy_mode,
        config_path,
        last_reload: None,
        error,
    }))
}

/// POST /api/nginx/enable-full-proxy - Enable full proxy mode
pub async fn enable_full_proxy(
    State(state): State<ProxyState>,
    Json(payload): Json<UpdateNginxConfigRequest>,
) -> Result<impl IntoResponse, AppError> {
    let backend_port = payload.backend_port.unwrap_or(8080);
    let server_name = payload.server_name.unwrap_or_else(|| "_".to_string());

    // Find existing config or create new
    let config_path = find_config_path().await
        .unwrap_or_else(|| format!("{}/lacis-proxy", NGINX_SITES_AVAILABLE));

    // Generate full proxy config
    let config = generate_full_proxy_config(&server_name, backend_port);

    // Backup existing config
    if let Ok(existing) = fs::read_to_string(&config_path).await {
        let backup_path = format!("{}.backup.{}", config_path, chrono::Utc::now().timestamp());
        let _ = fs::write(&backup_path, &existing).await;
        tracing::info!("Backed up existing config to {}", backup_path);
    }

    // Write new config
    fs::write(&config_path, &config).await
        .map_err(|e| AppError::InternalError(format!("Failed to write nginx config: {}", e)))?;

    // Ensure symlink in sites-enabled
    let enabled_path = format!("{}/lacis-proxy", NGINX_SITES_ENABLED);
    if !std::path::Path::new(&enabled_path).exists() {
        let _ = std::os::unix::fs::symlink(&config_path, &enabled_path);
    }

    // Test config
    let (valid, error) = test_nginx_config().await;
    if !valid {
        return Err(AppError::BadRequest(format!(
            "Nginx config test failed: {}",
            error.unwrap_or_default()
        )));
    }

    // Reload nginx
    reload_nginx().await?;

    // Send notification
    state.notifier.notify_config_change(
        "Nginx Configuration Updated",
        "Full proxy mode enabled. All routes are now managed through the UI.",
    ).await;

    tracing::info!("Enabled full proxy mode in nginx");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse::new("Full proxy mode enabled. Nginx reloaded.")),
    ))
}

/// POST /api/nginx/reload - Reload nginx
pub async fn reload_nginx_handler(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    // Test config first
    let (valid, error) = test_nginx_config().await;
    if !valid {
        return Err(AppError::BadRequest(format!(
            "Nginx config test failed: {}",
            error.unwrap_or_default()
        )));
    }

    reload_nginx().await?;

    state.notifier.notify_config_change(
        "Nginx Reloaded",
        "Nginx configuration reloaded successfully.",
    ).await;

    Ok(Json(SuccessResponse::new("Nginx reloaded successfully")))
}

/// POST /api/nginx/test - Test nginx config
pub async fn test_nginx_config_handler(
    State(_state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let (valid, error) = test_nginx_config().await;

    #[derive(Serialize)]
    struct TestResult {
        valid: bool,
        error: Option<String>,
    }

    Ok(Json(TestResult { valid, error }))
}

/// GET /api/nginx/config - Get current nginx config content
pub async fn get_nginx_config(
    State(_state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let config_path = find_config_path().await
        .ok_or_else(|| AppError::NotFound("Nginx config not found".to_string()))?;

    let content = fs::read_to_string(&config_path).await
        .map_err(|e| AppError::InternalError(format!("Failed to read config: {}", e)))?;

    #[derive(Serialize)]
    struct ConfigResponse {
        path: String,
        content: String,
    }

    Ok(Json(ConfigResponse {
        path: config_path,
        content,
    }))
}

// ============================================================================
// Helper functions
// ============================================================================

async fn check_nginx_running() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "nginx"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn test_nginx_config() -> (bool, Option<String>) {
    match Command::new("sudo")
        .args(["nginx", "-t"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                (true, None)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                (false, Some(stderr))
            }
        }
        Err(e) => (false, Some(format!("Failed to run nginx -t: {}", e))),
    }
}

async fn reload_nginx() -> Result<(), AppError> {
    let output = Command::new("sudo")
        .args(["systemctl", "reload", "nginx"])
        .output()
        .map_err(|e| AppError::InternalError(format!("Failed to reload nginx: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::InternalError(format!("Nginx reload failed: {}", stderr)));
    }

    Ok(())
}

async fn detect_proxy_mode() -> String {
    if let Some(config_path) = find_config_path().await {
        if let Ok(content) = fs::read_to_string(&config_path).await {
            // Check if config has "location /" with proxy_pass
            if content.contains("location / {") && content.contains("proxy_pass") {
                return "full_proxy".to_string();
            }
        }
    }
    "selective".to_string()
}

async fn find_config_path() -> Option<String> {
    // Check common locations
    let candidates = [
        format!("{}/lacis-proxy", NGINX_SITES_AVAILABLE),
        format!("{}/lacis-proxy-gateway", NGINX_SITES_AVAILABLE),
        format!("{}/default", NGINX_SITES_AVAILABLE),
        "/etc/nginx/conf.d/lacis-proxy.conf".to_string(),
    ];

    for path in candidates {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

fn generate_full_proxy_config(server_name: &str, backend_port: u16) -> String {
    format!(r#"# LacisProxyGateway2 - Full Proxy Mode
# Generated automatically - DO NOT EDIT MANUALLY
# All routing is managed through the LacisProxyGateway2 UI

server {{
    listen 80;
    server_name {server_name};

    # Redirect HTTP to HTTPS
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl http2;
    server_name {server_name};

    # SSL Configuration (managed by certbot)
    ssl_certificate /etc/letsencrypt/live/{server_name}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{server_name}/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Proxy all requests to LacisProxyGateway2 backend
    location / {{
        proxy_pass http://127.0.0.1:{backend_port};
        proxy_http_version 1.1;
        
        # Headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Buffering
        proxy_buffering off;
        proxy_request_buffering off;
    }}

    # Let's Encrypt challenge
    location /.well-known/acme-challenge/ {{
        root /var/www/certbot;
    }}
}}
"#)
}

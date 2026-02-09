//! Nginx configuration management handlers

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tokio::fs;

use crate::api::auth_middleware::require_permission;
use crate::db::mysql::MySqlDb;
use crate::error::AppError;
use crate::models::AuthUser;
use crate::proxy::ProxyState;

use super::SuccessResponse;

// ============================================================================
// Nginx Template Settings (DB-driven config)
// ============================================================================

/// All nginx template settings stored in DB
#[derive(Debug, Serialize, Clone)]
pub struct NginxTemplateSettings {
    pub server_name: String,
    pub backend_port: u16,
    pub gzip_enabled: bool,
    pub gzip_comp_level: u32,
    pub gzip_min_length: u32,
    pub proxy_connect_timeout: u32,
    pub proxy_send_timeout: u32,
    pub proxy_read_timeout: u32,
    pub header_x_frame_options: String,
    pub header_x_content_type: String,
    pub header_xss_protection: String,
    pub header_hsts: String,
    pub header_referrer_policy: String,
    pub header_permissions_policy: String,
    pub header_csp: String,
}

/// Partial update request - only Some fields are updated
#[derive(Debug, Deserialize)]
pub struct UpdateNginxTemplateSettingsRequest {
    pub server_name: Option<String>,
    pub backend_port: Option<u16>,
    pub gzip_enabled: Option<bool>,
    pub gzip_comp_level: Option<u32>,
    pub gzip_min_length: Option<u32>,
    pub proxy_connect_timeout: Option<u32>,
    pub proxy_send_timeout: Option<u32>,
    pub proxy_read_timeout: Option<u32>,
    pub header_x_frame_options: Option<String>,
    pub header_x_content_type: Option<String>,
    pub header_xss_protection: Option<String>,
    pub header_hsts: Option<String>,
    pub header_referrer_policy: Option<String>,
    pub header_permissions_policy: Option<String>,
    pub header_csp: Option<String>,
}

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
    pub client_max_body_size: Option<String>,
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
    let client_max_body_size = get_client_max_body_size().await;

    Ok(Json(NginxStatus {
        running,
        config_valid,
        proxy_mode,
        config_path,
        last_reload: None,
        error,
        client_max_body_size,
    }))
}

/// POST /api/nginx/enable-full-proxy - Enable full proxy mode (admin: permission >= 80)
pub async fn enable_full_proxy(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<UpdateNginxConfigRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let backend_port = payload.backend_port.unwrap_or(8080);
    let server_name = payload.server_name.unwrap_or_else(|| "_".to_string());

    // Save server_name and backend_port to DB for template settings
    let db = &state.app_state.mysql;
    db.set_setting("nginx_server_name", Some(&server_name))
        .await?;
    db.set_setting("nginx_backend_port", Some(&backend_port.to_string()))
        .await?;

    // Load full template settings from DB (now includes updated server_name/port)
    let settings = load_template_settings_from_db(db).await?;

    // Find existing config or create new
    let config_path = find_config_path()
        .await
        .unwrap_or_else(|| format!("{}/lacis-proxy", NGINX_SITES_AVAILABLE));

    // Generate full proxy config from DB settings
    let config = generate_full_proxy_config_from_settings(&settings);

    // Backup existing config
    if let Ok(existing) = fs::read_to_string(&config_path).await {
        let backup_path = format!("{}.backup.{}", config_path, chrono::Utc::now().timestamp());
        let _ = fs::write(&backup_path, &existing).await;
        tracing::info!("Backed up existing config to {}", backup_path);
    }

    // Write new config
    fs::write(&config_path, &config)
        .await
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
    state
        .notifier
        .notify_config_change(
            "Nginx Configuration Updated",
            "Full proxy mode enabled. All routes are now managed through the UI.",
        )
        .await;

    tracing::info!("Enabled full proxy mode in nginx");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse::new(
            "Full proxy mode enabled. Nginx reloaded.",
        )),
    ))
}

/// POST /api/nginx/reload - Reload nginx
pub async fn reload_nginx_handler(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Test config first
    let (valid, error) = test_nginx_config().await;
    if !valid {
        return Err(AppError::BadRequest(format!(
            "Nginx config test failed: {}",
            error.unwrap_or_default()
        )));
    }

    reload_nginx().await?;

    state
        .notifier
        .notify_config_change(
            "Nginx Reloaded",
            "Nginx configuration reloaded successfully.",
        )
        .await;

    Ok(Json(SuccessResponse::new("Nginx reloaded successfully")))
}

/// POST /api/nginx/test - Test nginx config
pub async fn test_nginx_config_handler(
    State(_state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

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
    let config_path = find_config_path()
        .await
        .ok_or_else(|| AppError::NotFound("Nginx config not found".to_string()))?;

    let content = fs::read_to_string(&config_path)
        .await
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

pub(crate) async fn check_nginx_running() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "nginx"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) async fn test_nginx_config() -> (bool, Option<String>) {
    match Command::new("sudo").args(["nginx", "-t"]).output() {
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
        return Err(AppError::InternalError(format!(
            "Nginx reload failed: {}",
            stderr
        )));
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
        format!("{}/eatyui", NGINX_SITES_AVAILABLE),
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

async fn get_client_max_body_size() -> Option<String> {
    if let Some(config_path) = find_config_path().await {
        if let Ok(content) = fs::read_to_string(&config_path).await {
            // Parse client_max_body_size from config
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("client_max_body_size") {
                    // Extract value like "50M" from "client_max_body_size 50M;"
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let value = parts[1].trim_end_matches(';');
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Request to update client_max_body_size
#[derive(Deserialize)]
pub struct UpdateBodySizeRequest {
    pub size: String, // e.g., "50M", "100M", "1G"
}

/// PUT /api/nginx/body-size - Update client_max_body_size
pub async fn update_body_size(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<UpdateBodySizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Validate size format (e.g., 50M, 100M, 1G)
    let size = payload.size.trim().to_uppercase();
    if !size
        .chars()
        .last()
        .map(|c| c == 'M' || c == 'G' || c == 'K')
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Size must end with K, M, or G (e.g., 50M, 1G)".to_string(),
        ));
    }
    let numeric_part = &size[..size.len() - 1];
    if numeric_part.parse::<u32>().is_err() {
        return Err(AppError::BadRequest("Invalid size format".to_string()));
    }

    let config_path = find_config_path()
        .await
        .ok_or_else(|| AppError::NotFound("Nginx config not found".to_string()))?;

    let content = fs::read_to_string(&config_path)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to read config: {}", e)))?;

    // Update or add client_max_body_size in each location block
    let mut new_content = String::new();
    let mut in_location_block = false;
    let mut brace_count = 0;
    let mut has_body_size = false;
    let mut pending_body_size_insert = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Track if we're entering a location block
        if trimmed.starts_with("location ") {
            in_location_block = true;
            has_body_size = false;
            pending_body_size_insert = true;
        }

        // Track braces
        if in_location_block {
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;

            if brace_count == 0 {
                in_location_block = false;
            }
        }

        // Replace existing client_max_body_size
        if trimmed.starts_with("client_max_body_size") {
            new_content.push_str(&format!("        client_max_body_size {};\n", size));
            has_body_size = true;
            pending_body_size_insert = false;
            continue;
        }

        // Add client_max_body_size after location line with opening brace
        if pending_body_size_insert && trimmed.contains('{') {
            new_content.push_str(line);
            new_content.push('\n');
            new_content.push_str(&format!("        client_max_body_size {};\n", size));
            pending_body_size_insert = false;
            continue;
        }

        new_content.push_str(line);
        new_content.push('\n');
    }

    // Write updated config using sudo
    let output = Command::new("sudo")
        .args(["tee", &config_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(new_content.as_bytes())?;
            }
            child.wait_with_output()
        })
        .map_err(|e| AppError::InternalError(format!("Failed to write config: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::InternalError(
            "Failed to write nginx config".to_string(),
        ));
    }

    // Test and reload nginx
    let (valid, error) = test_nginx_config().await;
    if !valid {
        return Err(AppError::BadRequest(format!(
            "Nginx config test failed: {}",
            error.unwrap_or_default()
        )));
    }

    reload_nginx().await?;

    state
        .notifier
        .notify_config_change(
            "Nginx Body Size Updated",
            &format!("client_max_body_size changed to {}", size),
        )
        .await;

    tracing::info!("Updated client_max_body_size to {}", size);

    Ok(Json(SuccessResponse::new(&format!(
        "Body size limit updated to {}",
        size
    ))))
}

// ============================================================================
// Template Settings Handlers
// ============================================================================

/// GET /api/nginx/template-settings - Get all nginx template settings from DB
pub async fn get_nginx_template_settings(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.app_state.mysql;
    let settings = load_template_settings_from_db(db).await?;
    Ok(Json(settings))
}

/// PUT /api/nginx/template-settings - Update nginx template settings (DB only, no nginx reload)
pub async fn update_nginx_template_settings(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<UpdateNginxTemplateSettingsRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    // Validation
    if let Some(port) = payload.backend_port {
        if port == 0 {
            return Err(AppError::BadRequest(
                "backend_port must be 1-65535".to_string(),
            ));
        }
    }
    if let Some(level) = payload.gzip_comp_level {
        if !(1..=9).contains(&level) {
            return Err(AppError::BadRequest(
                "gzip_comp_level must be 1-9".to_string(),
            ));
        }
    }
    if let Some(t) = payload.proxy_connect_timeout {
        if t == 0 || t > 3600 {
            return Err(AppError::BadRequest(
                "proxy_connect_timeout must be 1-3600".to_string(),
            ));
        }
    }
    if let Some(t) = payload.proxy_send_timeout {
        if t == 0 || t > 3600 {
            return Err(AppError::BadRequest(
                "proxy_send_timeout must be 1-3600".to_string(),
            ));
        }
    }
    if let Some(t) = payload.proxy_read_timeout {
        if t == 0 || t > 3600 {
            return Err(AppError::BadRequest(
                "proxy_read_timeout must be 1-3600".to_string(),
            ));
        }
    }

    let db = &state.app_state.mysql;

    // Update only provided fields
    if let Some(v) = &payload.server_name {
        db.set_setting("nginx_server_name", Some(v)).await?;
    }
    if let Some(v) = payload.backend_port {
        db.set_setting("nginx_backend_port", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = payload.gzip_enabled {
        db.set_setting("nginx_gzip_enabled", Some(if v { "true" } else { "false" }))
            .await?;
    }
    if let Some(v) = payload.gzip_comp_level {
        db.set_setting("nginx_gzip_comp_level", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = payload.gzip_min_length {
        db.set_setting("nginx_gzip_min_length", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = payload.proxy_connect_timeout {
        db.set_setting("nginx_proxy_connect_timeout", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = payload.proxy_send_timeout {
        db.set_setting("nginx_proxy_send_timeout", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = payload.proxy_read_timeout {
        db.set_setting("nginx_proxy_read_timeout", Some(&v.to_string()))
            .await?;
    }
    if let Some(v) = &payload.header_x_frame_options {
        db.set_setting("nginx_header_x_frame_options", Some(v))
            .await?;
    }
    if let Some(v) = &payload.header_x_content_type {
        db.set_setting("nginx_header_x_content_type", Some(v))
            .await?;
    }
    if let Some(v) = &payload.header_xss_protection {
        db.set_setting("nginx_header_xss_protection", Some(v))
            .await?;
    }
    if let Some(v) = &payload.header_hsts {
        db.set_setting("nginx_header_hsts", Some(v)).await?;
    }
    if let Some(v) = &payload.header_referrer_policy {
        db.set_setting("nginx_header_referrer_policy", Some(v))
            .await?;
    }
    if let Some(v) = &payload.header_permissions_policy {
        db.set_setting("nginx_header_permissions_policy", Some(v))
            .await?;
    }
    if let Some(v) = &payload.header_csp {
        db.set_setting("nginx_header_csp", Some(v)).await?;
    }

    tracing::info!("Updated nginx template settings in DB");

    Ok(Json(SuccessResponse::new("Nginx template settings saved")))
}

/// POST /api/nginx/regenerate - Regenerate nginx config from DB settings and reload
pub async fn regenerate_nginx_config(
    State(state): State<ProxyState>,
    Extension(user): Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError> {
    require_permission(&user, 80)?;

    let db = &state.app_state.mysql;
    let settings = load_template_settings_from_db(db).await?;

    // Find existing config or create new
    let config_path = find_config_path()
        .await
        .unwrap_or_else(|| format!("{}/lacis-proxy", NGINX_SITES_AVAILABLE));

    // Generate config from DB settings
    let config = generate_full_proxy_config_from_settings(&settings);

    // Backup existing config
    if let Ok(existing) = fs::read_to_string(&config_path).await {
        let backup_path = format!("{}.backup.{}", config_path, chrono::Utc::now().timestamp());
        let _ = fs::write(&backup_path, &existing).await;
        tracing::info!("Backed up existing config to {}", backup_path);
    }

    // Write new config
    fs::write(&config_path, &config)
        .await
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
            "Nginx config test failed. Config was NOT applied: {}",
            error.unwrap_or_default()
        )));
    }

    // Reload nginx
    reload_nginx().await?;

    state
        .notifier
        .notify_config_change(
            "Nginx Config Regenerated",
            "Nginx configuration regenerated from template settings and reloaded.",
        )
        .await;

    tracing::info!("Regenerated nginx config from DB template settings");

    Ok(Json(SuccessResponse::new(
        "Nginx config regenerated and reloaded successfully",
    )))
}

// ============================================================================
// Template Settings Helpers
// ============================================================================

/// Load NginxTemplateSettings from DB with sensible defaults
async fn load_template_settings_from_db(db: &MySqlDb) -> Result<NginxTemplateSettings, AppError> {
    let server_name = db
        .get_setting("nginx_server_name")
        .await?
        .unwrap_or_else(|| "_".to_string());
    let backend_port = db
        .get_setting("nginx_backend_port")
        .await?
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8081);
    let gzip_enabled = db
        .get_setting_bool("nginx_gzip_enabled")
        .await
        .unwrap_or(true);
    let gzip_comp_level = db
        .get_setting_i32("nginx_gzip_comp_level", 6)
        .await
        .unwrap_or(6) as u32;
    let gzip_min_length = db
        .get_setting_i32("nginx_gzip_min_length", 1024)
        .await
        .unwrap_or(1024) as u32;
    let proxy_connect_timeout = db
        .get_setting_i32("nginx_proxy_connect_timeout", 60)
        .await
        .unwrap_or(60) as u32;
    let proxy_send_timeout = db
        .get_setting_i32("nginx_proxy_send_timeout", 60)
        .await
        .unwrap_or(60) as u32;
    let proxy_read_timeout = db
        .get_setting_i32("nginx_proxy_read_timeout", 60)
        .await
        .unwrap_or(60) as u32;

    let header_x_frame_options = db
        .get_setting("nginx_header_x_frame_options")
        .await?
        .unwrap_or_else(|| "SAMEORIGIN".to_string());
    let header_x_content_type = db
        .get_setting("nginx_header_x_content_type")
        .await?
        .unwrap_or_else(|| "nosniff".to_string());
    let header_xss_protection = db
        .get_setting("nginx_header_xss_protection")
        .await?
        .unwrap_or_else(|| "1; mode=block".to_string());
    let header_hsts = db
        .get_setting("nginx_header_hsts")
        .await?
        .unwrap_or_else(|| "max-age=31536000; includeSubDomains".to_string());
    let header_referrer_policy = db
        .get_setting("nginx_header_referrer_policy")
        .await?
        .unwrap_or_else(|| "strict-origin-when-cross-origin".to_string());
    let header_permissions_policy = db
        .get_setting("nginx_header_permissions_policy")
        .await?
        .unwrap_or_else(|| "camera=(), microphone=(), geolocation=()".to_string());
    let header_csp = db.get_setting("nginx_header_csp").await?
        .unwrap_or_else(|| "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; frame-src 'self' https://maps.google.com https://www.google.com;".to_string());

    Ok(NginxTemplateSettings {
        server_name,
        backend_port,
        gzip_enabled,
        gzip_comp_level,
        gzip_min_length,
        proxy_connect_timeout,
        proxy_send_timeout,
        proxy_read_timeout,
        header_x_frame_options,
        header_x_content_type,
        header_xss_protection,
        header_hsts,
        header_referrer_policy,
        header_permissions_policy,
        header_csp,
    })
}

/// Generate nginx config from NginxTemplateSettings
fn generate_full_proxy_config_from_settings(s: &NginxTemplateSettings) -> String {
    // Build gzip section
    let gzip_section = if s.gzip_enabled {
        format!(
            r#"    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level {};
    gzip_min_length {};
    gzip_types text/plain text/css application/json application/javascript
               text/xml application/xml application/xml+rss text/javascript
               image/svg+xml application/font-woff2;"#,
            s.gzip_comp_level, s.gzip_min_length
        )
    } else {
        "    # Gzip compression (disabled)\n    gzip off;".to_string()
    };

    // Build security headers section - only emit non-empty values
    let mut headers = Vec::new();
    if !s.header_x_frame_options.is_empty() {
        headers.push(format!(
            r#"    add_header X-Frame-Options "{}" always;"#,
            s.header_x_frame_options
        ));
    }
    if !s.header_x_content_type.is_empty() {
        headers.push(format!(
            r#"    add_header X-Content-Type-Options "{}" always;"#,
            s.header_x_content_type
        ));
    }
    if !s.header_xss_protection.is_empty() {
        headers.push(format!(
            r#"    add_header X-XSS-Protection "{}" always;"#,
            s.header_xss_protection
        ));
    }
    if !s.header_hsts.is_empty() {
        headers.push(format!(
            r#"    add_header Strict-Transport-Security "{}" always;"#,
            s.header_hsts
        ));
    }
    if !s.header_referrer_policy.is_empty() {
        headers.push(format!(
            r#"    add_header Referrer-Policy "{}" always;"#,
            s.header_referrer_policy
        ));
    }
    if !s.header_permissions_policy.is_empty() {
        headers.push(format!(
            r#"    add_header Permissions-Policy "{}" always;"#,
            s.header_permissions_policy
        ));
    }
    if !s.header_csp.is_empty() {
        headers.push(format!(
            r#"    add_header Content-Security-Policy "{}" always;"#,
            s.header_csp
        ));
    }

    let security_headers_section = if headers.is_empty() {
        "    # Security headers (all disabled)".to_string()
    } else {
        format!("    # Security headers\n{}", headers.join("\n"))
    };

    let server_name = &s.server_name;
    let backend_port = s.backend_port;
    let connect_timeout = s.proxy_connect_timeout;
    let send_timeout = s.proxy_send_timeout;
    let read_timeout = s.proxy_read_timeout;

    format!(
        r#"# LacisProxyGateway2 - Full Proxy Mode
# Generated automatically from template settings - DO NOT EDIT MANUALLY
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

    # SSL hardening (TLS 1.2+ only, strong cipher suites)
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers on;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 1d;
    ssl_session_tickets off;

{gzip_section}

{security_headers_section}

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
        proxy_connect_timeout {connect_timeout}s;
        proxy_send_timeout {send_timeout}s;
        proxy_read_timeout {read_timeout}s;

        # Buffering
        proxy_buffering off;
        proxy_request_buffering off;
    }}

    # Let's Encrypt challenge
    location /.well-known/acme-challenge/ {{
        root /var/www/certbot;
    }}
}}
"#
    )
}

/// Backward-compatible wrapper: builds NginxTemplateSettings with defaults and the given params
fn generate_full_proxy_config(server_name: &str, backend_port: u16) -> String {
    let settings = NginxTemplateSettings {
        server_name: server_name.to_string(),
        backend_port,
        gzip_enabled: true,
        gzip_comp_level: 6,
        gzip_min_length: 1024,
        proxy_connect_timeout: 60,
        proxy_send_timeout: 60,
        proxy_read_timeout: 60,
        header_x_frame_options: "SAMEORIGIN".to_string(),
        header_x_content_type: "nosniff".to_string(),
        header_xss_protection: "1; mode=block".to_string(),
        header_hsts: "max-age=31536000; includeSubDomains".to_string(),
        header_referrer_policy: "strict-origin-when-cross-origin".to_string(),
        header_permissions_policy: "camera=(), microphone=(), geolocation=()".to_string(),
        header_csp: "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; frame-src 'self' https://maps.google.com https://www.google.com;".to_string(),
    };
    generate_full_proxy_config_from_settings(&settings)
}

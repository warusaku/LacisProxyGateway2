//! Settings handlers

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::proxy::ProxyState;

use super::SuccessResponse;

/// Restart settings response
#[derive(Debug, Serialize)]
pub struct RestartSettings {
    pub scheduled_enabled: bool,
    pub scheduled_time: String,
    pub auto_restart_enabled: bool,
    pub cpu_threshold: u32,
    pub ram_threshold: u32,
}

/// Restart settings update request
#[derive(Debug, Deserialize)]
pub struct UpdateRestartSettingsRequest {
    pub scheduled_enabled: Option<bool>,
    pub scheduled_time: Option<String>,
    pub auto_restart_enabled: Option<bool>,
    pub cpu_threshold: Option<u32>,
    pub ram_threshold: Option<u32>,
}

/// GET /api/settings - List all settings
pub async fn list_settings(State(state): State<ProxyState>) -> Result<impl IntoResponse, AppError> {
    let settings = state.app_state.mysql.list_settings().await?;

    // Mask the Discord webhook URL
    let masked: Vec<_> = settings
        .into_iter()
        .map(|mut s| {
            if s.setting_key == "discord_webhook_url" && s.setting_value.is_some() {
                s.setting_value = Some("********".to_string());
            }
            s
        })
        .collect();

    Ok(Json(masked))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: Option<String>,
}

/// PUT /api/settings/:key - Update a setting
pub async fn update_setting(
    State(state): State<ProxyState>,
    Path(key): Path<String>,
    Json(payload): Json<UpdateSettingRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate setting key exists
    let existing = state.app_state.mysql.get_setting(&key).await;
    if existing.is_err() {
        return Err(AppError::NotFound(format!("Setting {} not found", key)));
    }

    let updated = state
        .app_state
        .mysql
        .set_setting(&key, payload.value.as_deref())
        .await?;

    if updated {
        tracing::info!("Updated setting: {}", key);
        Ok(Json(SuccessResponse::new("Setting updated")))
    } else {
        Err(AppError::NotFound(format!("Setting {} not found", key)))
    }
}

/// POST /api/settings/test-discord - Test Discord webhook
pub async fn test_discord_notification(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let webhook_url = state
        .app_state
        .mysql
        .get_discord_webhook_url()
        .await?
        .ok_or_else(|| AppError::BadRequest("Discord webhook URL not configured".to_string()))?;

    // Send test notification
    let client = reqwest::Client::new();
    let response = client
        .post(&webhook_url)
        .json(&serde_json::json!({
            "embeds": [{
                "title": "LacisProxyGateway2 Test",
                "description": "Discord notification is working!",
                "color": 3066993,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        }))
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to send notification: {}", e)))?;

    if response.status().is_success() {
        tracing::info!("Discord test notification sent successfully");
        Ok(Json(SuccessResponse::new("Test notification sent")))
    } else {
        Err(AppError::InternalError(format!(
            "Discord returned status: {}",
            response.status()
        )))
    }
}

/// GET /api/settings/restart - Get restart settings
pub async fn get_restart_settings(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    let settings = state.app_state.mysql.list_settings().await?;

    let mut restart_settings = RestartSettings {
        scheduled_enabled: false,
        scheduled_time: "04:00".to_string(),
        auto_restart_enabled: false,
        cpu_threshold: 90,
        ram_threshold: 90,
    };

    for setting in settings {
        match setting.setting_key.as_str() {
            "restart_scheduled_enabled" => {
                restart_settings.scheduled_enabled = setting
                    .setting_value
                    .as_deref()
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
            }
            "restart_scheduled_time" => {
                if let Some(v) = setting.setting_value {
                    restart_settings.scheduled_time = v;
                }
            }
            "restart_auto_enabled" => {
                restart_settings.auto_restart_enabled = setting
                    .setting_value
                    .as_deref()
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
            }
            "restart_cpu_threshold" => {
                if let Some(v) = setting.setting_value {
                    restart_settings.cpu_threshold = v.parse().unwrap_or(90);
                }
            }
            "restart_ram_threshold" => {
                if let Some(v) = setting.setting_value {
                    restart_settings.ram_threshold = v.parse().unwrap_or(90);
                }
            }
            _ => {}
        }
    }

    Ok(Json(restart_settings))
}

/// PUT /api/settings/restart - Update restart settings
pub async fn update_restart_settings(
    State(state): State<ProxyState>,
    Json(payload): Json<UpdateRestartSettingsRequest>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(enabled) = payload.scheduled_enabled {
        state
            .app_state
            .mysql
            .set_setting("restart_scheduled_enabled", Some(&enabled.to_string()))
            .await?;
    }

    if let Some(time) = payload.scheduled_time {
        // Validate time format (HH:MM)
        if !time.chars().all(|c| c.is_ascii_digit() || c == ':')
            || time.len() != 5
            || time.chars().nth(2) != Some(':')
        {
            return Err(AppError::BadRequest("Invalid time format. Use HH:MM".to_string()));
        }
        state
            .app_state
            .mysql
            .set_setting("restart_scheduled_time", Some(&time))
            .await?;
    }

    if let Some(enabled) = payload.auto_restart_enabled {
        state
            .app_state
            .mysql
            .set_setting("restart_auto_enabled", Some(&enabled.to_string()))
            .await?;
    }

    if let Some(threshold) = payload.cpu_threshold {
        if threshold > 100 {
            return Err(AppError::BadRequest("CPU threshold must be 0-100".to_string()));
        }
        state
            .app_state
            .mysql
            .set_setting("restart_cpu_threshold", Some(&threshold.to_string()))
            .await?;
    }

    if let Some(threshold) = payload.ram_threshold {
        if threshold > 100 {
            return Err(AppError::BadRequest("RAM threshold must be 0-100".to_string()));
        }
        state
            .app_state
            .mysql
            .set_setting("restart_ram_threshold", Some(&threshold.to_string()))
            .await?;
    }

    tracing::info!("Restart settings updated");
    Ok(Json(SuccessResponse::new("Restart settings updated")))
}

/// POST /api/settings/restart/trigger - Manually trigger restart
pub async fn trigger_manual_restart(
    State(state): State<ProxyState>,
) -> Result<impl IntoResponse, AppError> {
    tracing::warn!("Manual restart triggered via API");

    // Send Discord notification
    if let Ok(Some(webhook_url)) = state.app_state.mysql.get_discord_webhook_url().await {
        let client = reqwest::Client::new();
        let _ = client
            .post(&webhook_url)
            .json(&serde_json::json!({
                "embeds": [{
                    "title": "Manual Restart Triggered",
                    "description": "System restart initiated via management UI",
                    "color": 15105570,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "footer": {"text": "LacisProxyGateway2"}
                }]
            }))
            .send()
            .await;
    }

    // Execute restart in background
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let _ = std::process::Command::new("sudo")
            .args(["systemctl", "reboot"])
            .output();
    });

    Ok(Json(SuccessResponse::new("Restart initiated")))
}
